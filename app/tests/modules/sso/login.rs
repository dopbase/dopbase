use super::secrets_import::browser_login;
use super::*;
use axum::{
  extract::State,
  http::{HeaderMap, StatusCode},
  routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use p256::ecdsa::{Signature, SigningKey, signature::Signer};
use std::{
  collections::HashMap,
  sync::{Arc, Mutex},
};

pub(super) const CLIENT_ID: &str = "dopbase";
pub(super) const CLIENT_SECRET: &str = "fixture-client-secret";

/// What the mock provider puts in the ID token for one authorization code.
#[derive(Clone)]
pub(super) struct Grant {
  pub(super) subject: &'static str,
  pub(super) email: &'static str,
  pub(super) email_verified: Option<bool>,
  pub(super) nonce: String,
  /// Seconds since the user signed in at the provider, sent as `auth_time`.
  pub(super) auth_age: Option<i64>,
}

#[derive(Clone)]
pub(super) struct MockIdp {
  pub(super) url: String,
  key: SigningKey,
  grants: Arc<Mutex<HashMap<String, Grant>>>,
  /// Client credentials and PKCE verifiers seen by the token endpoint.
  token_requests: Arc<Mutex<Vec<(String, String)>>>,
  /// A raw ID token to hand out for a code, for forged-token tests.
  raw_tokens: Arc<Mutex<HashMap<String, String>>>,
}

impl MockIdp {
  pub(super) async fn start() -> Self {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let idp = Self {
      url: format!("http://{}", listener.local_addr().unwrap()),
      key: SigningKey::from_slice(&[7_u8; 32]).unwrap(),
      grants: Arc::default(),
      token_requests: Arc::default(),
      raw_tokens: Arc::default(),
    };
    let router = Router::new()
      .route("/.well-known/openid-configuration", get(discovery))
      .route("/jwks", get(jwks))
      .route("/token", post(token))
      .with_state(idp.clone());
    tokio::spawn(async move {
      axum::serve(listener, router).await.unwrap();
    });
    idp
  }

  pub(super) fn grant(
    &self,
    code: &str,
    grant: Grant,
  ) {
    self.grants.lock().unwrap().insert(code.into(), grant);
  }

  /// Hand out exactly `token` for `code`.
  pub(super) fn raw(
    &self,
    code: &str,
    token: String,
  ) {
    self.raw_tokens.lock().unwrap().insert(code.into(), token);
  }

  /// Sign `claims` with `key` under `header`, as an ES256 JWT.
  pub(super) fn sign(
    key: &SigningKey,
    header: &Value,
    claims: &Value,
  ) -> String {
    let input = format!(
      "{}.{}",
      URL_SAFE_NO_PAD.encode(header.to_string()),
      URL_SAFE_NO_PAD.encode(claims.to_string())
    );
    let signature: Signature = key.sign(input.as_bytes());
    format!("{input}.{}", URL_SAFE_NO_PAD.encode(signature.to_bytes()))
  }

  pub(super) fn key(&self) -> &SigningKey {
    &self.key
  }

  fn id_token(
    &self,
    grant: &Grant,
  ) -> String {
    let now = chrono::Utc::now().timestamp();
    let mut claims = json!({
      "iss": self.url,
      "aud": CLIENT_ID,
      "sub": grant.subject,
      "email": grant.email,
      "nonce": grant.nonce,
      "iat": now,
      "exp": now + 300,
    });
    if let Some(verified) = grant.email_verified {
      claims["email_verified"] = json!(verified);
    }
    if let Some(age) = grant.auth_age {
      claims["auth_time"] = json!(now - age);
    }
    let header = URL_SAFE_NO_PAD.encode(json!({"alg":"ES256","typ":"JWT","kid":"k1"}).to_string());
    let payload = URL_SAFE_NO_PAD.encode(claims.to_string());
    let input = format!("{header}.{payload}");
    let signature: Signature = self.key.sign(input.as_bytes());
    format!("{input}.{}", URL_SAFE_NO_PAD.encode(signature.to_bytes()))
  }
}

async fn discovery(State(idp): State<MockIdp>) -> axum::Json<Value> {
  axum::Json(json!({
    "issuer": idp.url,
    "authorization_endpoint": format!("{}/authorize", idp.url),
    "token_endpoint": format!("{}/token", idp.url),
    "jwks_uri": format!("{}/jwks", idp.url),
    "response_types_supported": ["code"],
    "subject_types_supported": ["public"],
    "id_token_signing_alg_values_supported": ["ES256"],
  }))
}

async fn jwks(State(idp): State<MockIdp>) -> axum::Json<Value> {
  let point = idp.key.verifying_key().to_encoded_point(false);
  axum::Json(json!({"keys":[{
    "kty": "EC",
    "crv": "P-256",
    "kid": "k1",
    "use": "sig",
    "alg": "ES256",
    "x": URL_SAFE_NO_PAD.encode(point.x().unwrap()),
    "y": URL_SAFE_NO_PAD.encode(point.y().unwrap()),
  }]}))
}

async fn token(
  State(idp): State<MockIdp>,
  headers: HeaderMap,
  body: String,
) -> (StatusCode, axum::Json<Value>) {
  let form: HashMap<String, String> = url::form_urlencoded::parse(body.as_bytes())
    .into_owned()
    .collect();
  let authorization = headers
    .get("authorization")
    .and_then(|value| value.to_str().ok())
    .unwrap_or_default()
    .to_owned();
  idp.token_requests.lock().unwrap().push((
    authorization,
    form.get("code_verifier").cloned().unwrap_or_default(),
  ));
  let raw = form
    .get("code")
    .and_then(|code| idp.raw_tokens.lock().unwrap().remove(code));
  if let Some(id_token) = raw {
    return (
      StatusCode::OK,
      axum::Json(json!({
        "access_token": "provider-access-token",
        "token_type": "Bearer",
        "expires_in": 300,
        "id_token": id_token,
      })),
    );
  }
  let grant = form
    .get("code")
    .and_then(|code| idp.grants.lock().unwrap().remove(code));
  match grant {
    Some(grant) => (
      StatusCode::OK,
      axum::Json(json!({
        "access_token": "provider-access-token",
        "token_type": "Bearer",
        "expires_in": 300,
        "id_token": idp.id_token(&grant),
      })),
    ),
    None => (
      StatusCode::BAD_REQUEST,
      axum::Json(json!({"error":"invalid_grant"})),
    ),
  }
}

pub(super) async fn send(
  router: &Router,
  method: &str,
  path: &str,
  cookies: &[&str],
  csrf: Option<&str>,
  body: Option<Value>,
) -> (u16, Value, axum::http::HeaderMap) {
  let mut request = Request::builder().method(method).uri(path);
  if !cookies.is_empty() {
    request = request.header(header::COOKIE, cookies.join("; "));
  }
  if let Some(csrf) = csrf {
    request = request.header("x-dopbase-csrf", csrf);
  }
  if body.is_some() {
    request = request.header(header::CONTENT_TYPE, "application/json");
  }
  let response = router
    .clone()
    .oneshot(
      request
        .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
        .unwrap(),
    )
    .await
    .unwrap();
  let status = response.status().as_u16();
  let headers = response.headers().clone();
  let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
  (
    status,
    serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    headers,
  )
}

pub(super) fn location(headers: &axum::http::HeaderMap) -> String {
  headers[header::LOCATION].to_str().unwrap().to_owned()
}

/// The `name=value` part of the Set-Cookie header for `name`.
pub(super) fn set_cookie(
  headers: &axum::http::HeaderMap,
  name: &str,
) -> Option<String> {
  headers
    .get_all(header::SET_COOKIE)
    .iter()
    .filter_map(|value| value.to_str().ok())
    .map(|value| value.split(';').next().unwrap().to_owned())
    .find(|value| value.starts_with(&format!("{name}=")))
}

pub(super) fn query_param(
  url: &str,
  name: &str,
) -> String {
  url::Url::parse(url)
    .unwrap()
    .query_pairs()
    .find(|(key, _)| key == name)
    .unwrap()
    .1
    .into_owned()
}

/// A sign-in at the provider that has reached the redirect back, ready for
/// the callback.
pub(super) struct Started {
  pub(super) binding: String,
  pub(super) state: String,
  pub(super) nonce: String,
}

pub(super) async fn start_login(router: &Router) -> Started {
  let (status, _, headers) = send(
    router,
    "GET",
    "/api/v1/auth/sso/start?redirect=/projects",
    &[],
    None,
    None,
  )
  .await;
  assert_eq!(status, 302);
  let authorize = location(&headers);
  assert_eq!(query_param(&authorize, "code_challenge_method"), "S256");
  assert_eq!(query_param(&authorize, "client_id"), CLIENT_ID);
  Started {
    binding: set_cookie(&headers, "dopbase_sso").unwrap(),
    state: query_param(&authorize, "state"),
    nonce: query_param(&authorize, "nonce"),
  }
}

pub(super) async fn start_link(
  router: &Router,
  cookie: &str,
  csrf: &str,
) -> Started {
  let (status, body, headers) = send(
    router,
    "POST",
    "/api/v1/auth/sso/link",
    &[cookie],
    Some(csrf),
    None,
  )
  .await;
  assert_eq!(status, 200, "{body}");
  let authorize = body["data"]["authorizeUrl"].as_str().unwrap();
  Started {
    binding: set_cookie(&headers, "dopbase_sso").unwrap(),
    state: query_param(authorize, "state"),
    nonce: query_param(authorize, "nonce"),
  }
}

pub(super) async fn callback(
  router: &Router,
  started: &Started,
  code: &str,
) -> String {
  let (status, _, headers) = send(
    router,
    "GET",
    &format!(
      "/api/v1/auth/sso/callback?code={code}&state={}",
      started.state
    ),
    &[&started.binding],
    None,
    None,
  )
  .await;
  assert_eq!(status, 302);
  location(&headers)
}

/// Exchange the handoff fragment the way the Admin UI does.
pub(super) async fn exchange(
  router: &Router,
  binding: &str,
  location: &str,
) -> (u16, Value, axum::http::HeaderMap) {
  let code = location
    .strip_prefix("/login/sso#code=")
    .unwrap_or_else(|| panic!("not a handoff location: {location}"));
  send(
    router,
    "POST",
    "/api/v1/auth/sso/exchange",
    &[binding],
    None,
    Some(json!({"code":code})),
  )
  .await
}

pub(super) async fn save_provider(
  router: &Router,
  cookie: &str,
  csrf: &str,
  body: Value,
) -> (u16, Value) {
  let (status, body, _) = send(
    router,
    "PUT",
    "/api/v1/sso/provider",
    &[cookie],
    Some(csrf),
    Some(body),
  )
  .await;
  (status, body)
}

pub(super) fn provider_body(idp: &MockIdp) -> Value {
  json!({
    "preset": "generic",
    "displayName": "Acme SSO",
    "issuer": idp.url,
    "clientId": CLIENT_ID,
    "clientSecret": CLIENT_SECRET,
  })
}

pub(super) struct SsoFixture {
  _dir: TempDir,
  pub(super) state: app::state::AppState,
  pub(super) router: Router,
  pub(super) idp: MockIdp,
  pub(super) root_cookie: String,
  pub(super) root_csrf: String,
}

/// Root has configured the mock provider, passed the test sign-in, and
/// enabled it. A member account exists with a password.
pub(super) async fn sso_fixture() -> SsoFixture {
  let (dir, state, router) = test_app().await;
  bootstrap_admin(&state, &router).await;
  let idp = MockIdp::start().await;
  let (root_cookie, root_csrf) = browser_login(&router).await;
  let (status, created, _) = send(
    &router,
    "POST",
    "/api/v1/users",
    &[&root_cookie],
    Some(&root_csrf),
    Some(json!({"email":"member@dopbase.com","password":"correct-horse-456","role":"member"})),
  )
  .await;
  assert_eq!(status, 201, "{created}");
  let (status, body) = save_provider(&router, &root_cookie, &root_csrf, provider_body(&idp)).await;
  assert_eq!(status, 200, "{body}");
  let started = start_link(&router, &root_cookie, &root_csrf).await;
  idp.grant(
    "root-code",
    Grant {
      subject: "root-subject",
      email: "admin@dopbase.com",
      email_verified: Some(true),
      nonce: started.nonce.clone(),
      auth_age: None,
    },
  );
  assert_eq!(
    callback(&router, &started, "root-code").await,
    "/sso?linked=1"
  );
  let (status, body, _) = send(
    &router,
    "PUT",
    "/api/v1/sso/provider/enabled",
    &[&root_cookie],
    Some(&root_csrf),
    Some(json!({"enabled":true})),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  SsoFixture {
    _dir: dir,
    state,
    router,
    idp,
    root_cookie,
    root_csrf,
  }
}

#[tokio::test]
async fn root_must_pass_a_test_sign_in_before_enabling() {
  let (_dir, state, router) = test_app().await;
  bootstrap_admin(&state, &router).await;
  let idp = MockIdp::start().await;
  let (cookie, csrf) = browser_login(&router).await;
  let (status, body) = save_provider(&router, &cookie, &csrf, provider_body(&idp)).await;
  assert_eq!(status, 200, "{body}");
  let provider = &body["data"]["provider"];
  assert_eq!(provider["enabled"], false);
  assert!(provider["verifiedAt"].is_null());
  assert!(provider.get("clientSecret").is_none());
  assert_eq!(
    body["data"]["redirectUri"],
    "http://localhost:8840/api/v1/auth/sso/callback"
  );

  let (status, body, _) = send(
    &router,
    "PUT",
    "/api/v1/sso/provider/enabled",
    &[&cookie],
    Some(&csrf),
    Some(json!({"enabled":true})),
  )
  .await;
  assert_eq!(status, 409);
  assert!(body["error"]["SSO_NOT_VERIFIED"].is_string());

  // Nobody can start a login while it is off.
  let (status, _, headers) = send(&router, "GET", "/api/v1/auth/sso/start", &[], None, None).await;
  assert_eq!(status, 302);
  assert_eq!(location(&headers), "/login?sso_error=disabled");
  let (_, methods, _) = call(&router, "GET", "/api/v1/auth/methods", None, None).await;
  assert!(methods["data"]["sso"].is_null());

  let started = start_link(&router, &cookie, &csrf).await;
  idp.grant(
    "root-code",
    Grant {
      subject: "root-subject",
      email: "someone-else@dopbase.com",
      email_verified: None,
      nonce: started.nonce.clone(),
      auth_age: None,
    },
  );
  assert_eq!(
    callback(&router, &started, "root-code").await,
    "/sso?linked=1"
  );
  // The client secret was decrypted and sent, along with the PKCE verifier.
  let (authorization, verifier) = idp.token_requests.lock().unwrap()[0].clone();
  let expected =
    base64::engine::general_purpose::STANDARD.encode(format!("{CLIENT_ID}:{CLIENT_SECRET}"));
  assert_eq!(authorization, format!("Basic {expected}"));
  assert!(verifier.len() >= 43);

  let (status, body, _) = send(
    &router,
    "PUT",
    "/api/v1/sso/provider/enabled",
    &[&cookie],
    Some(&csrf),
    Some(json!({"enabled":true})),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["provider"]["enabled"], true);
  assert_eq!(body["data"]["provider"]["linkedAccounts"], 1);
  let (_, methods, _) = call(&router, "GET", "/api/v1/auth/methods", None, None).await;
  assert_eq!(methods["data"]["sso"]["name"], "Acme SSO");

  // New client settings are unproven again.
  let mut changed = provider_body(&idp);
  changed["clientId"] = json!("another-client");
  changed["clientSecret"] = Value::Null;
  let (status, body) = save_provider(&router, &cookie, &csrf, changed).await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["provider"]["enabled"], false);
  assert!(body["data"]["provider"]["verifiedAt"].is_null());

  let actions: Vec<String> =
    sqlx::query_scalar("SELECT action FROM audit_events WHERE action LIKE 'sso_%' ORDER BY seq")
      .fetch_all(state.db.pool())
      .await
      .unwrap();
  assert_eq!(
    actions,
    [
      "sso_provider.created",
      "sso_identity.linked",
      "sso_provider.verified",
      "sso_provider.enabled",
      "sso_provider.updated",
    ]
  );
}

#[tokio::test]
async fn provider_settings_are_validated_and_root_only() {
  let (_dir, state, router) = test_app().await;
  bootstrap_admin(&state, &router).await;
  let idp = MockIdp::start().await;
  let (cookie, csrf) = browser_login(&router).await;
  let mut body = provider_body(&idp);
  body["issuer"] = json!("http://idp.example.com");
  let (status, response) = save_provider(&router, &cookie, &csrf, body).await;
  assert_eq!(status, 422);
  assert!(response["error"]["SSO_ISSUER_INVALID"].is_string());

  let mut body = provider_body(&idp);
  body["preset"] = json!("microsoft");
  body["issuer"] = json!("https://login.microsoftonline.com/common/v2.0");
  let (status, response) = save_provider(&router, &cookie, &csrf, body).await;
  assert_eq!(status, 422);
  assert!(response["error"]["SSO_ISSUER_INVALID"].is_string());

  let mut body = provider_body(&idp);
  body["issuer"] = json!("http://127.0.0.1:1");
  let (status, response) = save_provider(&router, &cookie, &csrf, body).await;
  assert_eq!(status, 422);
  assert!(response["error"]["SSO_DISCOVERY_FAILED"].is_string());

  let mut body = provider_body(&idp);
  body["clientSecret"] = Value::Null;
  let (status, response) = save_provider(&router, &cookie, &csrf, body).await;
  assert_eq!(status, 422);
  assert!(response["error"]["SSO_CLIENT_INVALID"].is_string());

  // A CLI session, even root's, cannot change it.
  let cli = bootstrap_cli_token(&router).await;
  let (status, _, _) = call(
    &router,
    "PUT",
    "/api/v1/sso/provider",
    Some(&cli),
    Some(provider_body(&idp)),
  )
  .await;
  assert_eq!(status, 403);

  super::secrets_import::expire_confirmation(&state).await;
  let (status, response) = save_provider(&router, &cookie, &csrf, provider_body(&idp)).await;
  assert_eq!(status, 403);
  assert!(response["error"]["RECENT_AUTHENTICATION_REQUIRED"].is_string());
}

async fn bootstrap_cli_token(router: &Router) -> String {
  let (status, login, _) = call(
    router,
    "POST",
    "/api/v1/auth/login",
    None,
    Some(json!({"email":"admin@dopbase.com","password":"correct-horse-123","sessionKind":"cli"})),
  )
  .await;
  assert_eq!(status, 200);
  login["data"]["token"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn verified_email_links_once_then_the_subject_signs_in() {
  let fixture = sso_fixture().await;
  let router = &fixture.router;
  let started = start_login(router).await;
  fixture.idp.grant(
    "member-1",
    Grant {
      subject: "member-subject",
      email: "Member@Dopbase.com",
      email_verified: Some(true),
      nonce: started.nonce.clone(),
      auth_age: None,
    },
  );
  let handoff = callback(router, &started, "member-1").await;
  let (status, body, headers) = exchange(router, &started.binding, &handoff).await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["session"]["email"], "member@dopbase.com");
  assert_eq!(body["data"]["session"]["sessionKind"], "browser");
  assert!(body["data"]["session"]["token"].is_null());
  assert!(body["data"]["session"]["csrfToken"].is_string());
  assert_eq!(body["data"]["redirectTo"], "/projects");
  let session = set_cookie(&headers, "dopbase_session").unwrap();
  assert_eq!(set_cookie(&headers, "dopbase_sso").unwrap(), "dopbase_sso=");
  let (status, current, _) = send(
    router,
    "GET",
    "/api/v1/auth/session",
    &[&session],
    None,
    None,
  )
  .await;
  assert_eq!(status, 200);
  assert_eq!(current["data"]["email"], "member@dopbase.com");

  // The handoff works once.
  let (status, body, _) = exchange(router, &started.binding, &handoff).await;
  assert_eq!(status, 401);
  assert!(body["error"]["SSO_HANDOFF_INVALID"].is_string());

  // The provider changed the email; the stored subject still finds the
  // account, and the email is never matched again.
  let started = start_login(router).await;
  fixture.idp.grant(
    "member-2",
    Grant {
      subject: "member-subject",
      email: "renamed@dopbase.com",
      email_verified: Some(false),
      nonce: started.nonce.clone(),
      auth_age: None,
    },
  );
  let handoff = callback(router, &started, "member-2").await;
  let (status, body, _) = exchange(router, &started.binding, &handoff).await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["session"]["email"], "member@dopbase.com");

  let methods: Vec<String> = sqlx::query_scalar(
    "SELECT json_extract(metadata,'$.method') FROM audit_events WHERE action='login.succeeded' AND json_extract(metadata,'$.method') IS NOT NULL",
  )
  .fetch_all(fixture.state.db.pool())
  .await
  .unwrap();
  assert_eq!(methods, ["sso", "sso"]);
  let (provider_tokens,): (i64,) = sqlx::query_as(
    "SELECT COUNT(*) FROM audit_events WHERE metadata LIKE '%provider-access-token%'",
  )
  .fetch_one(fixture.state.db.pool())
  .await
  .unwrap();
  assert_eq!(provider_tokens, 0);
}

#[tokio::test]
async fn unverified_unknown_and_root_emails_are_refused() {
  let fixture = sso_fixture().await;
  let router = &fixture.router;
  for (code, subject, email, verified, reason) in [
    (
      "c1",
      "s1",
      "member@dopbase.com",
      Some(false),
      "email_unverified",
    ),
    ("c2", "s2", "member@dopbase.com", None, "email_unverified"),
    (
      "c3",
      "s3",
      "stranger@dopbase.com",
      Some(true),
      "account_not_found",
    ),
    // Root links only on purpose, from the settings page.
    (
      "c4",
      "s4",
      "admin@dopbase.com",
      Some(true),
      "root_not_linked",
    ),
  ] {
    let started = start_login(router).await;
    fixture.idp.grant(
      code,
      Grant {
        subject,
        email,
        email_verified: verified,
        nonce: started.nonce.clone(),
        auth_age: None,
      },
    );
    assert_eq!(
      callback(router, &started, code).await,
      format!("/login?sso_error={reason}")
    );
  }
  let linked: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admin_identities")
    .fetch_one(fixture.state.db.pool())
    .await
    .unwrap();
  assert_eq!(linked, 1, "only root's test sign-in is linked");
  let reasons: Vec<String> = sqlx::query_scalar(
    "SELECT json_extract(metadata,'$.reason') FROM audit_events WHERE action='sso_login.failed' ORDER BY seq",
  )
  .fetch_all(fixture.state.db.pool())
  .await
  .unwrap();
  assert_eq!(
    reasons,
    [
      "email_unverified",
      "email_unverified",
      "account_not_found",
      "root_not_linked"
    ]
  );
}

#[tokio::test]
async fn a_callback_works_only_in_the_browser_that_started_it() {
  let fixture = sso_fixture().await;
  let router = &fixture.router;
  let started = start_login(router).await;
  fixture.idp.grant(
    "member-1",
    Grant {
      subject: "member-subject",
      email: "member@dopbase.com",
      email_verified: Some(true),
      nonce: started.nonce.clone(),
      auth_age: None,
    },
  );
  let path = format!(
    "/api/v1/auth/sso/callback?code=member-1&state={}",
    started.state
  );
  for cookies in [vec![], vec!["dopbase_sso=ssob_wrong"]] {
    let (status, _, headers) = send(router, "GET", &path, &cookies, None, None).await;
    assert_eq!(status, 302);
    assert_eq!(location(&headers), "/login?sso_error=expired");
  }
  // Those attempts did not burn the real sign-in.
  let handoff = callback(router, &started, "member-1").await;
  assert!(handoff.starts_with("/login/sso#code="), "{handoff}");
  // Nor can the handoff be used from another browser.
  let (status, _, _) = exchange(router, "dopbase_sso=ssob_wrong", &handoff).await;
  assert_eq!(status, 401);
  let (status, _, _) = exchange(router, &started.binding, &handoff).await;
  assert_eq!(status, 200);
  // A consumed state cannot be replayed.
  assert_eq!(
    callback(router, &started, "member-1").await,
    "/login?sso_error=expired"
  );
}

#[tokio::test]
async fn provider_errors_and_bad_tokens_fail_the_sign_in() {
  let fixture = sso_fixture().await;
  let router = &fixture.router;

  let started = start_login(router).await;
  let (_, _, headers) = send(
    router,
    "GET",
    &format!(
      "/api/v1/auth/sso/callback?error=access_denied&state={}",
      started.state
    ),
    &[&started.binding],
    None,
    None,
  )
  .await;
  assert_eq!(location(&headers), "/login?sso_error=cancelled");

  // An ID token minted for another sign-in carries the wrong nonce.
  let started = start_login(router).await;
  fixture.idp.grant(
    "wrong-nonce",
    Grant {
      subject: "member-subject",
      email: "member@dopbase.com",
      email_verified: Some(true),
      nonce: "some-other-nonce".into(),
      auth_age: None,
    },
  );
  assert_eq!(
    callback(router, &started, "wrong-nonce").await,
    "/login?sso_error=failed"
  );

  // The provider refuses an unknown code.
  let started = start_login(router).await;
  assert_eq!(
    callback(router, &started, "never-issued").await,
    "/login?sso_error=failed"
  );
}

#[tokio::test]
async fn an_identity_links_to_one_account_only() {
  let fixture = sso_fixture().await;
  let router = &fixture.router;
  let (status, login, headers) = call(
    router,
    "POST",
    "/api/v1/auth/login",
    None,
    Some(
      json!({"email":"member@dopbase.com","password":"correct-horse-456","sessionKind":"browser"}),
    ),
  )
  .await;
  assert_eq!(status, 200);
  let cookie = headers[header::SET_COOKIE]
    .to_str()
    .unwrap()
    .split(';')
    .next()
    .unwrap()
    .to_owned();
  let csrf = login["data"]["csrfToken"].as_str().unwrap().to_owned();

  let (_, status_before, _) = send(
    router,
    "GET",
    "/api/v1/auth/sso/identity",
    &[&cookie],
    None,
    None,
  )
  .await;
  assert_eq!(status_before["data"]["available"], true);
  assert_eq!(status_before["data"]["linked"], false);
  assert_eq!(status_before["data"]["providerName"], "Acme SSO");

  // Root's subject is already taken.
  let started = start_link(router, &cookie, &csrf).await;
  fixture.idp.grant(
    "taken",
    Grant {
      subject: "root-subject",
      email: "member@dopbase.com",
      email_verified: Some(true),
      nonce: started.nonce.clone(),
      auth_age: None,
    },
  );
  assert_eq!(
    callback(router, &started, "taken").await,
    "/account?sso_error=identity_taken"
  );

  // An unverified email is fine when linking on purpose: the user proved
  // the account by being signed in to it.
  let started = start_link(router, &cookie, &csrf).await;
  fixture.idp.grant(
    "mine",
    Grant {
      subject: "member-subject",
      email: "personal@dopbase.com",
      email_verified: Some(false),
      nonce: started.nonce.clone(),
      auth_age: None,
    },
  );
  assert_eq!(
    callback(router, &started, "mine").await,
    "/account?sso=linked"
  );
  let (_, status_after, _) = send(
    router,
    "GET",
    "/api/v1/auth/sso/identity",
    &[&cookie],
    None,
    None,
  )
  .await;
  assert_eq!(status_after["data"]["linked"], true);
  assert_eq!(status_after["data"]["emailAtLink"], "personal@dopbase.com");

  // Root deleting the provider removes every link.
  let (status, _, _) = send(
    router,
    "DELETE",
    "/api/v1/sso/provider",
    &[&fixture.root_cookie],
    Some(&fixture.root_csrf),
    None,
  )
  .await;
  assert_eq!(status, 200);
  let linked: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admin_identities")
    .fetch_one(fixture.state.db.pool())
    .await
    .unwrap();
  assert_eq!(linked, 0);
}

#[tokio::test]
async fn single_sign_on_needs_an_https_public_url() {
  let directory = TempDir::new().unwrap();
  let config = app::config::ServerConfig {
    data_dir: directory.path().to_path_buf(),
    database_url: format!("sqlite://{}", directory.path().join("dopbase.db").display()),
    master_key: app::config::MasterKeyConfig {
      provider: "file".into(),
      path: directory.path().join("master.key"),
    },
    public_url: "http://dopbase.example.test".into(),
    features: app::services::features::Features::all(),
    ..app::config::ServerConfig::default()
  };
  let state = app::server::build_setup_state(config).await.unwrap();
  let router = app::server::router(state.clone());
  bootstrap_admin(&state, &router).await;
  let idp = MockIdp::start().await;
  let (cookie, csrf) = browser_login(&router).await;

  let (status, settings, _) = send(
    &router,
    "GET",
    "/api/v1/sso/provider",
    &[&cookie],
    None,
    None,
  )
  .await;
  assert_eq!(status, 200);
  assert_eq!(settings["data"]["publicUrlSecure"], false);
  assert_eq!(settings["data"]["publicUrl"], "http://dopbase.example.test");

  let (status, body) = save_provider(&router, &cookie, &csrf, provider_body(&idp)).await;
  assert_eq!(status, 409, "{body}");
  let message = body["error"]["SSO_PUBLIC_URL_INSECURE"].as_str().unwrap();
  assert!(message.contains("http://dopbase.example.test"), "{message}");
  assert!(message.contains("--public-url"), "{message}");

  // A provider left over from when the URL was HTTPS cannot be used either.
  let (ciphertext, nonce) = state
    .crypto
    .encrypt_master(
      CLIENT_SECRET.as_bytes(),
      &app::services::crypto::sso_client_secret_aad("sso_left"),
    )
    .unwrap();
  let now = chrono::Utc::now().to_rfc3339();
  sqlx::query("INSERT INTO sso_providers(id,preset,display_name,issuer,client_id,client_secret_ciphertext,client_secret_nonce,enabled,verified_at,created_at,updated_at) VALUES('sso_left','generic','Acme SSO',?,?,?,?,1,?,?,?)")
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
  assert_eq!(location(&headers), "/login?sso_error=unavailable");
}

#[tokio::test]
async fn an_issuer_entered_without_its_trailing_slash_is_corrected() {
  let (_dir, state, router) = test_app().await;
  bootstrap_admin(&state, &router).await;
  let idp = MockIdp::start().await;
  let (cookie, csrf) = browser_login(&router).await;
  // The mock's issuer has no trailing slash; entering one must still work,
  // and the stored issuer must be the one the provider publishes.
  let mut body = provider_body(&idp);
  body["preset"] = json!("auth0");
  body["issuer"] = json!(format!("{}/", idp.url));
  let (status, saved) = save_provider(&router, &cookie, &csrf, body).await;
  assert_eq!(status, 200, "{saved}");
  assert_eq!(saved["data"]["provider"]["issuer"], idp.url);
  assert_eq!(saved["data"]["provider"]["preset"], "auth0");
}

#[tokio::test]
async fn switching_providers_relinks_people_by_verified_email() {
  let fixture = sso_fixture().await;
  let router = &fixture.router;
  let (cookie, csrf) = (&fixture.root_cookie, &fixture.root_csrf);

  // Single sign-on is required, and the member signs in through the first
  // provider, say Google.
  let (status, body, _) = send(
    router,
    "PUT",
    "/api/v1/sso/provider/required",
    &[cookie],
    Some(csrf),
    Some(json!({"required":true})),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  let started = start_login(router).await;
  fixture.idp.grant(
    "google-member",
    Grant {
      subject: "google-subject",
      email: "member@dopbase.com",
      email_verified: Some(true),
      nonce: started.nonce.clone(),
      auth_age: None,
    },
  );
  let handoff = callback(router, &started, "google-member").await;
  let (status, body, headers) = exchange(router, &started.binding, &handoff).await;
  assert_eq!(status, 200, "{body}");
  let member_session = set_cookie(&headers, "dopbase_session").unwrap();

  // Root moves to a second provider, say Keycloak, with the same people.
  let keycloak = MockIdp::start().await;
  let mut body = provider_body(&keycloak);
  body["preset"] = json!("keycloak");
  body["clientSecret"] = Value::Null;
  let (status, saved) = save_provider(router, cookie, csrf, body).await;
  assert_eq!(status, 200, "{saved}");
  let provider = &saved["data"]["provider"];
  assert_eq!(provider["issuer"], keycloak.url);
  assert_eq!(provider["enabled"], false);
  assert_eq!(provider["requireSso"], false);
  assert!(provider["verifiedAt"].is_null());
  // Links to the old issuer, root's included, are gone.
  assert_eq!(provider["linkedAccounts"], 0);
  let removed: i64 = sqlx::query_scalar(
    "SELECT json_extract(metadata,'$.identitiesRemoved') FROM audit_events WHERE action='sso_provider.updated'",
  )
  .fetch_one(fixture.state.db.pool())
  .await
  .unwrap();
  assert_eq!(removed, 2);
  // Changing providers signs nobody out.
  let (status, _, _) = send(
    router,
    "GET",
    "/api/v1/auth/session",
    &[&member_session],
    None,
    None,
  )
  .await;
  assert_eq!(status, 200);

  // Until root proves the new provider, nobody can use it.
  let (_, _, headers) = send(router, "GET", "/api/v1/auth/sso/start", &[], None, None).await;
  assert_eq!(location(&headers), "/login?sso_error=disabled");
  let started = start_link(router, cookie, csrf).await;
  keycloak.grant(
    "keycloak-root",
    Grant {
      subject: "keycloak-root-subject",
      email: "admin@dopbase.com",
      email_verified: Some(true),
      nonce: started.nonce.clone(),
      auth_age: None,
    },
  );
  assert_eq!(
    callback(router, &started, "keycloak-root").await,
    "/sso?linked=1"
  );
  let (status, body, _) = send(
    router,
    "PUT",
    "/api/v1/sso/provider/enabled",
    &[cookie],
    Some(csrf),
    Some(json!({"enabled":true})),
  )
  .await;
  assert_eq!(status, 200, "{body}");

  // The member's new identity has another subject but the same verified
  // email, so it links to the same account.
  let started = start_login(router).await;
  keycloak.grant(
    "keycloak-member",
    Grant {
      subject: "keycloak-subject",
      email: "member@dopbase.com",
      email_verified: Some(true),
      nonce: started.nonce.clone(),
      auth_age: None,
    },
  );
  let handoff = callback(router, &started, "keycloak-member").await;
  let (status, body, _) = exchange(router, &started.binding, &handoff).await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["session"]["email"], "member@dopbase.com");
  let subjects: Vec<String> =
    sqlx::query_scalar("SELECT subject FROM admin_identities ORDER BY subject")
      .fetch_all(fixture.state.db.pool())
      .await
      .unwrap();
  assert_eq!(subjects, ["keycloak-root-subject", "keycloak-subject"]);
}

#[tokio::test]
async fn turning_sso_off_hides_it_and_brings_password_sign_in_back() {
  use app::services::features::Features;
  let fixture = sso_fixture().await;
  let root = (&fixture.root_cookie, fixture.root_csrf.as_str());
  let (status, body, _) = send(
    &fixture.router,
    "PUT",
    "/api/v1/sso/provider/required",
    &[root.0],
    Some(root.1),
    Some(json!({"required":true})),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  let member_login = json!({
    "email":"member@dopbase.com","password":"correct-horse-456","sessionKind":"cli"
  });
  let (status, _, _) = call(
    &fixture.router,
    "POST",
    "/api/v1/auth/login",
    None,
    Some(member_login.clone()),
  )
  .await;
  assert_eq!(status, 403, "required single sign-on blocks the password");

  let off = with_features(
    &fixture.state,
    Features {
      sso: false,
      ..Features::all()
    },
  );
  // The login page offers the password form and no provider button.
  let (status, methods, _) = send(&off, "GET", "/api/v1/auth/methods", &[], None, None).await;
  assert_eq!(status, 200);
  assert_eq!(methods["data"]["sso"], Value::Null);
  assert_eq!(methods["data"]["password"], true);
  assert_eq!(methods["data"]["passwordForm"], true);
  // A stored "required" no longer blocks password sign-in.
  let (status, body, _) = call(&off, "POST", "/api/v1/auth/login", None, Some(member_login)).await;
  assert_eq!(status, 200, "{body}");

  // Every single sign-on route answers the same way, root included.
  for (method, path) in [
    ("GET", "/api/v1/auth/sso/start"),
    ("GET", "/api/v1/sso/provider"),
    ("PUT", "/api/v1/sso/provider/enabled"),
    ("GET", "/api/v1/auth/sso/identity"),
  ] {
    let (status, body, _) = send(
      &off,
      method,
      path,
      &[root.0],
      Some(root.1),
      Some(json!({"enabled":false})),
    )
    .await;
    assert_eq!(status, 403, "{method} {path}: {body}");
    let message = body["error"]["FEATURE_DISABLED"].as_str();
    assert!(
      message.is_some_and(|message| message.contains("DOPBASE_SSO=true")),
      "{method} {path}: {body}"
    );
  }
  let (status, session, _) = send(&off, "GET", "/api/v1/auth/session", &[root.0], None, None).await;
  assert_eq!(status, 200, "{session}");
  assert_eq!(session["data"]["features"], json!(["rbac", "rbac.roles"]));
  assert_eq!(session["data"]["ssoConfirmation"], Value::Null);
  let capabilities = session["data"]["capabilities"].as_array().unwrap();
  assert!(!capabilities.contains(&json!("sso.manage")), "{session}");
  assert!(capabilities.contains(&json!("users.manage")), "{session}");

  // Nothing was lost: turned back on, the provider is still set up.
  let (status, body, _) = send(
    &fixture.router,
    "GET",
    "/api/v1/sso/provider",
    &[root.0],
    None,
    None,
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["provider"]["enabled"], true);
  assert_eq!(body["data"]["provider"]["requireSso"], true);
}
