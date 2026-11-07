use super::secrets_import::{browser_call, browser_login};
use super::*;
use app::services::passcodes::{self, PasscodePurpose};

const ROOT: &str = "admin@dopbase.com";

/// Mint a launch passcode the way `dopbase admin passcode` does, straight
/// into the database. Returns the code as the CLI prints it.
async fn mint(
  state: &app::state::AppState,
  email: &str,
  ttl_seconds: i64,
) -> String {
  let admin_id: String = sqlx::query_scalar("SELECT id FROM admins WHERE email=?")
    .bind(email)
    .fetch_one(state.db.pool())
    .await
    .unwrap();
  let code = app::utils::passcode::generate().unwrap();
  let mut connection = state.db.pool().acquire().await.unwrap();
  passcodes::issue(
    &mut connection,
    PasscodePurpose::Launch,
    &admin_id,
    &code,
    chrono::Duration::seconds(ttl_seconds),
    Some("test-host"),
    chrono::Utc::now(),
  )
  .await
  .unwrap();
  app::utils::passcode::display(&code)
}

async fn launch(
  router: &Router,
  identity: &str,
  passcode: &str,
) -> (u16, Value, axum::http::HeaderMap) {
  call(
    router,
    "POST",
    "/api/v1/auth/launch",
    None,
    Some(json!({"identity":identity,"passcode":passcode})),
  )
  .await
}

async fn passcode_rows(state: &app::state::AppState) -> i64 {
  sqlx::query_scalar("SELECT COUNT(*) FROM passcodes")
    .fetch_one(state.db.pool())
    .await
    .unwrap()
}

/// A passcode for another account that expired and that nobody presented.
async fn stale_passcode(state: &app::state::AppState) {
  sqlx::query("INSERT INTO admins(id,email,password_hash,role,role_id,is_root,created_at,updated_at) VALUES('usr_stale','stale@dopbase.com','','member','rol_member',0,'t','t')")
    .execute(state.db.pool())
    .await
    .unwrap();
  mint(state, "stale@dopbase.com", -5).await;
}

async fn add_account(
  state: &app::state::AppState,
  email: &str,
  role_id: &str,
  status: &str,
) {
  sqlx::query("INSERT INTO admins(id,email,password_hash,role,role_id,is_root,status,created_at,updated_at) VALUES(?,?,'','member',?,0,?,'t','t')")
    .bind(format!("usr_{}", email.split('@').next().unwrap()))
    .bind(email)
    .bind(role_id)
    .bind(status)
    .execute(state.db.pool())
    .await
    .unwrap();
}

async fn browser_sessions(state: &app::state::AppState) -> i64 {
  sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE kind='browser'")
    .fetch_one(state.db.pool())
    .await
    .unwrap()
}

fn assert_invalid(
  status: u16,
  body: &Value,
) {
  assert_eq!(status, 401, "{body}");
  assert!(body["error"]["PASSCODE_INVALID"].is_string(), "{body}");
}

#[tokio::test]
async fn a_passcode_starts_a_browser_session_once() {
  let (_dir, state, router) = test_app().await;
  bootstrap_admin(&state, &router).await;
  let code = mint(&state, ROOT, 60).await;

  let (status, body, headers) = launch(&router, "ADMIN@dopbase.com", &code.to_lowercase()).await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["email"], ROOT);
  assert_eq!(body["data"]["sessionKind"], "browser");
  assert!(body["data"]["token"].is_null());
  let csrf = body["data"]["csrfToken"].as_str().unwrap();
  let cookie = headers.get(header::SET_COOKIE).unwrap().to_str().unwrap();
  assert!(cookie.starts_with("dopbase_session="));
  assert!(cookie.contains("HttpOnly; SameSite=Strict"));
  assert_eq!(passcode_rows(&state).await, 0);

  // The new session works like one from password login.
  let cookie = cookie.split(';').next().unwrap();
  let (status, body) = browser_call(
    &router,
    "/api/v1/projects",
    cookie,
    Some(csrf),
    json!({"name":"launched"}),
  )
  .await;
  assert_eq!(status, 201, "{body}");

  let (status, body, _) = launch(&router, ROOT, &code).await;
  assert_invalid(status, &body);
  state.db.close().await;
}

#[tokio::test]
async fn success_does_not_sweep_other_passcodes() {
  let (_dir, state, router) = test_app().await;
  bootstrap_admin(&state, &router).await;
  stale_passcode(&state).await;
  let code = mint(&state, ROOT, 60).await;
  let (status, body, _) = launch(&router, ROOT, &code).await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(passcode_rows(&state).await, 1, "the stale row stays");
  state.db.close().await;
}

