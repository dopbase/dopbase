use super::secrets_import::{browser_login, expire_confirmation};
use super::sso_login::{
  Grant, SsoFixture, callback, exchange, query_param, send, set_cookie, sso_fixture, start_login,
};
use super::*;

/// Create an admin-provisioned account, with or without a password.
async fn create_user(
  fixture: &SsoFixture,
  email: &str,
  password: Option<&str>,
) -> (u16, Value) {
  let mut body = json!({"email":email,"role":"member"});
  if let Some(password) = password {
    body["password"] = json!(password);
  }
  let (status, created, _) = send(
    &fixture.router,
    "POST",
    "/api/v1/users",
    &[&fixture.root_cookie],
    Some(&fixture.root_csrf),
    Some(body),
  )
  .await;
  (status, created)
}

/// Sign in through the mock provider and return the browser session.
async fn sso_session(
  fixture: &SsoFixture,
  code: &str,
  subject: &'static str,
  email: &'static str,
) -> (String, String) {
  let started = start_login(&fixture.router).await;
  fixture.idp.grant(
    code,
    Grant {
      subject,
      email,
      email_verified: Some(true),
      nonce: started.nonce.clone(),
      auth_age: None,
    },
  );
  let handoff = callback(&fixture.router, &started, code).await;
  let (status, body, headers) = exchange(&fixture.router, &started.binding, &handoff).await;
  assert_eq!(status, 200, "{body}");
  (
    set_cookie(&headers, "dopbase_session").unwrap(),
    body["data"]["session"]["csrfToken"]
      .as_str()
      .unwrap()
      .to_owned(),
  )
}

async fn password_login(
  router: &Router,
  email: &str,
  password: &str,
) -> (u16, Value) {
  let (status, body, _) = call(
    router,
    "POST",
    "/api/v1/auth/login",
    None,
    Some(json!({"email":email,"password":password,"sessionKind":"browser"})),
  )
  .await;
  (status, body)
}

async fn set_required(
  fixture: &SsoFixture,
  required: bool,
) -> (u16, Value) {
  let (status, body, _) = send(
    &fixture.router,
    "PUT",
    "/api/v1/sso/provider/required",
    &[&fixture.root_cookie],
    Some(&fixture.root_csrf),
    Some(json!({"required":required})),
  )
  .await;
  (status, body)
}

#[tokio::test]
async fn accounts_without_a_password_need_single_sign_on() {
  let (_dir, state, router) = test_app().await;
  bootstrap_admin(&state, &router).await;
  let (cookie, csrf) = browser_login(&router).await;
  let (status, body, _) = send(
    &router,
    "POST",
    "/api/v1/users",
    &[&cookie],
    Some(&csrf),
    Some(json!({"email":"sso-only@dopbase.com","role":"member"})),
  )
  .await;
  assert_eq!(status, 422, "{body}");
  assert!(body["error"]["PASSWORD_REQUIRED"].is_string());

  let fixture = sso_fixture().await;
  let (status, created) = create_user(&fixture, "sso-only@dopbase.com", None).await;
  assert_eq!(status, 201, "{created}");
  assert_eq!(created["data"]["hasPassword"], false);
  assert_eq!(created["data"]["ssoLinked"], false);

  // No password matches the empty one.
  for guess in ["", "correct-horse-123"] {
    let (status, _) = password_login(&fixture.router, "sso-only@dopbase.com", guess).await;
    assert!(status == 401 || status == 422, "{guess}: {status}");
  }
  sso_session(&fixture, "c1", "sso-only-subject", "sso-only@dopbase.com").await;

  let (_, users, _) = send(
    &fixture.router,
    "GET",
    "/api/v1/users",
    &[&fixture.root_cookie],
    None,
    None,
  )
  .await;
  let row = users["data"]
    .as_array()
    .unwrap()
    .iter()
    .find(|user| user["email"] == "sso-only@dopbase.com")
    .unwrap();
  assert_eq!(row["hasPassword"], false);
  assert_eq!(row["ssoLinked"], true);
  let root = &users["data"][0];
  assert_eq!(root["role"], "root");
  assert_eq!(root["hasPassword"], true);
  assert_eq!(root["ssoLinked"], true);

  let (_, settings, _) = send(
    &fixture.router,
    "GET",
    "/api/v1/sso/provider",
    &[&fixture.root_cookie],
    None,
    None,
  )
  .await;
  assert_eq!(settings["data"]["provider"]["passwordlessAccounts"], 1);
}

