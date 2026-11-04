use super::secrets_import::{browser_call, browser_login, expire_confirmation};
use super::*;

async fn start_device(router: &Router) -> Value {
  let (status, body, _) = call(
    router,
    "POST",
    "/api/v1/auth/cli/device",
    None,
    Some(json!({"hostname":" dev-box.local <b>"})),
  )
  .await;
  assert_eq!(status, 201, "{body}");
  body["data"].clone()
}

async fn poll(
  router: &Router,
  device_code: &str,
) -> (u16, Value) {
  let (status, body, _) = call(
    router,
    "POST",
    "/api/v1/auth/cli/token",
    None,
    Some(json!({"deviceCode":device_code})),
  )
  .await;
  (status, body)
}

/// Let the next poll through without tripping `slowDown`.
async fn age_last_poll(state: &app::state::AppState) {
  sqlx::query("UPDATE cli_device_requests SET last_polled_at=?")
    .bind((chrono::Utc::now() - chrono::Duration::seconds(6)).to_rfc3339())
    .execute(state.db.pool())
    .await
    .unwrap();
}

#[tokio::test]
async fn sign_in_methods_are_public() {
  let (_dir, _state, router) = test_app().await;
  let (status, body, _) = call(&router, "GET", "/api/v1/auth/methods", None, None).await;
  assert_eq!(status, 200);
  assert_eq!(
    body["data"],
    json!({"password":true,"passwordForm":true,"sso":null,"cliBrowser":true})
  );
}

#[tokio::test]
async fn approved_browser_login_issues_one_cli_session() {
  let (_dir, state, router) = test_app().await;
  bootstrap_admin(&state, &router).await;
  let device = start_device(&router).await;
  let device_code = device["deviceCode"].as_str().unwrap();
  let user_code = device["userCode"].as_str().unwrap();
  assert!(device_code.starts_with("dbd_"));
  assert_eq!(device["interval"], 5);
  assert_eq!(device["expiresIn"], 600);
  assert!(
    device["verificationUri"]
      .as_str()
      .unwrap()
      .ends_with("/cli/approve")
  );
  assert!(
    device["verificationUriComplete"]
      .as_str()
      .unwrap()
      .ends_with(&format!("/cli/approve?code={user_code}"))
  );

  let (status, body) = poll(&router, device_code).await;
  assert_eq!(status, 200);
  assert_eq!(body["data"]["status"], "pending");
  assert!(body["data"]["session"].is_null());

  // The approver may type the code in any case, with a space for the dash.
  let typed = user_code.to_lowercase().replace('-', " ");
  let (cookie, csrf) = browser_login(&router).await;
  let (status, details) = browser_call(
    &router,
    "/api/v1/auth/cli/inspect",
    &cookie,
    Some(&csrf),
    json!({"userCode":typed}),
  )
  .await;
  assert_eq!(status, 200, "{details}");
  assert_eq!(details["data"]["hostname"], "dev-box.localb");
  let (status, body) = browser_call(
    &router,
    "/api/v1/auth/cli/approve",
    &cookie,
    Some(&csrf),
    json!({"userCode":typed}),
  )
  .await;
  assert_eq!(status, 200, "{body}");

  let (status, body) = poll(&router, device_code).await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["status"], "approved");
  let session = &body["data"]["session"];
  assert_eq!(session["email"], "admin@example.com");
  assert_eq!(session["sessionKind"], "cli");
  assert!(session["csrfToken"].is_null());
  let token = session["token"].as_str().unwrap();
  let (status, current, _) = call(&router, "GET", "/api/v1/auth/session", Some(token), None).await;
  assert_eq!(status, 200);
  // A CLI session has no tabs to protect, so no CSRF token either.
  assert!(current["data"]["csrfToken"].is_null());
  assert_eq!(current["data"]["sessionKind"], "cli");

  let (status, body) = poll(&router, device_code).await;
  assert_eq!(status, 400);
  assert!(body["error"]["DEVICE_CODE_INVALID"].is_string());

  let actions: Vec<(String, String)> = sqlx::query_as(
    "SELECT action, json(metadata) FROM audit_events WHERE action IN ('cli_login.approved','login.succeeded') ORDER BY seq",
  )
  .fetch_all(state.db.pool())
  .await
  .unwrap();
  let approved = actions
    .iter()
    .position(|(action, _)| action == "cli_login.approved")
    .unwrap();
  let (action, metadata) = &actions[approved + 1];
  assert_eq!(action, "login.succeeded");
  assert!(metadata.contains("\"method\":\"browser\""), "{metadata}");
}

