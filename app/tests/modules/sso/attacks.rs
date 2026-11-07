//! What an attacker can try against single sign-on, and what stops it.

use super::secrets_import::browser_call;
use super::sso_login::{
  CLIENT_ID, CLIENT_SECRET, Grant, MockIdp, SsoFixture, Started, callback, exchange, location,
  provider_body, save_provider, send, set_cookie, sso_fixture, start_link, start_login,
};
use super::*;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use p256::ecdsa::SigningKey;
use sha2::{Digest, Sha256};

fn now() -> i64 {
  chrono::Utc::now().timestamp()
}

fn claims(
  idp: &MockIdp,
  started: &Started,
) -> Value {
  json!({
    "iss": idp.url,
    "aud": CLIENT_ID,
    "sub": "attacker-subject",
    "email": "member@dopbase.com",
    "email_verified": true,
    "nonce": started.nonce,
    "iat": now(),
    "exp": now() + 300,
  })
}

fn es256() -> Value {
  json!({"alg":"ES256","typ":"JWT","kid":"k1"})
}

/// HMAC-SHA256 by hand, to forge a token "signed" with the client secret.
fn hmac_sha256(
  key: &[u8],
  message: &[u8],
) -> Vec<u8> {
  let mut block = [0_u8; 64];
  block[..key.len()].copy_from_slice(key);
  let inner: Vec<u8> = block.iter().map(|byte| byte ^ 0x36).collect();
  let outer: Vec<u8> = block.iter().map(|byte| byte ^ 0x5c).collect();
  let inner_hash = Sha256::new()
    .chain_update(&inner)
    .chain_update(message)
    .finalize();
  Sha256::new()
    .chain_update(&outer)
    .chain_update(inner_hash)
    .finalize()
    .to_vec()
}

fn unsigned(
  header: &Value,
  claims: &Value,
) -> String {
  format!(
    "{}.{}",
    URL_SAFE_NO_PAD.encode(header.to_string()),
    URL_SAFE_NO_PAD.encode(claims.to_string())
  )
}

async fn linked_subjects(fixture: &SsoFixture) -> Vec<String> {
  sqlx::query_scalar("SELECT subject FROM admin_identities ORDER BY subject")
    .fetch_all(fixture.state.db.pool())
    .await
    .unwrap()
}

