use super::*;

pub(super) async fn browser_call(
  router: &Router,
  path: &str,
  cookie: &str,
  csrf: Option<&str>,
  body: Value,
) -> (u16, Value) {
  let mut request = Request::builder()
    .method("POST")
    .uri(path)
    .header(header::COOKIE, cookie)
    .header(header::CONTENT_TYPE, "application/json");
  if let Some(csrf) = csrf {
    request = request.header("x-dopbase-csrf", csrf);
  }
  let response = router
    .clone()
    .oneshot(request.body(Body::from(body.to_string())).unwrap())
    .await
    .unwrap();
  let status = response.status().as_u16();
  let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
  (status, serde_json::from_slice(&bytes).unwrap())
}

pub(super) async fn browser_login(router: &Router) -> (String, String) {
  let (status, body, headers) = call(
    router,
    "POST",
    "/api/v1/auth/login",
    None,
    Some(
      json!({"email":"admin@example.com","password":"correct-horse-123","sessionKind":"browser"}),
    ),
  )
  .await;
  assert_eq!(status, 200);
  (
    headers
      .get(header::SET_COOKIE)
      .unwrap()
      .to_str()
      .unwrap()
      .split(';')
      .next()
      .unwrap()
      .to_owned(),
    body["data"]["csrfToken"].as_str().unwrap().to_owned(),
  )
}

/// The plaintext of a personal token for a new AI agent account that reads
/// secret names on `project` and nothing more.
pub(super) async fn metadata_agent_token(
  router: &Router,
  cookie: &str,
  csrf: &str,
  name: &str,
  project: &str,
) -> String {
  let send = |method: &str, path: String, body: Value| {
    let request = Request::builder()
      .method(method)
      .uri(path)
      .header(header::COOKIE, cookie)
      .header("x-dopbase-csrf", csrf)
      .header(header::CONTENT_TYPE, "application/json")
      .body(Body::from(body.to_string()))
      .unwrap();
    async move {
      let response = router.clone().oneshot(request).await.unwrap();
      let status = response.status().as_u16();
      let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
      let body: Value = serde_json::from_slice(&bytes).unwrap();
      assert!(status < 300, "{body}");
      body
    }
  };
  let agent = send(
    "POST",
    "/api/v1/users".into(),
    json!({"email":name,"role":"ai_agent"}),
  )
  .await;
  let id = agent["data"]["id"].as_str().unwrap();
  send(
    "PUT",
    "/api/v1/grants".into(),
    json!({"principalType":"user","principalId":id,"scopeType":"project","scope":project,"level":"metadata"}),
  )
  .await;
  let token = send(
    "POST",
    format!("/api/v1/users/{id}/tokens"),
    json!({"name":"default","level":"metadata"}),
  )
  .await;
  token["data"]["plaintextToken"].as_str().unwrap().to_owned()
}

pub(super) async fn expire_confirmation(state: &app::state::AppState) {
  sqlx::query("UPDATE sessions SET recent_auth_at=? WHERE kind='browser'")
    .bind((chrono::Utc::now() - chrono::Duration::minutes(11)).to_rfc3339())
    .execute(state.db.pool())
    .await
    .unwrap();
}