#[tokio::test]
async fn inspect_names_the_client_and_how_long_the_session_lasts() {
  let (_dir, state, router) = test_app().await;
  bootstrap_admin(&state, &router).await;
  let (cookie, csrf) = browser_login(&router).await;
  let long = "x".repeat(200);
  let cases = [
    (
      json!({"hostname":"dev-box","client":"  dopbase 0.2.0\ton   macOS \u{7}"}),
      json!("dopbase 0.2.0 on macOS"),
    ),
    (json!({"hostname":"dev-box"}), Value::Null),
    (json!({"client":null}), Value::Null),
    (json!({"client":" \u{1}\u{2}\n\u{1b} "}), Value::Null),
    (
      json!({"client":"dopbase\u{202e} 0.2.0"}),
      json!("dopbase 0.2.0"),
    ),
    (json!({"client":long}), json!("x".repeat(80))),
  ];
  for (request, client) in cases {
    let (status, device, _) = call(
      &router,
      "POST",
      "/api/v1/auth/cli/device",
      None,
      Some(request.clone()),
    )
    .await;
    assert_eq!(status, 201, "{device}");
    let (status, details) = browser_call(
      &router,
      "/api/v1/auth/cli/inspect",
      &cookie,
      Some(&csrf),
      json!({"userCode":device["data"]["userCode"]}),
    )
    .await;
    assert_eq!(status, 200, "{details}");
    assert_eq!(details["data"]["client"], client, "{request}");
    // CLI_SESSION_ABSOLUTE_DAYS, the limit `issue_session` gives a CLI
    // session.
    assert_eq!(details["data"]["sessionDays"], 90, "{request}");
  }
}

#[tokio::test]
async fn polling_faster_than_the_interval_slows_down() {
  let (_dir, state, router) = test_app().await;
  let device = start_device(&router).await;
  let device_code = device["deviceCode"].as_str().unwrap();
  let (_, body) = poll(&router, device_code).await;
  assert_eq!(body["data"]["status"], "pending");
  let (_, body) = poll(&router, device_code).await;
  assert_eq!(body["data"]["status"], "slowDown");
  age_last_poll(&state).await;
  let (_, body) = poll(&router, device_code).await;
  assert_eq!(body["data"]["status"], "pending");
}

#[tokio::test]
async fn denied_cancelled_expired_and_unknown_requests_end_distinctly() {
  let (_dir, state, router) = test_app().await;
  bootstrap_admin(&state, &router).await;
  let (cookie, csrf) = browser_login(&router).await;

  // Denying needs no fresh password: it can only stop a login.
  expire_confirmation(&state).await;
  let denied = start_device(&router).await;
  let (status, _) = browser_call(
    &router,
    "/api/v1/auth/cli/deny",
    &cookie,
    Some(&csrf),
    json!({"userCode":denied["userCode"]}),
  )
  .await;
  assert_eq!(status, 200);
  let (status, body) = poll(&router, denied["deviceCode"].as_str().unwrap()).await;
  assert_eq!(status, 403);
  assert!(body["error"]["DEVICE_ACCESS_DENIED"].is_string());

  let cancelled = start_device(&router).await;
  let (status, _, _) = call(
    &router,
    "POST",
    "/api/v1/auth/cli/cancel",
    None,
    Some(json!({"deviceCode":cancelled["deviceCode"]})),
  )
  .await;
  assert_eq!(status, 200);
  let (status, body) = poll(&router, cancelled["deviceCode"].as_str().unwrap()).await;
  assert_eq!(status, 400);
  assert!(body["error"]["DEVICE_CODE_INVALID"].is_string());

  let expired = start_device(&router).await;
  sqlx::query("UPDATE cli_device_requests SET expires_at=? WHERE status='pending'")
    .bind((chrono::Utc::now() - chrono::Duration::seconds(1)).to_rfc3339())
    .execute(state.db.pool())
    .await
    .unwrap();
  let (status, body) = poll(&router, expired["deviceCode"].as_str().unwrap()).await;
  assert_eq!(status, 410);
  assert!(body["error"]["DEVICE_CODE_EXPIRED"].is_string());
  let (status, _) = browser_call(
    &router,
    "/api/v1/auth/cli/deny",
    &cookie,
    Some(&csrf),
    json!({"userCode":expired["userCode"]}),
  )
  .await;
  assert_eq!(status, 404);

  let (status, _) = poll(&router, "dbd_unknown").await;
  assert_eq!(status, 400);
  // Cancelling something that does not exist says nothing about it.
  let (status, _, _) = call(
    &router,
    "POST",
    "/api/v1/auth/cli/cancel",
    None,
    Some(json!({"deviceCode":"dbd_unknown"})),
  )
  .await;
  assert_eq!(status, 200);
}