#[tokio::test]
async fn forged_and_tampered_id_tokens_never_sign_anyone_in() {
  let fixture = sso_fixture().await;
  let idp = &fixture.idp;
  let foreign = SigningKey::from_slice(&[9_u8; 32]).unwrap();
  type Forge = Box<dyn Fn(&MockIdp, &Started) -> String>;
  let cases: Vec<(&str, Forge)> = vec![
    (
      "alg none",
      Box::new(|idp, started| {
        format!(
          "{}.",
          unsigned(&json!({"alg":"none","typ":"JWT"}), &claims(idp, started))
        )
      }),
    ),
    (
      "HS256 keyed with the client secret",
      Box::new(|idp, started| {
        let input = unsigned(
          &json!({"alg":"HS256","typ":"JWT","kid":"k1"}),
          &claims(idp, started),
        );
        let mac = hmac_sha256(CLIENT_SECRET.as_bytes(), input.as_bytes());
        format!("{input}.{}", URL_SAFE_NO_PAD.encode(mac))
      }),
    ),
    (
      "signed by a key the provider never published",
      Box::new(move |idp, started| MockIdp::sign(&foreign, &es256(), &claims(idp, started))),
    ),
    (
      "payload changed after signing",
      Box::new(|idp, started| {
        let signed = MockIdp::sign(idp.key(), &es256(), &claims(idp, started));
        let mut parts: Vec<String> = signed.split('.').map(str::to_owned).collect();
        let mut forged = claims(idp, started);
        forged["email"] = json!("admin@dopbase.com");
        parts[1] = URL_SAFE_NO_PAD.encode(forged.to_string());
        parts.join(".")
      }),
    ),
    (
      "garbage signature",
      Box::new(|idp, started| {
        format!(
          "{}.{}",
          unsigned(&es256(), &claims(idp, started)),
          URL_SAFE_NO_PAD.encode([1_u8; 64])
        )
      }),
    ),
    (
      "audience of another client",
      Box::new(|idp, started| {
        let mut claims = claims(idp, started);
        claims["aud"] = json!("another-client");
        MockIdp::sign(idp.key(), &es256(), &claims)
      }),
    ),
    (
      "issued to us and another client",
      Box::new(|idp, started| {
        let mut claims = claims(idp, started);
        claims["aud"] = json!([CLIENT_ID, "another-client"]);
        claims["azp"] = json!("another-client");
        MockIdp::sign(idp.key(), &es256(), &claims)
      }),
    ),
    (
      "another issuer",
      Box::new(|idp, started| {
        let mut claims = claims(idp, started);
        claims["iss"] = json!("https://evil.example.test");
        MockIdp::sign(idp.key(), &es256(), &claims)
      }),
    ),
    (
      "expired",
      Box::new(|idp, started| {
        let mut claims = claims(idp, started);
        claims["iat"] = json!(now() - 900);
        claims["exp"] = json!(now() - 600);
        MockIdp::sign(idp.key(), &es256(), &claims)
      }),
    ),
    (
      "no nonce",
      Box::new(|idp, started| {
        let mut claims = claims(idp, started);
        claims.as_object_mut().unwrap().remove("nonce");
        MockIdp::sign(idp.key(), &es256(), &claims)
      }),
    ),
  ];
  for (index, (name, forge)) in cases.iter().enumerate() {
    // Each rejection counts against this client; reset so every forgery
    // reaches token validation instead of the rate limit.
    fixture
      .state
      .rate_limiter
      .clear("sso_callback:unknown")
      .await;
    let started = start_login(&fixture.router).await;
    let code = format!("forged-{index}");
    idp.raw(&code, forge(idp, &started));
    assert_eq!(
      callback(&fixture.router, &started, &code).await,
      "/login?sso_error=failed",
      "{name}"
    );
  }
  assert_eq!(linked_subjects(&fixture).await, ["root-subject"]);
  let sso_logins: i64 = sqlx::query_scalar(
    "SELECT COUNT(*) FROM audit_events WHERE action='login.succeeded' AND json_extract(metadata,'$.method')='sso'",
  )
  .fetch_one(fixture.state.db.pool())
  .await
  .unwrap();
  assert_eq!(sso_logins, 0);

  // The same claims, properly signed, do sign in: the cases above failed on
  // what was forged, not on the claims themselves.
  fixture
    .state
    .rate_limiter
    .clear("sso_callback:unknown")
    .await;
  let started = start_login(&fixture.router).await;
  idp.raw(
    "genuine",
    MockIdp::sign(idp.key(), &es256(), &claims(idp, &started)),
  );
  let handoff = callback(&fixture.router, &started, "genuine").await;
  assert!(handoff.starts_with("/login/sso#code="), "{handoff}");
}

#[tokio::test]
async fn microsoft_accounts_link_by_email_only_with_domain_ownership() {
  let fixture = sso_fixture().await;
  // Changing only the preset keeps the provider verified and enabled.
  let mut body = provider_body(&fixture.idp);
  body["preset"] = json!("microsoft");
  body["clientSecret"] = Value::Null;
  let (status, saved) = save_provider(
    &fixture.router,
    &fixture.root_cookie,
    &fixture.root_csrf,
    body,
  )
  .await;
  assert_eq!(status, 200, "{saved}");
  assert_eq!(saved["data"]["provider"]["enabled"], true);

  // Entra lets other tenants put any address in `email`, even marked verified.
  let started = start_login(&fixture.router).await;
  let mut entra = claims(&fixture.idp, &started);
  entra["sub"] = json!("entra-subject");
  fixture.idp.raw(
    "no-edov",
    MockIdp::sign(fixture.idp.key(), &es256(), &entra),
  );
  assert_eq!(
    callback(&fixture.router, &started, "no-edov").await,
    "/login?sso_error=email_unverified"
  );

  let started = start_login(&fixture.router).await;
  let mut entra = claims(&fixture.idp, &started);
  entra["sub"] = json!("entra-subject");
  entra["xms_edov"] = json!(true);
  entra.as_object_mut().unwrap().remove("email_verified");
  fixture
    .idp
    .raw("edov", MockIdp::sign(fixture.idp.key(), &es256(), &entra));
  let handoff = callback(&fixture.router, &started, "edov").await;
  assert!(handoff.starts_with("/login/sso#code="), "{handoff}");
}