#[tokio::test]
async fn requiring_single_sign_on_keeps_root_as_the_way_back_in() {
  let fixture = sso_fixture().await;
  let (status, _) =
    password_login(&fixture.router, "member@dopbase.com", "correct-horse-456").await;
  assert_eq!(status, 200);

  let (status, body) = set_required(&fixture, true).await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["provider"]["requireSso"], true);
  let (_, methods, _) = call(&fixture.router, "GET", "/api/v1/auth/methods", None, None).await;
  assert_eq!(methods["data"]["password"], false);

  // A right password is refused with a reason; a wrong one stays generic.
  let (status, body) =
    password_login(&fixture.router, "member@dopbase.com", "correct-horse-456").await;
  assert_eq!(status, 403);
  assert!(body["error"]["PASSWORD_LOGIN_DISABLED"].is_string());
  let (status, body) =
    password_login(&fixture.router, "member@dopbase.com", "wrong-password-1").await;
  assert_eq!(status, 401);
  assert!(body["error"]["AUTHENTICATION_INVALID"].is_string());
  let (status, _, _) = call(
    &fixture.router,
    "POST",
    "/api/v1/auth/login",
    None,
    Some(json!({"email":"member@dopbase.com","password":"correct-horse-456","sessionKind":"cli"})),
  )
  .await;
  assert_eq!(status, 403);

  let (status, _) = password_login(&fixture.router, "admin@dopbase.com", "correct-horse-123").await;
  assert_eq!(status, 200, "root keeps password login");

  // Turning single sign-on off lifts the requirement with it.
  let (status, body, _) = send(
    &fixture.router,
    "PUT",
    "/api/v1/sso/provider/enabled",
    &[&fixture.root_cookie],
    Some(&fixture.root_csrf),
    Some(json!({"enabled":false})),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["provider"]["requireSso"], false);
  let (status, _) =
    password_login(&fixture.router, "member@dopbase.com", "correct-horse-456").await;
  assert_eq!(status, 200);
  let (status, body) = set_required(&fixture, true).await;
  assert_eq!(status, 409, "{body}");
}