#[tokio::test]
async fn only_a_recently_confirmed_browser_session_can_approve() {
  let (_dir, state, router) = test_app().await;
  let cli_token = bootstrap_admin(&state, &router).await;
  let device = start_device(&router).await;
  let user_code = device["userCode"].as_str().unwrap();

  let (status, body, _) = call(
    &router,
    "POST",
    "/api/v1/auth/cli/approve",
    Some(&cli_token),
    Some(json!({"userCode":user_code})),
  )
  .await;
  assert_eq!(status, 403, "{body}");
  assert!(body["error"]["AUTHORIZATION_DENIED"].is_string());

  let (cookie, csrf) = browser_login(&router).await;
  let (status, _) = browser_call(
    &router,
    "/api/v1/auth/cli/approve",
    &cookie,
    None,
    json!({"userCode":user_code}),
  )
  .await;
  assert_eq!(status, 403);

  expire_confirmation(&state).await;
  let (status, body) = browser_call(
    &router,
    "/api/v1/auth/cli/approve",
    &cookie,
    Some(&csrf),
    json!({"userCode":user_code}),
  )
  .await;
  assert_eq!(status, 403);
  assert!(body["error"]["RECENT_AUTHENTICATION_REQUIRED"].is_string());

  let (_, body) = poll(&router, device["deviceCode"].as_str().unwrap()).await;
  assert_eq!(body["data"]["status"], "pending");
}

#[tokio::test]
async fn guessing_user_codes_is_rate_limited_per_session() {
  let (_dir, state, router) = test_app().await;
  bootstrap_admin(&state, &router).await;
  let device = start_device(&router).await;
  let (cookie, csrf) = browser_login(&router).await;
  for guess in [
    "BCDF-GHJK",
    "not-a-code",
    "BCDF-GHJL",
    "BCDF-GHJM",
    "BCDF-GHJN",
  ] {
    let (status, _) = browser_call(
      &router,
      "/api/v1/auth/cli/inspect",
      &cookie,
      Some(&csrf),
      json!({"userCode":guess}),
    )
    .await;
    assert_eq!(status, 404, "{guess}");
  }
  // Once limited, even the right code is refused.
  let (status, body) = browser_call(
    &router,
    "/api/v1/auth/cli/approve",
    &cookie,
    Some(&csrf),
    json!({"userCode":device["userCode"]}),
  )
  .await;
  assert_eq!(status, 429);
  assert!(body["error"]["RATE_LIMITED"].is_string());
}

#[tokio::test]
async fn a_confirmation_refreshes_the_cli_session_that_asked_for_it() {
  let (_dir, state, router) = test_app().await;
  let cli_token = bootstrap_admin(&state, &router).await;
  let (status, _, _) = call(
    &router,
    "POST",
    "/api/v1/auth/cli/device",
    None,
    Some(json!({"purpose":"reauth"})),
  )
  .await;
  assert_eq!(status, 403, "a confirmation needs a CLI session");

  sqlx::query("UPDATE sessions SET recent_auth_at=? WHERE kind='cli'")
    .bind((chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339())
    .execute(state.db.pool())
    .await
    .unwrap();
  let (status, device, _) = call(
    &router,
    "POST",
    "/api/v1/auth/cli/device",
    Some(&cli_token),
    Some(json!({"purpose":"reauth","hostname":"dev-box"})),
  )
  .await;
  assert_eq!(status, 201, "{device}");
  let device = &device["data"];

  // Another account cannot confirm someone else's terminal.
  let (cookie, csrf) = browser_login(&router).await;
  let (status, created) = browser_call(
    &router,
    "/api/v1/users",
    &cookie,
    Some(&csrf),
    json!({"email":"other@example.com","password":"correct-horse-456","role":"member"}),
  )
  .await;
  assert_eq!(status, 201, "{created}");
  let (_, other, headers) = call(
    &router,
    "POST",
    "/api/v1/auth/login",
    None,
    Some(
      json!({"email":"other@example.com","password":"correct-horse-456","sessionKind":"browser"}),
    ),
  )
  .await;
  let other_cookie = headers[header::SET_COOKIE]
    .to_str()
    .unwrap()
    .split(';')
    .next()
    .unwrap()
    .to_owned();
  let other_csrf = other["data"]["csrfToken"].as_str().unwrap();
  let (status, _) = browser_call(
    &router,
    "/api/v1/auth/cli/approve",
    &other_cookie,
    Some(other_csrf),
    json!({"userCode":device["userCode"]}),
  )
  .await;
  assert_eq!(status, 404);

  let (status, details) = browser_call(
    &router,
    "/api/v1/auth/cli/inspect",
    &cookie,
    Some(&csrf),
    json!({"userCode":device["userCode"]}),
  )
  .await;
  assert_eq!(status, 200);
  assert_eq!(details["data"]["purpose"], "reauth");
  let (status, _) = browser_call(
    &router,
    "/api/v1/auth/cli/approve",
    &cookie,
    Some(&csrf),
    json!({"userCode":device["userCode"]}),
  )
  .await;
  assert_eq!(status, 200);
  let (status, body) = poll(&router, device["deviceCode"].as_str().unwrap()).await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["status"], "approved");
  assert!(
    body["data"]["session"].is_null(),
    "no new session is issued"
  );

  let (_, session, _) = call(
    &router,
    "GET",
    "/api/v1/auth/session",
    Some(&cli_token),
    None,
  )
  .await;
  assert_eq!(session["data"]["recentAuthentication"], true);
  let cli_sessions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE kind='cli'")
    .fetch_one(state.db.pool())
    .await
    .unwrap();
  assert_eq!(cli_sessions, 1);
}