#[tokio::test]
async fn sign_ins_never_return_to_a_page_outside_the_admin_ui() {
  let fixture = sso_fixture().await;
  let attempt = |redirect: &'static str, code: &'static str| {
    let fixture = &fixture;
    async move {
      let encoded: String = url::form_urlencoded::byte_serialize(redirect.as_bytes()).collect();
      let (status, _, headers) = send(
        &fixture.router,
        "GET",
        &format!("/api/v1/auth/sso/start?redirect={encoded}"),
        &[],
        None,
        None,
      )
      .await;
      assert_eq!(status, 302);
      let authorize = location(&headers);
      let started = Started {
        binding: set_cookie(&headers, "dopbase_sso").unwrap(),
        state: super::sso_login::query_param(&authorize, "state"),
        nonce: super::sso_login::query_param(&authorize, "nonce"),
      };
      fixture.idp.grant(
        code,
        Grant {
          subject: "member-subject",
          email: "member@dopbase.com",
          email_verified: Some(true),
          nonce: started.nonce.clone(),
          auth_age: None,
        },
      );
      let handoff = callback(&fixture.router, &started, code).await;
      let (status, body, _) = exchange(&fixture.router, &started.binding, &handoff).await;
      assert_eq!(status, 200, "{body}");
      body["data"]["redirectTo"].clone()
    }
  };
  for (index, redirect) in [
    "//evil.example.test/phish",
    "https://evil.example.test/phish",
    "/\\evil.example.test",
    "/\t/evil.example.test",
    "/\n/evil.example.test",
    "javascript:alert(1)",
    "projects",
  ]
  .into_iter()
  .enumerate()
  {
    let code: &'static str = Box::leak(format!("bad-{index}").into_boxed_str());
    assert_eq!(attempt(redirect, code).await, Value::Null, "{redirect:?}");
  }
  assert_eq!(attempt("/projects/p/api", "good").await, "/projects/p/api");
}

#[tokio::test]
async fn the_handoff_cannot_be_posted_cross_site() {
  let fixture = sso_fixture().await;
  let started = start_login(&fixture.router).await;
  fixture.idp.grant(
    "member",
    Grant {
      subject: "member-subject",
      email: "member@dopbase.com",
      email_verified: Some(true),
      nonce: started.nonce.clone(),
      auth_age: None,
    },
  );
  let handoff = callback(&fixture.router, &started, "member").await;
  let code = handoff.strip_prefix("/login/sso#code=").unwrap();
  // A form on another site can only send simple content types.
  let response = fixture
    .router
    .clone()
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/api/v1/auth/sso/exchange")
        .header(header::COOKIE, &started.binding)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Body::from(json!({"code":code}).to_string()))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(response.status().as_u16(), 415);
  assert!(response.headers().get(header::SET_COOKIE).is_none());
  // The code is still good for the real page.
  let (status, _, _) = exchange(&fixture.router, &started.binding, &handoff).await;
  assert_eq!(status, 200);
}

