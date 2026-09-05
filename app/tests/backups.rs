use app::{config::ServerConfig, server};
use axum::{
  Router,
  body::{Body, to_bytes},
  http::{Request, header},
};
use serde_json::{Value, json};
use tempfile::TempDir;
use tower::ServiceExt;

async fn test_app() -> (TempDir, app::state::AppState, Router) {
  let directory = TempDir::new().unwrap();
  let database = directory.path().join("dopbase.db");
  let config = ServerConfig {
    data_dir: directory.path().to_path_buf(),
    database_url: format!("sqlite://{}", database.display()),
    master_key: app::config::MasterKeyConfig {
      provider: "file".into(),
      path: directory.path().join("master.key"),
    },
    docs_enabled: true,
    ..ServerConfig::default()
  };
  let state = server::build_state(config).await.unwrap();
  let router = server::router(state.clone());
  (directory, state, router)
}

async fn call(
  router: &Router,
  method: &str,
  path: &str,
  token: Option<&str>,
  body: Option<Value>,
) -> (u16, Value, axum::http::HeaderMap) {
  let mut builder = Request::builder().method(method).uri(path);
  if let Some(token) = token {
    builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
  }
  if body.is_some() {
    builder = builder.header(header::CONTENT_TYPE, "application/json");
  }
  let request = builder
    .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
    .unwrap();
  let response = router.clone().oneshot(request).await.unwrap();
  let status = response.status().as_u16();
  let headers = response.headers().clone();
  let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
  let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
  (status, json, headers)
}

async fn call_raw(
  router: &Router,
  method: &str,
  path: &str,
  token: Option<&str>,
) -> (u16, Vec<u8>, axum::http::HeaderMap) {
  let mut builder = Request::builder().method(method).uri(path);
  if let Some(token) = token {
    builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
  }
  let request = builder.body(Body::empty()).unwrap();
  let response = router.clone().oneshot(request).await.unwrap();
  let status = response.status().as_u16();
  let headers = response.headers().clone();
  let bytes = to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap()
    .to_vec();
  (status, bytes, headers)
}

async fn admin_setup() -> (TempDir, app::state::AppState, Router, String) {
  let (directory, state, router) = test_app().await;
  let setup = state.setup.read().await.token.clone().unwrap();
  let (status, _, _) = call(
    &router,
    "POST",
    "/api/v1/bootstrap/admin",
    None,
    Some(
      json!({"setupToken": setup, "email": "admin@example.com", "password": "correct-horse-123"}),
    ),
  )
  .await;
  assert_eq!(status, 201);
  let (_, login, _) = call(
    &router,
    "POST",
    "/api/v1/auth/login",
    None,
    Some(
      json!({"email": "admin@example.com", "password": "correct-horse-123", "sessionKind": "cli"}),
    ),
  )
  .await;
  let token = login["data"]["token"].as_str().unwrap().to_owned();
  (directory, state, router, token)
}