#[tokio::test]
async fn an_existing_browser_session_approves_from_a_new_tab() {
  let (_dir, state, router) = test_app().await;
  bootstrap_admin(&state, &router).await;
  // Signed in before the request existed. The approval link opens in a
  // new tab, which has the cookie but not the CSRF token from sign-in.
  let (cookie, csrf) = browser_login(&router).await;
  sqlx::query("UPDATE sessions SET created_at=? WHERE kind='browser'")
    .bind((chrono::Utc::now() - chrono::Duration::hours(2)).to_rfc3339())
    .execute(state.db.pool())
    .await
    .unwrap();
  let device = start_device(&router).await;
  let code = json!({"userCode":device["userCode"]});
  let (status, body) = browser_call(
    &router,
    "/api/v1/auth/cli/inspect",
    &cookie,
    None,
    code.clone(),
  )
  .await;
  assert_eq!(status, 403, "{body}");
  assert!(body["error"]["AUTHORIZATION_DENIED"].is_string());

  // The session hands the tab a token of its own.
  let (status, current) = get_with_cookie(&router, "/api/v1/auth/session", &cookie).await;
  assert_eq!(status, 200, "{current}");
  let tab = current["data"]["csrfToken"].as_str().unwrap().to_owned();
  assert!(tab.starts_with("csrf_"));
  assert_ne!(tab, csrf);
  let (_, again) = get_with_cookie(&router, "/api/v1/auth/session", &cookie).await;
  assert_eq!(again["data"]["csrfToken"], tab);

  let (status, body) = browser_call(
    &router,
    "/api/v1/auth/cli/inspect",
    &cookie,
    Some(&tab),
    code.clone(),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["purpose"], "login");
  let (status, body) = browser_call(
    &router,
    "/api/v1/auth/cli/approve",
    &cookie,
    Some(&tab),
    code,
  )
  .await;
  assert_eq!(status, 200, "{body}");
  let (_, body) = poll(&router, device["deviceCode"].as_str().unwrap()).await;
  assert_eq!(body["data"]["status"], "approved");

  // The sign-in tab's token still works, and a tab token works only with
  // its own session.
  let next = start_device(&router).await;
  let (status, body) = browser_call(
    &router,
    "/api/v1/auth/cli/inspect",
    &cookie,
    Some(&csrf),
    json!({"userCode":next["userCode"]}),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  let (other_cookie, _) = browser_login(&router).await;
  let (status, body) = browser_call(
    &router,
    "/api/v1/auth/cli/inspect",
    &other_cookie,
    Some(&tab),
    json!({"userCode":next["userCode"]}),
  )
  .await;
  assert_eq!(status, 403, "{body}");
  assert!(body["error"]["AUTHORIZATION_DENIED"].is_string());
}

async fn get_with_cookie(
  router: &Router,
  path: &str,
  cookie: &str,
) -> (u16, Value) {
  let response = router
    .clone()
    .oneshot(
      Request::builder()
        .uri(path)
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();
  let status = response.status().as_u16();
  let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
  (status, serde_json::from_slice(&bytes).unwrap())
}