#[tokio::test]
async fn only_root_controls_single_sign_on_and_root_links() {
  let fixture = sso_fixture().await;
  let (status, created, _) = send(
    &fixture.router,
    "POST",
    "/api/v1/users",
    &[&fixture.root_cookie],
    Some(&fixture.root_csrf),
    Some(json!({"email":"second-admin@dopbase.com","password":"correct-horse-789","role":"admin"})),
  )
  .await;
  assert_eq!(status, 201, "{created}");
  let login = |email: &'static str, password: &'static str| {
    let router = fixture.router.clone();
    async move {
      let (status, body, headers) = call(
        &router,
        "POST",
        "/api/v1/auth/login",
        None,
        Some(json!({"email":email,"password":password,"sessionKind":"browser"})),
      )
      .await;
      assert_eq!(status, 200, "{body}");
      (
        set_cookie(&headers, "dopbase_session").unwrap(),
        body["data"]["csrfToken"].as_str().unwrap().to_owned(),
      )
    }
  };
  let (admin_cookie, admin_csrf) = login("second-admin@dopbase.com", "correct-horse-789").await;
  for (method, path, body) in [
    ("GET", "/api/v1/sso/provider", None),
    (
      "PUT",
      "/api/v1/sso/provider",
      Some(provider_body(&fixture.idp)),
    ),
    (
      "PUT",
      "/api/v1/sso/provider/enabled",
      Some(json!({"enabled":false})),
    ),
    (
      "PUT",
      "/api/v1/sso/provider/required",
      Some(json!({"required":true})),
    ),
    ("DELETE", "/api/v1/sso/provider", None),
  ] {
    let (status, response, _) = send(
      &fixture.router,
      method,
      path,
      &[&admin_cookie],
      Some(&admin_csrf),
      body,
    )
    .await;
    assert_eq!(status, 403, "{method} {path}: {response}");
  }

  let root_id: String = sqlx::query_scalar("SELECT id FROM admins WHERE role='root'")
    .fetch_one(fixture.state.db.pool())
    .await
    .unwrap();
  let (status, body, _) = send(
    &fixture.router,
    "DELETE",
    &format!("/api/v1/users/{root_id}/sso"),
    &[&admin_cookie],
    Some(&admin_csrf),
    None,
  )
  .await;
  assert_eq!(status, 403);
  assert!(body["error"]["ROOT_PROTECTED"].is_string());

  // A member cannot unlink anyone.
  let admin_id: String =
    sqlx::query_scalar("SELECT id FROM admins WHERE email='second-admin@dopbase.com'")
      .fetch_one(fixture.state.db.pool())
      .await
      .unwrap();
  let (member_cookie, member_csrf) = login("member@dopbase.com", "correct-horse-456").await;
  let (status, _, _) = send(
    &fixture.router,
    "DELETE",
    &format!("/api/v1/users/{admin_id}/sso"),
    &[&member_cookie],
    Some(&member_csrf),
    None,
  )
  .await;
  assert_eq!(status, 403);
  assert_eq!(linked_subjects(&fixture).await, ["root-subject"]);
}