#[tokio::test]
async fn import_requires_recent_browser_auth_for_every_mode_and_guess() {
  let (_dir, state, router, token, id) = admin_environment().await;
  let base = format!("/api/v1/environments/{id}/secrets");
  let (status, _, _) = call(
    &router,
    "PUT",
    &format!("{base}/APP_SECRETS"),
    Some(&token),
    Some(json!({"value":"stored-fixture-value"})),
  )
  .await;
  assert_eq!(status, 200);
  let (_, snapshot, _) = call(
    &router,
    "POST",
    &format!("{base}/export"),
    Some(&token),
    None,
  )
  .await;
  let revision = &snapshot["data"]["revision"];
  let (cookie, csrf) = browser_login(&router).await;
  expire_confirmation(&state).await;
  for mode in ["merge", "replace"] {
    for dry_run in [true, false] {
      for guess in ["stored-fixture-value", "wrong-fixture-value"] {
        let (status, body) = browser_call(&router, &format!("{base}/import"), &cookie, Some(&csrf),
          json!({"mode":mode,"dryRun":dry_run,"entries":[{"key":"APP_SECRETS","value":guess}],"expectedRevision":revision,"envLayout":"APP_SECRETS=\n"})).await;
        assert_eq!(status, 403, "{mode}, dryRun={dry_run}, body={body}");
        assert!(body["error"]["RECENT_AUTHENTICATION_REQUIRED"].is_string());
        assert!(body.get("data").is_none_or(Value::is_null));
      }
    }
  }
  let (_, after, _) = call(
    &router,
    "POST",
    &format!("{base}/export"),
    Some(&token),
    None,
  )
  .await;
  assert_eq!(snapshot["data"], after["data"]);
  let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_events WHERE action IN ('secret.imported','secret.import_previewed')").fetch_one(state.db.pool()).await.unwrap();
  assert_eq!(count, 0);
  let (status, _) = browser_call(
    &router,
    "/api/v1/auth/reauthenticate",
    &cookie,
    Some(&csrf),
    json!({"password":"wrong-password"}),
  )
  .await;
  assert_eq!(status, 401);
  let request = json!({"mode":"merge","dryRun":true,"entries":[{"key":"APP_SECRETS","value":"stored-fixture-value"}]});
  let (status, _) = browser_call(
    &router,
    &format!("{base}/import"),
    &cookie,
    Some(&csrf),
    request.clone(),
  )
  .await;
  assert_eq!(status, 403);
  let (status, _) = browser_call(
    &router,
    "/api/v1/auth/reauthenticate",
    &cookie,
    Some(&csrf),
    json!({"password":"correct-horse-123"}),
  )
  .await;
  assert_eq!(status, 200);
  for csrf_header in [None, Some("wrong-csrf")] {
    let (status, _) = browser_call(
      &router,
      &format!("{base}/import"),
      &cookie,
      csrf_header,
      request.clone(),
    )
    .await;
    assert_eq!(status, 403);
  }
  let (status, preview) = browser_call(
    &router,
    &format!("{base}/import"),
    &cookie,
    Some(&csrf),
    request,
  )
  .await;
  assert_eq!(status, 200);
  assert_eq!(preview["data"]["unchangedKeys"], json!(["APP_SECRETS"]));
  let apply = json!({"mode":"replace","dryRun":false,"entries":[{"key":"APP_SECRETS","value":"new-fixture-value"}],"expectedRevision":preview["data"]["revision"]});
  expire_confirmation(&state).await;
  assert_eq!(
    browser_call(
      &router,
      &format!("{base}/import"),
      &cookie,
      Some(&csrf),
      apply.clone()
    )
    .await
    .0,
    403
  );
  assert_eq!(
    browser_call(
      &router,
      "/api/v1/auth/reauthenticate",
      &cookie,
      Some(&csrf),
      json!({"password":"correct-horse-123"})
    )
    .await
    .0,
    200
  );
  assert_eq!(
    browser_call(
      &router,
      &format!("{base}/import"),
      &cookie,
      Some(&csrf),
      apply
    )
    .await
    .0,
    200
  );
  state.db.close().await;
}

#[tokio::test]
async fn import_preview_audits_metadata_and_fails_closed_when_audit_fails() {
  let (_dir, state, router, token, id) = admin_environment().await;
  let path = format!("/api/v1/environments/{id}/secrets/import");
  let request = json!({"mode":"merge","dryRun":true,"entries":[{"key":"APP_SECRETS","value":"private-fixture-value"}],"envLayout":"APP_SECRETS=\n"});
  assert_eq!(
    call(&router, "POST", &path, Some(&token), Some(request.clone()))
      .await
      .0,
    200
  );
  let (actor_type, environment_id, metadata): (String, String, String) = sqlx::query_as("SELECT actor_type,environment_id,metadata FROM audit_events WHERE action='secret.import_previewed'").fetch_one(state.db.pool()).await.unwrap();
  // The fixture signs in as the root user.
  assert_eq!(actor_type, "root");
  assert_eq!(environment_id, id);
  assert_eq!(
    serde_json::from_str::<Value>(&metadata).unwrap(),
    json!({"mode":"merge","entryCount":1})
  );
  let secret_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM secrets")
    .fetch_one(state.db.pool())
    .await
    .unwrap();
  assert_eq!(secret_count, 0);
  let layout_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM environment_env_layout")
    .fetch_one(state.db.pool())
    .await
    .unwrap();
  assert_eq!(layout_count, 0);
  sqlx::query("CREATE TRIGGER reject_preview_audit BEFORE INSERT ON audit_events WHEN NEW.action='secret.import_previewed' BEGIN SELECT RAISE(FAIL, 'audit unavailable'); END").execute(state.db.pool()).await.unwrap();
  let (status, body, _) = call(&router, "POST", &path, Some(&token), Some(request)).await;
  assert_eq!(status, 500);
  assert!(body.get("data").is_none_or(Value::is_null));
  assert!(!body.to_string().contains("private-fixture-value"));
  state.db.close().await;
}