#[tokio::test]
async fn backup_and_restore_lifecycle() {
  let (_dir, state, router, token) = admin_setup().await;

  // 1. Create a project and environment with secrets
  let (status, project, _) = call(
    &router,
    "POST",
    "/api/v1/projects",
    Some(&token),
    Some(json!({"name": "backup-test-project"})),
  )
  .await;
  assert_eq!(status, 201);
  let project_id = project["data"]["id"].as_str().unwrap();

  let (status, env, _) = call(
    &router,
    "POST",
    &format!("/api/v1/projects/{project_id}/environments"),
    Some(&token),
    Some(json!({"name": "staging"})),
  )
  .await;
  assert_eq!(status, 201);
  let env_id = env["data"]["id"].as_str().unwrap();

  let (status, _, _) = call(
    &router,
    "PUT",
    &format!("/api/v1/environments/{env_id}/secrets/API_KEY"),
    Some(&token),
    Some(json!({"value": "super-secret-key-123"})),
  )
  .await;
  assert_eq!(status, 200);

  // 2. Initialize new backup with custom name
  let (status, create_res, _) = call(
    &router,
    "POST",
    "/api/v1/backups",
    Some(&token),
    Some(json!({"name": "snapshot_v1"})),
  )
  .await;
  assert_eq!(status, 201);
  assert_eq!(create_res["data"]["key"], "snapshot_v1.dop");
  assert!(create_res["data"]["size"].as_u64().unwrap() > 0);

  // 3. List backups
  let (status, list_res, _) = call(&router, "GET", "/api/v1/backups", Some(&token), None).await;
  assert_eq!(status, 200);
  let list = list_res["data"].as_array().unwrap();
  assert_eq!(list.len(), 1);
  assert_eq!(list[0]["key"], "snapshot_v1.dop");

  // 4. Download backup
  let (status, raw_bytes, headers) = call_raw(
    &router,
    "GET",
    "/api/v1/backups/snapshot_v1.dop",
    Some(&token),
  )
  .await;
  assert_eq!(status, 200);
  assert_eq!(
    headers.get(header::CONTENT_DISPOSITION).unwrap(),
    "attachment; filename=\"snapshot_v1.dop\""
  );
  assert!(!raw_bytes.is_empty());
  assert!(raw_bytes.starts_with(b"DOPBASE_BK1\0"));

  // 5. Test tamper resistance
  let mut tampered = raw_bytes.clone();
  let last = tampered.len() - 1;
  tampered[last] ^= 0xFF;
  // Decrypting tampered should fail
  assert!(state.crypto.decrypt_backup(&tampered).is_err());

  // 6. Modify secret and add a new secret after backup was taken
  let (status, _, _) = call(
    &router,
    "PUT",
    &format!("/api/v1/environments/{env_id}/secrets/API_KEY"),
    Some(&token),
    Some(json!({"value": "MODIFIED_KEY_VALUE"})),
  )
  .await;
  assert_eq!(status, 200);

  let (status, _, _) = call(
    &router,
    "PUT",
    &format!("/api/v1/environments/{env_id}/secrets/NEW_SECRET"),
    Some(&token),
    Some(json!({"value": "temporary"})),
  )
  .await;
  assert_eq!(status, 200);

  // 7. Restore from backup
  let (status, restore_res, _) = call(
    &router,
    "POST",
    "/api/v1/backups/snapshot_v1.dop/restore",
    Some(&token),
    None,
  )
  .await;
  assert_eq!(status, 200);
  assert_eq!(restore_res["data"]["restored"], true);

  // 8. Verify secrets are restored to their original snapshot state!
  let (status, reveal_res, _) = call(
    &router,
    "POST",
    &format!("/api/v1/environments/{env_id}/secrets/API_KEY/reveal"),
    Some(&token),
    None,
  )
  .await;
  assert_eq!(status, 200);
  assert_eq!(reveal_res["data"]["value"], "super-secret-key-123");

  // Verify NEW_SECRET is gone (restored to point-in-time)
  let (status, _, _) = call(
    &router,
    "GET",
    &format!("/api/v1/environments/{env_id}/secrets/NEW_SECRET"),
    Some(&token),
    None,
  )
  .await;
  assert_eq!(status, 404);

  // 9. Delete backup
  let (status, delete_res, _) = call(
    &router,
    "DELETE",
    "/api/v1/backups/snapshot_v1.dop",
    Some(&token),
    None,
  )
  .await;
  assert_eq!(status, 200);
  assert_eq!(delete_res["data"]["deleted"], true);

  // 10. Verify list is now empty
  let (status, empty_res, _) = call(&router, "GET", "/api/v1/backups", Some(&token), None).await;
  assert_eq!(status, 200);
  assert_eq!(empty_res["data"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn backup_upload_and_tamper_rejection() {
  let (_dir, _state, router, token) = admin_setup().await;

  // 1. Create a backup
  let (status, _, _) = call(
    &router,
    "POST",
    "/api/v1/backups",
    Some(&token),
    Some(json!({"name": "upload_src"})),
  )
  .await;
  assert_eq!(status, 201);

  // 2. Download it
  let (status, raw_bytes, _) = call_raw(
    &router,
    "GET",
    "/api/v1/backups/upload_src.dop",
    Some(&token),
  )
  .await;
  assert_eq!(status, 200);

  // 3. Upload valid backup with a new filename
  let boundary = "---------------------------974767299852498929531610575";
  let mut body = Vec::new();
  body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
  body.extend_from_slice(
    b"Content-Disposition: form-data; name=\"file\"; filename=\"uploaded_copy.dop\"\r\n",
  );
  body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
  body.extend_from_slice(&raw_bytes);
  body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

  let req = Request::builder()
    .method("POST")
    .uri("/api/v1/backups/upload")
    .header(header::AUTHORIZATION, format!("Bearer {token}"))
    .header(
      header::CONTENT_TYPE,
      format!("multipart/form-data; boundary={boundary}"),
    )
    .body(Body::from(body))
    .unwrap();

  let res = router.clone().oneshot(req).await.unwrap();
  assert_eq!(res.status().as_u16(), 201);

  // Verify it appears in list
  let (status, list_res, _) = call(&router, "GET", "/api/v1/backups", Some(&token), None).await;
  assert_eq!(status, 200);
  let keys: Vec<_> = list_res["data"]
    .as_array()
    .unwrap()
    .iter()
    .map(|item| item["key"].as_str().unwrap())
    .collect();
  assert!(keys.contains(&"uploaded_copy.dop"));

  // 4. Upload corrupted/tampered bytes -> should return 400 Bad Request
  let mut corrupted = raw_bytes.clone();
  corrupted[15] ^= 0xAA;
  let mut body_corrupt = Vec::new();
  body_corrupt.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
  body_corrupt.extend_from_slice(
    b"Content-Disposition: form-data; name=\"file\"; filename=\"corrupt.dop\"\r\n",
  );
  body_corrupt.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
  body_corrupt.extend_from_slice(&corrupted);
  body_corrupt.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

  let req_corrupt = Request::builder()
    .method("POST")
    .uri("/api/v1/backups/upload")
    .header(header::AUTHORIZATION, format!("Bearer {token}"))
    .header(
      header::CONTENT_TYPE,
      format!("multipart/form-data; boundary={boundary}"),
    )
    .body(Body::from(body_corrupt))
    .unwrap();

  let res_corrupt = router.clone().oneshot(req_corrupt).await.unwrap();
  assert_eq!(res_corrupt.status().as_u16(), 400);
}

#[tokio::test]
async fn bootstrap_restore_lifecycle() {
  // 1. Create source server with admin and a secret
  let (dir1, _state1, router1, token1) = admin_setup().await;
  let (_, project_res, _) = call(
    &router1,
    "POST",
    "/api/v1/projects",
    Some(&token1),
    Some(json!({ "name": "bootstrapped-app" })),
  )
  .await;
  let project_id = project_res["data"]["id"].as_str().unwrap();

  let (_, env_res, _) = call(
    &router1,
    "POST",
    &format!("/api/v1/projects/{project_id}/environments"),
    Some(&token1),
    Some(json!({ "name": "production" })),
  )
  .await;
  let env_id = env_res["data"]["id"].as_str().unwrap();

  let _ = call(
    &router1,
    "POST",
    &format!("/api/v1/environments/{env_id}/secrets"),
    Some(&token1),
    Some(json!({ "key": "DATABASE_URL", "value": "postgres://prod/db" })),
  )
  .await;

  // Create backup and download raw bytes
  let (_, backup_res, _) = call(
    &router1,
    "POST",
    "/api/v1/backups",
    Some(&token1),
    Some(json!({ "name": "initial_seed" })),
  )
  .await;
  let key = backup_res["data"]["key"].as_str().unwrap();
  let (status, raw_bytes, _) = call_raw(
    &router1,
    "GET",
    &format!("/api/v1/backups/{key}"),
    Some(&token1),
  )
  .await;
  assert_eq!(status, 200);

  // 2. Create brand-new uninitialized server sharing the same master key
  let dir2 = TempDir::new().unwrap();
  let db2 = dir2.path().join("dopbase.db");
  let key_path = dir1.path().join("master.key"); // use same master key
  let config2 = ServerConfig {
    data_dir: dir2.path().to_path_buf(),
    database_url: format!("sqlite://{}", db2.display()),
    master_key: app::config::MasterKeyConfig {
      provider: "file".into(),
      path: key_path,
    },
    docs_enabled: true,
    ..ServerConfig::default()
  };
  let state2 = server::build_state(config2).await.unwrap();
  let router2 = server::router(state2.clone());
  let setup_token2 = state2.setup.read().await.token.clone().unwrap();

  // Confirm server2 is in setupRequired state
  let (status, status_res, _) = call(&router2, "GET", "/api/v1/bootstrap/status", None, None).await;
  assert_eq!(status, 200);
  assert_eq!(status_res["data"]["state"], "setupRequired");

  // 3. Post backup to /api/v1/bootstrap/restore
  let boundary = "---------------------------974767299852498929531610575";
  let mut body = Vec::new();
  body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
  body.extend_from_slice(
    b"Content-Disposition: form-data; name=\"file\"; filename=\"initial_seed.dop\"\r\n",
  );
  body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
  body.extend_from_slice(&raw_bytes);
  body.extend_from_slice(format!("\r\n--{boundary}\r\n").as_bytes());
  body.extend_from_slice(b"Content-Disposition: form-data; name=\"setup_token\"\r\n\r\n");
  body.extend_from_slice(setup_token2.as_bytes());
  body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

  let req = Request::builder()
    .method("POST")
    .uri("/api/v1/bootstrap/restore")
    .header(
      header::CONTENT_TYPE,
      format!("multipart/form-data; boundary={boundary}"),
    )
    .body(Body::from(body))
    .unwrap();

  let res = router2.clone().oneshot(req).await.unwrap();
  assert_eq!(res.status().as_u16(), 200);

  // Confirm server2 is now ready
  let (status, status_res2, _) =
    call(&router2, "GET", "/api/v1/bootstrap/status", None, None).await;
  assert_eq!(status, 200);
  assert_eq!(status_res2["data"]["state"], "ready");

  // Sign in on server2 using the admin account restored from backup!
  let (status, login, _) = call(
    &router2,
    "POST",
    "/api/v1/auth/login",
    None,
    Some(
      json!({"email": "admin@example.com", "password": "correct-horse-123", "sessionKind": "cli"}),
    ),
  )
  .await;
  assert_eq!(status, 200);
  let token2 = login["data"]["token"].as_str().unwrap();

  // Verify restored project and secret
  let (status, projects_res, _) =
    call(&router2, "GET", "/api/v1/projects", Some(token2), None).await;
  assert_eq!(status, 200);
  let projects = projects_res["data"].as_array().unwrap();
  assert_eq!(projects.len(), 1);
  assert_eq!(projects[0]["name"], "bootstrapped-app");

  // 4. Attempting restore again on initialized server returns 409 Conflict
  let mut body_again = Vec::new();
  body_again.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
  body_again.extend_from_slice(
    b"Content-Disposition: form-data; name=\"file\"; filename=\"again.dop\"\r\n",
  );
  body_again.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
  body_again.extend_from_slice(&raw_bytes);
  body_again.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

  let req_again = Request::builder()
    .method("POST")
    .uri("/api/v1/bootstrap/restore")
    .header(
      header::CONTENT_TYPE,
      format!("multipart/form-data; boundary={boundary}"),
    )
    .body(Body::from(body_again))
    .unwrap();

  let res_again = router2.clone().oneshot(req_again).await.unwrap();
  assert_eq!(res_again.status().as_u16(), 409);
}

#[tokio::test]
async fn bootstrap_restore_with_different_master_key() {
  let (dir1, _state1, router1, token1) = admin_setup().await;

  // 1. Create secret on Server 1
  let (status, project, _) = call(
    &router1,
    "POST",
    "/api/v1/projects",
    Some(&token1),
    Some(json!({"name": "cross-server-app"})),
  )
  .await;
  assert_eq!(status, 201);
  let project_id = project["data"]["id"].as_str().unwrap();

  let (status, env, _) = call(
    &router1,
    "POST",
    &format!("/api/v1/projects/{project_id}/environments"),
    Some(&token1),
    Some(json!({"name": "production"})),
  )
  .await;
  assert_eq!(status, 201);
  let env_id = env["data"]["id"].as_str().unwrap();

  let (status, _, _) = call(
    &router1,
    "PUT",
    &format!("/api/v1/environments/{env_id}/secrets/DATABASE_URL"),
    Some(&token1),
    Some(json!({"value": "postgres://prod:secret@db.internal:5432/main"})),
  )
  .await;
  assert_eq!(status, 200);

  // Download Server 1 backup
  let (status, backup_item, _) = call(
    &router1,
    "POST",
    "/api/v1/backups",
    Some(&token1),
    Some(json!({"name": "migration_snapshot"})),
  )
  .await;
  assert_eq!(status, 201);
  let backup_key = backup_item["data"]["key"].as_str().unwrap();

  let (status, raw_bytes, _) = call_raw(
    &router1,
    "GET",
    &format!("/api/v1/backups/{backup_key}"),
    Some(&token1),
  )
  .await;
  assert_eq!(status, 200);

  // Read Server 1 master key
  let server1_key_bytes = std::fs::read(dir1.path().join("master.key")).unwrap();

  // 2. Initialize fresh Server 2 with its OWN separate master key
  let dir2 = TempDir::new().unwrap();
  let db2 = dir2.path().join("dopbase.db");
  let server2_key_path = dir2.path().join("master.key");
  let config2 = ServerConfig {
    data_dir: dir2.path().to_path_buf(),
    database_url: format!("sqlite://{}", db2.display()),
    master_key: app::config::MasterKeyConfig {
      provider: "file".into(),
      path: server2_key_path.clone(),
    },
    docs_enabled: true,
    ..ServerConfig::default()
  };
  let state2 = server::build_state(config2).await.unwrap();
  let router2 = server::router(state2.clone());
  let setup_token2 = state2.setup.read().await.token.clone().unwrap();

  let server2_original_key = std::fs::read(&server2_key_path).unwrap();
  assert_ne!(server2_original_key, server1_key_bytes);

  // Confirm server2 is in setupRequired state
  let (status, status_res, _) = call(&router2, "GET", "/api/v1/bootstrap/status", None, None).await;
  assert_eq!(status, 200);
  assert_eq!(status_res["data"]["state"], "setupRequired");

  // Attempting to restore without master key fails with 400 BACKUP_KEY_REQUIRED
  let boundary = "---------------------------123456789012345678901234567";
  let mut fail_body = Vec::new();
  fail_body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
  fail_body.extend_from_slice(
    b"Content-Disposition: form-data; name=\"file\"; filename=\"migration.dop\"\r\n",
  );
  fail_body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
  fail_body.extend_from_slice(&raw_bytes);
  fail_body.extend_from_slice(format!("\r\n--{boundary}\r\n").as_bytes());
  fail_body.extend_from_slice(b"Content-Disposition: form-data; name=\"setup_token\"\r\n\r\n");
  fail_body.extend_from_slice(setup_token2.as_bytes());
  fail_body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

  let req_fail = Request::builder()
    .method("POST")
    .uri("/api/v1/bootstrap/restore")
    .header(
      header::CONTENT_TYPE,
      format!("multipart/form-data; boundary={boundary}"),
    )
    .body(Body::from(fail_body))
    .unwrap();
  let res_fail = router2.clone().oneshot(req_fail).await.unwrap();
  assert_eq!(res_fail.status().as_u16(), 400);

  // Now restore with both backup file AND server1's master key
  let mut restore_body = Vec::new();
  restore_body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
  restore_body.extend_from_slice(
    b"Content-Disposition: form-data; name=\"file\"; filename=\"migration.dop\"\r\n",
  );
  restore_body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
  restore_body.extend_from_slice(&raw_bytes);
  restore_body.extend_from_slice(format!("\r\n--{boundary}\r\n").as_bytes());
  restore_body.extend_from_slice(
    b"Content-Disposition: form-data; name=\"master_key\"; filename=\"master.key\"\r\n",
  );
  restore_body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
  restore_body.extend_from_slice(&server1_key_bytes);
  restore_body.extend_from_slice(format!("\r\n--{boundary}\r\n").as_bytes());
  restore_body.extend_from_slice(b"Content-Disposition: form-data; name=\"setup_token\"\r\n\r\n");
  restore_body.extend_from_slice(setup_token2.as_bytes());
  restore_body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

  let req_success = Request::builder()
    .method("POST")
    .uri("/api/v1/bootstrap/restore")
    .header(
      header::CONTENT_TYPE,
      format!("multipart/form-data; boundary={boundary}"),
    )
    .body(Body::from(restore_body))
    .unwrap();
  let res_success = router2.clone().oneshot(req_success).await.unwrap();
  assert_eq!(res_success.status().as_u16(), 200);

  // Confirm server2 master.key file on disk was NOT replaced with server1's key! It keeps its own key!
  let server2_updated_key = std::fs::read(&server2_key_path).unwrap();
  assert_eq!(server2_updated_key, server2_original_key);
  assert_ne!(server2_updated_key, server1_key_bytes);

  // Sign in on server2 using restored admin account
  let (status, login, _) = call(
    &router2,
    "POST",
    "/api/v1/auth/login",
    None,
    Some(
      json!({"email": "admin@example.com", "password": "correct-horse-123", "sessionKind": "cli"}),
    ),
  )
  .await;
  assert_eq!(status, 200);
  let token2 = login["data"]["token"].as_str().unwrap();

  // Decrypt secret on server2 using server2's own re-keyed master key!
  let (status, secret_res, _) = call(
    &router2,
    "POST",
    &format!("/api/v1/environments/{env_id}/secrets/DATABASE_URL/reveal"),
    Some(token2),
    None,
  )
  .await;
  assert_eq!(status, 200);
  assert_eq!(
    secret_res["data"]["value"],
    "postgres://prod:secret@db.internal:5432/main"
  );
}

#[tokio::test]
async fn test_download_master_key_cookie_session_without_csrf() {
  let (_dir, _state, router, _) = admin_setup().await;

  // Login as browser session
  let (status, _, headers) = call(
    &router,
    "POST",
    "/api/v1/auth/login",
    None,
    Some(json!({"email": "admin@example.com", "password": "correct-horse-123", "sessionKind": "browser"})),
  )
  .await;
  assert_eq!(status, 200);

  let cookie = headers
    .get(header::SET_COOKIE)
    .and_then(|v| v.to_str().ok())
    .unwrap()
    .to_string();

  // Browser download of master key should NOT require X-Dopbase-CSRF header on GET
  let req = Request::builder()
    .method("GET")
    .uri("/api/v1/backups/master-key")
    .header(header::COOKIE, &cookie)
    .body(Body::empty())
    .unwrap();
  let res = router.clone().oneshot(req).await.unwrap();
  assert_eq!(res.status().as_u16(), 200);

  let body = axum::body::to_bytes(res.into_body(), 1024 * 1024)
    .await
    .unwrap();
  assert_eq!(body.len(), 32);
}

#[tokio::test]
async fn test_dashboard_upload_cross_key_rekeys_cleanly() {
  // Server 1 creates secret and backup
  let (dir1, _state1, router1, token1) = admin_setup().await;

  let (status, proj, _) = call(
    &router1,
    "POST",
    "/api/v1/projects",
    Some(&token1),
    Some(json!({"name": "shop-project"})),
  )
  .await;
  assert_eq!(status, 201);
  let project_id = proj["data"]["id"].as_str().unwrap();

  let (status, env, _) = call(
    &router1,
    "POST",
    &format!("/api/v1/projects/{project_id}/environments"),
    Some(&token1),
    Some(json!({"name": "production"})),
  )
  .await;
  assert_eq!(status, 201);
  let env_id = env["data"]["id"].as_str().unwrap();

  let (status, _, _) = call(
    &router1,
    "PUT",
    &format!("/api/v1/environments/{env_id}/secrets/API_KEY"),
    Some(&token1),
    Some(json!({"value": "super-secret-key-12345"})),
  )
  .await;
  assert_eq!(status, 200);

  let (status, backup_res, _) = call(
    &router1,
    "POST",
    "/api/v1/backups",
    Some(&token1),
    Some(json!({"name": "shop_backup"})),
  )
  .await;
  assert_eq!(status, 201);
  let backup_key = backup_res["data"]["key"].as_str().unwrap();

  let (status, raw_backup, _) = call_raw(
    &router1,
    "GET",
    &format!("/api/v1/backups/{backup_key}"),
    Some(&token1),
  )
  .await;
  assert_eq!(status, 200);

  let server1_key_bytes = std::fs::read(dir1.path().join("master.key")).unwrap();

  // Server 2 is an established server with its own key and admin
  let (dir2, _state2, router2, token2) = admin_setup().await;
  let server2_orig_key = std::fs::read(dir2.path().join("master.key")).unwrap();
  assert_ne!(server1_key_bytes, server2_orig_key);

  // Upload Server 1 backup with Server 1 master key to Server 2
  let boundary = "---------------------------987654321098765432109876543";
  let mut upload_body = Vec::new();
  upload_body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
  upload_body.extend_from_slice(
    b"Content-Disposition: form-data; name=\"file\"; filename=\"shop_migrated.dop\"\r\n",
  );
  upload_body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
  upload_body.extend_from_slice(&raw_backup);
  upload_body.extend_from_slice(format!("\r\n--{boundary}\r\n").as_bytes());
  upload_body.extend_from_slice(
    b"Content-Disposition: form-data; name=\"master_key\"; filename=\"master.key\"\r\n",
  );
  upload_body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
  upload_body.extend_from_slice(&server1_key_bytes);
  upload_body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

  let req_upload = Request::builder()
    .method("POST")
    .uri("/api/v1/backups/upload")
    .header(header::AUTHORIZATION, format!("Bearer {token2}"))
    .header(
      header::CONTENT_TYPE,
      format!("multipart/form-data; boundary={boundary}"),
    )
    .body(Body::from(upload_body))
    .unwrap();
  let res_upload = router2.clone().oneshot(req_upload).await.unwrap();
  assert_eq!(res_upload.status().as_u16(), 201);
  let upload_res_body = axum::body::to_bytes(res_upload.into_body(), 1024 * 1024)
    .await
    .unwrap();
  let upload_json: Value = serde_json::from_slice(&upload_res_body).unwrap();
  let uploaded_key = upload_json["data"]["key"].as_str().unwrap();

  // Now restore on Server 2 directly WITHOUT any master key!
  // Because it was already re-keyed during upload to Server 2's master key!
  let (status, _, _) = call(
    &router2,
    "POST",
    &format!("/api/v1/backups/{uploaded_key}/restore"),
    Some(&token2),
    None,
  )
  .await;
  assert_eq!(status, 200);

  // Server 2 master key was NOT changed
  let server2_key_after = std::fs::read(dir2.path().join("master.key")).unwrap();
  assert_eq!(server2_key_after, server2_orig_key);

  // Reveal restored secret on Server 2
  let (status, secret_res, _) = call(
    &router2,
    "POST",
    &format!("/api/v1/environments/{env_id}/secrets/API_KEY/reveal"),
    Some(&token2),
    None,
  )
  .await;
  assert_eq!(status, 200);
  assert_eq!(secret_res["data"]["value"], "super-secret-key-12345");
}