#[tokio::test]
async fn single_sign_on_confirms_an_account_without_a_password() {
  let fixture = sso_fixture().await;
  create_user(&fixture, "sso-only@dopbase.com", None).await;
  let (cookie, csrf) =
    sso_session(&fixture, "c1", "sso-only-subject", "sso-only@dopbase.com").await;
  expire_confirmation(&fixture.state).await;

  let (_, session, _) = send(
    &fixture.router,
    "GET",
    "/api/v1/auth/session",
    &[&cookie],
    None,
    None,
  )
  .await;
  assert_eq!(session["data"]["recentAuthentication"], false);
  assert_eq!(session["data"]["passwordConfirmation"], false);
  assert_eq!(session["data"]["ssoConfirmation"], "Acme SSO");
  let (status, body, _) = send(
    &fixture.router,
    "POST",
    "/api/v1/auth/reauthenticate",
    &[&cookie],
    Some(&csrf),
    Some(json!({"password":""})),
  )
  .await;
  assert_eq!(status, 409);
  assert!(body["error"]["PASSWORD_NOT_SET"].is_string());

  let reauth = |code: &'static str, subject: &'static str, auth_age: Option<i64>| {
    let fixture = &fixture;
    let cookie = cookie.clone();
    let csrf = csrf.clone();
    async move {
      let (status, body, headers) = send(
        &fixture.router,
        "POST",
        "/api/v1/auth/sso/reauth",
        &[&cookie],
        Some(&csrf),
        None,
      )
      .await;
      assert_eq!(status, 200, "{body}");
      let url = body["data"]["authorizeUrl"].as_str().unwrap();
      assert_eq!(query_param(url, "max_age"), "0");
      assert_eq!(query_param(url, "prompt"), "login");
      let started = super::sso_login::Started {
        binding: set_cookie(&headers, "dopbase_sso").unwrap(),
        state: query_param(url, "state"),
        nonce: query_param(url, "nonce"),
      };
      fixture.idp.grant(
        code,
        Grant {
          subject,
          email: "sso-only@dopbase.com",
          email_verified: Some(true),
          nonce: started.nonce.clone(),
          auth_age,
        },
      );
      callback(&fixture.router, &started, code).await
    }
  };

  // The provider reused an old sign-in, or did not say when it was.
  assert_eq!(
    reauth("stale", "sso-only-subject", Some(600)).await,
    "/sso/confirmed?sso_error=reauth_not_fresh"
  );
  assert_eq!(
    reauth("missing", "sso-only-subject", None).await,
    "/sso/confirmed?sso_error=reauth_not_fresh"
  );
  // Someone else signed in at the provider.
  assert_eq!(
    reauth("other", "root-subject", Some(0)).await,
    "/sso/confirmed?sso_error=identity_mismatch"
  );
  let (_, session, _) = send(
    &fixture.router,
    "GET",
    "/api/v1/auth/session",
    &[&cookie],
    None,
    None,
  )
  .await;
  assert_eq!(session["data"]["recentAuthentication"], false);
  let mismatch: String = sqlx::query_scalar(
    "SELECT actor_label FROM audit_events WHERE action='sso_login.failed' AND json_extract(metadata,'$.reason')='identity_mismatch'",
  )
  .fetch_one(fixture.state.db.pool())
  .await
  .unwrap();
  assert_eq!(mismatch, "sso-only@dopbase.com");

  assert_eq!(
    reauth("fresh", "sso-only-subject", Some(0)).await,
    "/sso/confirmed"
  );
  let (_, session, _) = send(
    &fixture.router,
    "GET",
    "/api/v1/auth/session",
    &[&cookie],
    None,
    None,
  )
  .await;
  assert_eq!(session["data"]["recentAuthentication"], true);
  let method: String = sqlx::query_scalar(
    "SELECT json_extract(metadata,'$.method') FROM audit_events WHERE action='admin.reauthenticated' ORDER BY seq DESC LIMIT 1",
  )
  .fetch_one(fixture.state.db.pool())
  .await
  .unwrap();
  assert_eq!(method, "sso");
}

#[tokio::test]
async fn an_account_without_a_password_can_set_one_after_confirming() {
  let fixture = sso_fixture().await;
  create_user(&fixture, "sso-only@dopbase.com", None).await;
  let (cookie, csrf) =
    sso_session(&fixture, "c1", "sso-only-subject", "sso-only@dopbase.com").await;
  let change = |cookie: String, csrf: String| {
    let router = fixture.router.clone();
    async move {
      send(
        &router,
        "POST",
        "/api/v1/auth/change-password",
        &[&cookie],
        Some(&csrf),
        Some(json!({"newPassword":"brand-new-password-1"})),
      )
      .await
    }
  };

  expire_confirmation(&fixture.state).await;
  let (status, body, _) = change(cookie.clone(), csrf.clone()).await;
  assert_eq!(status, 403);
  assert!(body["error"]["RECENT_AUTHENTICATION_REQUIRED"].is_string());

  let (cookie, csrf) =
    sso_session(&fixture, "c2", "sso-only-subject", "sso-only@dopbase.com").await;
  let (status, body, _) = change(cookie, csrf).await;
  assert_eq!(status, 200, "{body}");
  let (status, _) = password_login(
    &fixture.router,
    "sso-only@dopbase.com",
    "brand-new-password-1",
  )
  .await;
  assert_eq!(status, 200);
}