#[tokio::test]
async fn import_rejects_nonhuman_credentials_and_invalid_sessions() {
  let (_dir, state, router, token, id) = admin_environment().await;
  let path = format!("/api/v1/environments/{id}/secrets/import");
  let (status, runner, _) = call(
    &router,
    "POST",
    &format!("/api/v1/environments/{id}/tokens"),
    Some(&token),
    Some(json!({"name":"test-runner","role":"runner"})),
  )
  .await;
  assert_eq!(status, 201);
  let (cookie, csrf) = browser_login(&router).await;
  let agent_token =
    metadata_agent_token(&router, &cookie, &csrf, "test-agent", "concurrency-test").await;
  for dry_run in [true, false] {
    let request =
      json!({"mode":"merge","dryRun":dry_run,"entries":[{"key":"APP_SECRETS","value":"fixture"}]});
    for credential in [None, Some("invalid-token")] {
      assert_eq!(
        call(&router, "POST", &path, credential, Some(request.clone()))
          .await
          .0,
        401
      );
    }
    for credential in [
      runner["data"]["plaintextToken"].as_str().unwrap(),
      agent_token.as_str(),
    ] {
      let (status, body, _) = call(
        &router,
        "POST",
        &path,
        Some(credential),
        Some(request.clone()),
      )
      .await;
      assert_eq!(status, 403);
      assert!(body["error"]["AUTHORIZATION_DENIED"].is_string());
    }
  }
  sqlx::query("UPDATE sessions SET revoked_at=? WHERE kind='browser'")
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(state.db.pool())
    .await
    .unwrap();
  assert_eq!(
    browser_call(
      &router,
      &path,
      &cookie,
      Some(&csrf),
      json!({"mode":"merge","dryRun":true,"entries":[]})
    )
    .await
    .0,
    401
  );
  state.db.close().await;
}

#[tokio::test]
async fn import_checks_confirmation_before_validation_and_decryption_for_all_human_roles() {
  let (_dir, state, router, token, id) = admin_environment().await;
  let base = format!("/api/v1/environments/{id}/secrets");
  assert_eq!(
    call(
      &router,
      "PUT",
      &format!("{base}/APP_SECRETS"),
      Some(&token),
      Some(json!({"value":"fixture"}))
    )
    .await
    .0,
    200
  );
  let (cookie, csrf) = browser_login(&router).await;
  // Broken fixture ciphertext would return 500 if import tried to decrypt it.
  sqlx::query("UPDATE secrets SET ciphertext=X'00' WHERE environment_id=?")
    .bind(&id)
    .execute(state.db.pool())
    .await
    .unwrap();
  expire_confirmation(&state).await;
  for role in ["root", "admin", "member"] {
    sqlx::query("UPDATE admins SET role=? WHERE email='admin@example.com'")
      .bind(role)
      .execute(state.db.pool())
      .await
      .unwrap();
    for key in ["APP_SECRETS", "INVALID KEY"] {
      let (status, body) = browser_call(
        &router,
        &format!("{base}/import"),
        &cookie,
        Some(&csrf),
        json!({"mode":"merge","dryRun":true,"entries":[{"key":key,"value":"fixture"}]}),
      )
      .await;
      assert_eq!(status, 403);
      assert!(body["error"]["RECENT_AUTHENTICATION_REQUIRED"].is_string());
    }
  }
  assert_eq!(
    browser_call(
      &router,
      "/api/v1/auth/reauthenticate",
      &cookie,
      Some(&csrf),
      json!({"password":"correct-horse-123"})
    )
    .await
    .0,
    200
  );
  assert_eq!(
    browser_call(
      &router,
      &format!("{base}/import"),
      &cookie,
      Some(&csrf),
      json!({"mode":"merge","dryRun":true,"entries":[{"key":"APP_SECRETS","value":"fixture"}]})
    )
    .await
    .0,
    500
  );
  state.db.close().await;
}