#[tokio::test]
async fn an_expired_passcode_fails_and_sweeps() {
  let (_dir, state, router) = test_app().await;
  bootstrap_admin(&state, &router).await;
  stale_passcode(&state).await;
  let code = mint(&state, ROOT, -1).await;
  let (status, body, _) = launch(&router, ROOT, &code).await;
  assert_invalid(status, &body);
  assert_eq!(passcode_rows(&state).await, 0);
  state.db.close().await;
}

#[tokio::test]
async fn a_wrong_passcode_fails_and_sweeps_but_keeps_the_live_one() {
  let (_dir, state, router) = test_app().await;
  bootstrap_admin(&state, &router).await;
  stale_passcode(&state).await;
  let code = mint(&state, ROOT, 60).await;
  let (status, body, _) = launch(&router, ROOT, "BBBB-BBBB").await;
  assert_invalid(status, &body);
  assert_eq!(passcode_rows(&state).await, 1, "only the live code is left");
  let (status, body, _) = launch(&router, ROOT, &code).await;
  assert_eq!(status, 200, "{body}");
  state.db.close().await;
}

#[tokio::test]
async fn five_wrong_tries_burn_the_passcode() {
  let (_dir, state, router) = test_app().await;
  bootstrap_admin(&state, &router).await;
  let code = mint(&state, ROOT, 60).await;
  for wrong in ["BBBB-BBBB", "not a code", "", "CCCC-CCCC", "DDDD-DDDD"] {
    let (status, body, _) = launch(&router, ROOT, wrong).await;
    assert_invalid(status, &body);
  }
  assert_eq!(passcode_rows(&state).await, 0);
  // The limiter trips on the same try; clear it to show the code itself is
  // gone. In-process requests carry no client address.
  state
    .rate_limiter
    .clear(&format!("launch:unknown:{ROOT}"))
    .await;
  let (status, body, _) = launch(&router, ROOT, &code).await;
  assert_invalid(status, &body);
  state.db.close().await;
}

#[tokio::test]
async fn repeated_failures_for_one_email_are_rate_limited() {
  let (_dir, state, router) = test_app().await;
  bootstrap_admin(&state, &router).await;
  mint(&state, ROOT, 60).await;
  for _ in 0..5 {
    let (status, body, _) = launch(&router, ROOT, "BBBB-BBBB").await;
    assert_invalid(status, &body);
  }
  let code = mint(&state, ROOT, 60).await;
  let (status, body, _) = launch(&router, ROOT, &code).await;
  assert_eq!(status, 429, "{body}");
  assert_eq!(
    passcode_rows(&state).await,
    1,
    "a refused try never reaches the code"
  );
  state.db.close().await;
}

#[tokio::test]
async fn tries_without_a_live_passcode_cost_nothing() {
  let (_dir, state, router) = test_app().await;
  bootstrap_admin(&state, &router).await;
  // Someone who only knows the email can't use up the owner's tries.
  for _ in 0..10 {
    let (status, body, _) = launch(&router, ROOT, "BBBB-BBBB").await;
    assert_invalid(status, &body);
  }
  let code = mint(&state, ROOT, 60).await;
  let (status, body, _) = launch(&router, ROOT, &code).await;
  assert_eq!(status, 200, "{body}");
  state.db.close().await;
}

/// A launch request from `address`, the way a real connection arrives.
async fn launch_from(
  router: &Router,
  address: &str,
  identity: &str,
  passcode: &str,
) -> u16 {
  let mut request = Request::builder()
    .method("POST")
    .uri("/api/v1/auth/launch")
    .header(header::CONTENT_TYPE, "application/json")
    .body(Body::from(
      json!({"identity":identity,"passcode":passcode}).to_string(),
    ))
    .unwrap();
  request.extensions_mut().insert(axum::extract::ConnectInfo(
    address.parse::<std::net::SocketAddr>().unwrap(),
  ));
  router
    .clone()
    .oneshot(request)
    .await
    .unwrap()
    .status()
    .as_u16()
}

#[tokio::test]
async fn failures_from_one_address_do_not_lock_out_another() {
  let (_dir, state, router) = test_app().await;
  bootstrap_admin(&state, &router).await;
  mint(&state, ROOT, 60).await;
  for _ in 0..4 {
    assert_eq!(
      launch_from(&router, "198.51.100.7:4000", ROOT, "BBBB-BBBB").await,
      401
    );
  }
  // The fifth wrong try burns the code and fills that address's bucket.
  assert_eq!(
    launch_from(&router, "198.51.100.7:4000", ROOT, "BBBB-BBBB").await,
    401
  );
  let code = mint(&state, ROOT, 60).await;
  assert_eq!(
    launch_from(&router, "198.51.100.7:4000", ROOT, &code).await,
    429
  );
  assert_eq!(
    launch_from(&router, "203.0.113.9:4000", ROOT, &code).await,
    200
  );
  state.db.close().await;
}