#[tokio::test]
async fn requiring_single_sign_on_signs_out_password_sessions() {
  let fixture = sso_fixture().await;
  let mut sessions = Vec::new();
  for kind in ["browser", "cli"] {
    let (status, body, headers) = call(
      &fixture.router,
      "POST",
      "/api/v1/auth/login",
      None,
      Some(json!({"email":"member@dopbase.com","password":"correct-horse-456","sessionKind":kind})),
    )
    .await;
    assert_eq!(status, 200);
    sessions.push(match kind {
      "cli" => format!("Bearer {}", body["data"]["token"].as_str().unwrap()),
      _ => set_cookie(&headers, "dopbase_session").unwrap(),
    });
  }
  let (status, body, _) = send(
    &fixture.router,
    "PUT",
    "/api/v1/sso/provider/required",
    &[&fixture.root_cookie],
    Some(&fixture.root_csrf),
    Some(json!({"required":true})),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  for session in &sessions {
    let response = fixture
      .router
      .clone()
      .oneshot(
        Request::builder()
          .uri("/api/v1/auth/session")
          .header(
            if session.starts_with("Bearer ") {
              header::AUTHORIZATION
            } else {
              header::COOKIE
            },
            session,
          )
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(response.status().as_u16(), 401, "{session}");
  }
  let (status, _, _) = send(
    &fixture.router,
    "GET",
    "/api/v1/auth/session",
    &[&fixture.root_cookie],
    None,
    None,
  )
  .await;
  assert_eq!(status, 200, "root stays signed in");
  let revoked: i64 = sqlx::query_scalar(
    "SELECT json_extract(metadata,'$.revokedSessions') FROM audit_events WHERE action='sso_provider.policy_changed'",
  )
  .fetch_one(fixture.state.db.pool())
  .await
  .unwrap();
  assert_eq!(revoked, 2);
}

#[tokio::test]
async fn anonymous_endpoints_cap_what_one_client_can_hold_open() {
  let fixture = sso_fixture().await;
  // The fixture's test sign-in already finished, so nothing is pending.
  for attempt in 0..20 {
    let (status, _, headers) = send(
      &fixture.router,
      "GET",
      "/api/v1/auth/sso/start",
      &[],
      None,
      None,
    )
    .await;
    assert_eq!(status, 302);
    assert!(
      !location(&headers).starts_with("/login"),
      "attempt {attempt}"
    );
  }
  let (_, _, headers) = send(
    &fixture.router,
    "GET",
    "/api/v1/auth/sso/start",
    &[],
    None,
    None,
  )
  .await;
  assert_eq!(location(&headers), "/login?sso_error=unavailable");

  for _ in 0..20 {
    let (status, _, _) = call(
      &fixture.router,
      "POST",
      "/api/v1/auth/cli/device",
      None,
      Some(json!({})),
    )
    .await;
    assert_eq!(status, 201);
  }
  let (status, body, _) = call(
    &fixture.router,
    "POST",
    "/api/v1/auth/cli/device",
    None,
    Some(json!({})),
  )
  .await;
  assert_eq!(status, 429);
  assert!(body["error"]["RATE_LIMITED"].is_string());
}

#[tokio::test]
async fn guessed_callbacks_are_rate_limited_without_flooding_the_audit_log() {
  let fixture = sso_fixture().await;
  for guess in 0..5 {
    let (_, _, headers) = send(
      &fixture.router,
      "GET",
      &format!("/api/v1/auth/sso/callback?code=x&state=guess-{guess}"),
      &["dopbase_sso=ssob_guess"],
      None,
      None,
    )
    .await;
    assert_eq!(location(&headers), "/login?sso_error=expired");
  }
  // Even a real sign-in from this client waits out the window.
  let started = start_login(&fixture.router).await;
  assert_eq!(
    callback(&fixture.router, &started, "unused").await,
    "/login?sso_error=rate_limited"
  );
  let audited: i64 =
    sqlx::query_scalar("SELECT COUNT(*) FROM audit_events WHERE action='sso_login.failed'")
      .fetch_one(fixture.state.db.pool())
      .await
      .unwrap();
  assert_eq!(
    audited, 0,
    "guesses without a sign-in leave no audit records"
  );
}

#[tokio::test]
async fn a_link_dies_with_the_session_that_asked_for_it() {
  let fixture = sso_fixture().await;
  let (status, body, headers) = call(
    &fixture.router,
    "POST",
    "/api/v1/auth/login",
    None,
    Some(
      json!({"email":"member@dopbase.com","password":"correct-horse-456","sessionKind":"browser"}),
    ),
  )
  .await;
  assert_eq!(status, 200);
  let cookie = set_cookie(&headers, "dopbase_session").unwrap();
  let csrf = body["data"]["csrfToken"].as_str().unwrap().to_owned();
  let started = start_link(&fixture.router, &cookie, &csrf).await;
  let (status, _) = browser_call(
    &fixture.router,
    "/api/v1/auth/logout",
    &cookie,
    Some(&csrf),
    json!({}),
  )
  .await;
  assert_eq!(status, 200);
  fixture.idp.grant(
    "late",
    Grant {
      subject: "member-subject",
      email: "member@dopbase.com",
      email_verified: Some(true),
      nonce: started.nonce.clone(),
      auth_age: None,
    },
  );
  assert_eq!(
    callback(&fixture.router, &started, "late").await,
    "/account?sso_error=expired"
  );
  assert_eq!(linked_subjects(&fixture).await, ["root-subject"]);
}

#[tokio::test]
async fn reauthentication_needs_a_linked_identity() {
  let fixture = sso_fixture().await;
  let (_, body, headers) = call(
    &fixture.router,
    "POST",
    "/api/v1/auth/login",
    None,
    Some(
      json!({"email":"member@dopbase.com","password":"correct-horse-456","sessionKind":"browser"}),
    ),
  )
  .await;
  let cookie = set_cookie(&headers, "dopbase_session").unwrap();
  let csrf = body["data"]["csrfToken"].as_str().unwrap();
  let (status, body, _) = send(
    &fixture.router,
    "POST",
    "/api/v1/auth/sso/reauth",
    &[&cookie],
    Some(csrf),
    None,
  )
  .await;
  assert_eq!(status, 409);
  assert!(body["error"]["SSO_NOT_LINKED"].is_string());
}

#[tokio::test]
async fn factory_reset_forgets_single_sign_on() {
  let fixture = sso_fixture().await;
  let (status, body, _) = send(
    &fixture.router,
    "POST",
    "/api/v1/instance/factory-reset",
    &[&fixture.root_cookie],
    Some(&fixture.root_csrf),
    Some(json!({"currentPassword":"correct-horse-123","confirmation":"FACTORY RESET","acknowledged":true})),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  for table in [
    "sso_providers",
    "admin_identities",
    "sso_flows",
    "cli_device_requests",
  ] {
    let rows: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
      .fetch_one(fixture.state.db.pool())
      .await
      .unwrap();
    assert_eq!(rows, 0, "{table}");
  }
}

#[tokio::test]
async fn sign_in_cookies_are_secure_behind_https() {
  let directory = TempDir::new().unwrap();
  let config = app::config::ServerConfig {
    data_dir: directory.path().to_path_buf(),
    database_url: format!("sqlite://{}", directory.path().join("dopbase.db").display()),
    master_key: app::config::MasterKeyConfig {
      provider: "file".into(),
      path: directory.path().join("master.key"),
    },
    public_url: "https://dopbase.example.test".into(),
    features: app::services::features::Features::all(),
    ..app::config::ServerConfig::default()
  };
  let state = app::server::build_setup_state(config).await.unwrap();
  let router = app::server::router(state.clone());
  let idp = MockIdp::start().await;
  let (ciphertext, nonce) = state
    .crypto
    .encrypt_master(
      CLIENT_SECRET.as_bytes(),
      &app::services::crypto::sso_client_secret_aad("sso_https"),
    )
    .unwrap();
  let now = chrono::Utc::now().to_rfc3339();
  sqlx::query("INSERT INTO sso_providers(id,preset,display_name,issuer,client_id,client_secret_ciphertext,client_secret_nonce,enabled,verified_at,created_at,updated_at) VALUES('sso_https','generic','Acme SSO',?,?,?,?,1,?,?,?)")
    .bind(&idp.url)
    .bind(CLIENT_ID)
    .bind(ciphertext)
    .bind(nonce)
    .bind(&now)
    .bind(&now)
    .bind(&now)
    .execute(state.db.pool())
    .await
    .unwrap();
  let (status, _, headers) = send(&router, "GET", "/api/v1/auth/sso/start", &[], None, None).await;
  assert_eq!(status, 302);
  let cookie = headers[header::SET_COOKIE].to_str().unwrap();
  assert!(cookie.contains("; Secure"), "{cookie}");
  assert!(
    cookie.contains("HttpOnly") && cookie.contains("SameSite=Lax"),
    "{cookie}"
  );
  // The provider sends the browser back to the public URL, whatever Host
  // header the request carried.
  assert!(location(&headers).contains(
    "redirect_uri=https%3A%2F%2Fdopbase.example.test%2Fapi%2Fv1%2Fauth%2Fsso%2Fcallback"
  ));
}

#[tokio::test]
async fn callbacks_never_echo_the_request_cookie() {
  let fixture = sso_fixture().await;
  let started = start_login(&fixture.router).await;
  fixture.idp.grant(
    "member",
    Grant {
      subject: "member-subject",
      email: "member@dopbase.com",
      email_verified: Some(true),
      nonce: started.nonce.clone(),
      auth_age: None,
    },
  );
  let callback_path = |state: &str| format!("/api/v1/auth/sso/callback?code=member&state={state}");
  let (status, _, headers) = send(
    &fixture.router,
    "GET",
    &callback_path(&started.state),
    &[&started.binding],
    None,
    None,
  )
  .await;
  assert_eq!(status, 302);
  assert!(location(&headers).starts_with("/login/sso#code="));
  assert!(
    headers.get(header::SET_COOKIE).is_none(),
    "the binding set by start stays as it is"
  );

  // A failed callback clears the binding with a fixed value, whatever the
  // request carried.
  let (_, _, headers) = send(
    &fixture.router,
    "GET",
    &callback_path("unknown"),
    &["dopbase_sso=attacker value"],
    None,
    None,
  )
  .await;
  assert_eq!(
    set_cookie(&headers, "dopbase_sso").as_deref(),
    Some("dopbase_sso=")
  );
}