#[tokio::test]
async fn links_can_be_removed_without_locking_anyone_out() {
  let fixture = sso_fixture().await;
  create_user(&fixture, "sso-only@dopbase.com", None).await;
  // The fixture's member has a password.
  let member_id: String =
    sqlx::query_scalar("SELECT id FROM admins WHERE email='member@dopbase.com'")
      .fetch_one(fixture.state.db.pool())
      .await
      .unwrap();
  let (cookie, csrf) =
    sso_session(&fixture, "c1", "sso-only-subject", "sso-only@dopbase.com").await;

  // Unlinking yourself needs a password to fall back on.
  let (status, body, _) = send(
    &fixture.router,
    "DELETE",
    "/api/v1/auth/sso/identity",
    &[&cookie],
    Some(&csrf),
    None,
  )
  .await;
  assert_eq!(status, 409);
  assert!(body["error"]["PASSWORD_NOT_SET"].is_string());

  let (cookie, csrf) = sso_session(&fixture, "c2", "member-subject", "member@dopbase.com").await;
  let (status, body, _) = send(
    &fixture.router,
    "DELETE",
    "/api/v1/auth/sso/identity",
    &[&cookie],
    Some(&csrf),
    None,
  )
  .await;
  assert_eq!(status, 200, "{body}");

  // An administrator removes a link, for example after the identity was
  // recreated at the provider. A verified email links it again later.
  sso_session(&fixture, "c3", "member-subject", "member@dopbase.com").await;
  let path = format!("/api/v1/users/{member_id}/sso");
  let (status, body, _) = send(
    &fixture.router,
    "DELETE",
    &path,
    &[&fixture.root_cookie],
    Some(&fixture.root_csrf),
    None,
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["ssoLinked"], false);
  let (status, body, _) = send(
    &fixture.router,
    "DELETE",
    &path,
    &[&fixture.root_cookie],
    Some(&fixture.root_csrf),
    None,
  )
  .await;
  assert_eq!(status, 404);
  assert!(body["error"]["SSO_NOT_LINKED"].is_string());

  let unlinked: Vec<String> = sqlx::query_scalar(
    "SELECT actor_type FROM audit_events WHERE action='sso_identity.unlinked' ORDER BY seq",
  )
  .fetch_all(fixture.state.db.pool())
  .await
  .unwrap();
  assert_eq!(unlinked, ["admin", "root"]);
}

#[tokio::test]
async fn the_password_form_can_be_hidden_without_turning_passwords_off() {
  let fixture = sso_fixture().await;
  let set = |hidden: bool| {
    let fixture = &fixture;
    async move {
      send(
        &fixture.router,
        "PUT",
        "/api/v1/sso/provider/password-form",
        &[&fixture.root_cookie],
        Some(&fixture.root_csrf),
        Some(json!({"hidden":hidden})),
      )
      .await
    }
  };
  let methods = || async {
    call(&fixture.router, "GET", "/api/v1/auth/methods", None, None)
      .await
      .1["data"]
      .clone()
  };
  assert_eq!(methods().await["passwordForm"], true);

  let (status, body, _) = set(true).await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["provider"]["hidePasswordForm"], true);
  let listed = methods().await;
  assert_eq!(listed["passwordForm"], false);
  assert_eq!(listed["password"], true, "passwords still work");
  // Only the page changes: the API still signs in with a password.
  let (status, _) =
    password_login(&fixture.router, "member@dopbase.com", "correct-horse-456").await;
  assert_eq!(status, 200);

  // With single sign-on off, the form always comes back.
  send(
    &fixture.router,
    "PUT",
    "/api/v1/sso/provider/enabled",
    &[&fixture.root_cookie],
    Some(&fixture.root_csrf),
    Some(json!({"enabled":false})),
  )
  .await;
  assert_eq!(methods().await["passwordForm"], true);

  // Root only.
  let (_, _, headers) = call(
    &fixture.router,
    "POST",
    "/api/v1/auth/login",
    None,
    Some(
      json!({"email":"member@dopbase.com","password":"correct-horse-456","sessionKind":"browser"}),
    ),
  )
  .await;
  let member_cookie = set_cookie(&headers, "dopbase_session").unwrap();
  let (status, _, _) = send(
    &fixture.router,
    "PUT",
    "/api/v1/sso/provider/password-form",
    &[&member_cookie],
    None,
    Some(json!({"hidden":false})),
  )
  .await;
  assert_eq!(status, 403);
}