#[tokio::test]
async fn every_failure_gets_the_same_reply() {
  let (_dir, state, router) = test_app().await;
  bootstrap_admin(&state, &router).await;
  let expired = mint(&state, ROOT, -1).await;
  let (_, expired_body, _) = launch(&router, ROOT, &expired).await;
  let (_, missing_body, _) = launch(&router, ROOT, "BBBB-BBBB").await;
  let (_, unknown_body, _) = launch(&router, "nobody@dopbase.com", "BBBB-BBBB").await;
  let (_, malformed_body, _) = launch(&router, "not an email", "BBBB-BBBB").await;
  assert_eq!(expired_body, missing_body);
  assert_eq!(expired_body, unknown_body);
  assert_eq!(expired_body, malformed_body);
  state.db.close().await;
}

#[tokio::test]
async fn accounts_that_cannot_sign_in_are_refused_when_the_code_is_used() {
  let (_dir, state, router) = test_app().await;
  bootstrap_admin(&state, &router).await;
  add_account(&state, "agent@dopbase.com", "rol_ai_agent", "active").await;
  add_account(&state, "gone@dopbase.com", "rol_member", "disabled").await;
  let sessions_before = browser_sessions(&state).await;
  for email in ["agent@dopbase.com", "gone@dopbase.com"] {
    let code = mint(&state, email, 60).await;
    let (status, body, _) = launch(&router, email, &code).await;
    assert_invalid(status, &body);
  }
  assert_eq!(browser_sessions(&state).await, sessions_before);
  assert_eq!(passcode_rows(&state).await, 0, "used codes are gone");
  state.db.close().await;
}

#[tokio::test]
async fn required_single_sign_on_still_lets_root_launch_but_nobody_else() {
  let (_dir, state, router) = test_app().await;
  bootstrap_admin(&state, &router).await;
  add_account(&state, "member@dopbase.com", "rol_member", "active").await;
  sqlx::query("INSERT INTO sso_providers(id,preset,display_name,issuer,client_id,client_secret_ciphertext,client_secret_nonce,enabled,require_sso,created_at,updated_at) VALUES('sso_x','generic','Okta','https://idp.example.com','client',x'00',x'00',1,1,'t','t')")
    .execute(state.db.pool())
    .await
    .unwrap();
  let member = mint(&state, "member@dopbase.com", 60).await;
  let (status, body, _) = launch(&router, "member@dopbase.com", &member).await;
  assert_invalid(status, &body);
  let root = mint(&state, ROOT, 60).await;
  let (status, body, _) = launch(&router, ROOT, &root).await;
  assert_eq!(status, 200, "{body}");
  state.db.close().await;
}

#[tokio::test]
async fn launching_in_a_signed_in_browser_ends_the_old_session() {
  let (_dir, state, router) = test_app().await;
  bootstrap_admin(&state, &router).await;
  let (old_cookie, _) = browser_login(&router).await;
  let code = mint(&state, ROOT, 60).await;
  let (status, body) = browser_call(
    &router,
    "/api/v1/auth/launch",
    &old_cookie,
    None,
    json!({"identity":ROOT,"passcode":code}),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  let old_token = old_cookie.trim_start_matches("dopbase_session=");
  let revoked: Option<String> =
    sqlx::query_scalar("SELECT revoked_at FROM sessions WHERE token_hash=?")
      .bind(app::services::token::hash(old_token))
      .fetch_one(state.db.pool())
      .await
      .unwrap();
  assert!(revoked.is_some(), "the old browser session is ended");
  let revoked_total: i64 =
    sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE revoked_at IS NOT NULL")
      .fetch_one(state.db.pool())
      .await
      .unwrap();
  assert_eq!(revoked_total, 1, "only that one");
  state.db.close().await;
}

#[tokio::test]
async fn launches_are_audited_and_the_chain_holds() {
  let (_dir, state, router) = test_app().await;
  bootstrap_admin(&state, &router).await;
  let code = mint(&state, ROOT, 60).await;
  launch(&router, ROOT, "BBBB-BBBB").await;
  let (status, _, _) = launch(&router, ROOT, &code).await;
  assert_eq!(status, 200);
  let events: Vec<(String, String, String)> = sqlx::query_as(
    "SELECT action,actor_type,metadata FROM audit_events WHERE metadata LIKE '%passcode%' ORDER BY seq",
  )
  .fetch_all(state.db.pool())
  .await
  .unwrap();
  assert_eq!(events.len(), 2, "{events:?}");
  assert_eq!(events[0].0, "login.failed");
  assert_eq!(events[0].1, "anonymous");
  assert!(events[0].2.contains("passcode_wrong"), "{events:?}");
  assert_eq!(events[1].0, "login.succeeded");
  assert!(events[1].2.contains(r#""method":"passcode""#), "{events:?}");
  let report = app::services::audit_chain::verify(state.db.pool())
    .await
    .unwrap();
  assert!(report.ok, "chain broke: {:?}", report.broken);
  state.db.close().await;
}
