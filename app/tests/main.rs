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
  let (directory, state, router) = test_app_with_docs(true).await;
  (directory, state, router)
}

async fn test_app_with_docs(docs_enabled: bool) -> (TempDir, app::state::AppState, Router) {
  let directory = TempDir::new().unwrap();
  let database = directory.path().join("dopbase.db");
  let config = ServerConfig {
    database_url: format!("sqlite://{}", database.display()),
    master_key: app::config::MasterKeyConfig {
      provider: "file".into(),
      path: directory.path().join("master.key"),
    },
    docs_enabled,
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

#[tokio::test]
async fn health_and_openapi_are_available() {
  let (_directory, state, router) = test_app().await;
  let (status, body, _) = call(&router, "GET", "/api/v1/health", None, None).await;
  assert_eq!(status, 200);
  assert_eq!(body["data"]["product"], "dopbase");
  let (status, spec, _) = call(&router, "GET", "/api/v1/openapi.json", None, None).await;
  assert_eq!(status, 200);
  for path in [
    "/api/v1/health",
    "/api/v1/bootstrap/status",
    "/api/v1/bootstrap/admin",
    "/api/v1/auth/login",
    "/api/v1/auth/logout",
    "/api/v1/auth/session",
    "/api/v1/auth/reauthenticate",
    "/api/v1/auth/change-password",
    "/api/v1/projects",
    "/api/v1/projects/init",
    "/api/v1/projects/{project_ref}",
    "/api/v1/environments",
    "/api/v1/environments/resolve",
    "/api/v1/projects/{project_ref}/environments",
    "/api/v1/environments/{environment_id}",
    "/api/v1/environments/{environment_id}/secrets",
    "/api/v1/environments/{environment_id}/secrets/{key}",
    "/api/v1/environments/{environment_id}/secrets/{key}/reveal",
    "/api/v1/environments/{environment_id}/secrets/import",
    "/api/v1/environments/{environment_id}/secrets/layout",
    "/api/v1/environments/{environment_id}/secrets/export",
    "/api/v1/environments/{environment_id}/secrets/runtime",
    "/api/v1/environments/{environment_id}/tokens",
    "/api/v1/tokens/{token_id}/revoke",
    "/api/v1/audit-events",
    "/api/v1/instance",
  ] {
    assert!(
      spec["paths"][path].is_object(),
      "missing OpenAPI path {path}"
    );
  }
  assert!(spec["components"]["schemas"]["ErrorBody"].is_object());
  state.db.close().await;
}

#[tokio::test]
async fn docs_are_disabled_by_default() {
  let (_directory, state, router) = test_app_with_docs(false).await;
  let (status, body, _) = call(&router, "GET", "/api/v1/health", None, None).await;
  assert_eq!(status, 200);
  assert_eq!(body["data"]["product"], "dopbase");
  let (status, body, _) = call(&router, "GET", "/api/v1/openapi.json", None, None).await;
  assert_eq!(status, 404);
  assert_eq!(
    body,
    json!({"success":false,"error":{"REQUEST_INVALID":"The requested API route was not found."}})
  );
  let (status, _, _) = call(&router, "GET", "/api/docs", None, None).await;
  assert_eq!(status, 404);
  state.db.close().await;
}

#[tokio::test]
async fn docs_can_be_enabled_per_config() {
  let (_directory, state, router) = test_app_with_docs(true).await;
  let request = Request::builder()
    .uri("/api/docs/")
    .body(Body::empty())
    .unwrap();
  let response = router.clone().oneshot(request).await.unwrap();
  assert_eq!(response.status().as_u16(), 200, "trailing-slash docs route");
  assert_eq!(
    response.headers().get(header::CONTENT_TYPE).unwrap(),
    "text/html"
  );
  let (status, spec, _) = call(&router, "GET", "/api/v1/openapi.json", None, None).await;
  assert_eq!(status, 200);
  assert!(spec["paths"]["/api/v1/health"].is_object());
  state.db.close().await;
}

#[cfg(not(feature = "embedded-ui"))]
#[tokio::test]
async fn backend_only_root_explains_how_to_start_the_admin_ui() {
  let (_directory, state, router) = test_app().await;
  let request = Request::builder().uri("/").body(Body::empty()).unwrap();
  let response = router.oneshot(request).await.unwrap();

  assert_eq!(response.status(), 200);
  assert_eq!(
    response.headers().get(header::CONTENT_TYPE).unwrap(),
    "text/html; charset=utf-8"
  );
  let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
  let html = String::from_utf8(body.to_vec()).unwrap();
  assert!(html.contains("backend-only development build"));
  assert!(html.contains("bun run dev"));
  assert!(html.contains("bun run build:binary"));
  state.db.close().await;
}

#[tokio::test]
async fn validation_uses_required_error_map() {
  let (_directory, state, router) = test_app().await;
  let setup = state.setup.read().await.token.clone().unwrap();
  let (status, body, _) = call(
    &router,
    "POST",
    "/api/v1/bootstrap/admin",
    None,
    Some(json!({"setupToken":setup,"email":"invalid","password":"123456789012"})),
  )
  .await;
  assert_eq!(status, 422);
  assert_eq!(
    body,
    json!({"success":false,"error":{"EMAIL_INVAILD":"Please use proper email"}})
  );
  state.db.close().await;
}

#[tokio::test]
async fn malformed_json_uses_required_error_map() {
  let (_directory, state, router) = test_app().await;
  let request = Request::builder()
    .method("POST")
    .uri("/api/v1/auth/login")
    .header(header::CONTENT_TYPE, "application/json")
    .body(Body::from("{"))
    .unwrap();
  let response = router.oneshot(request).await.unwrap();
  assert_eq!(response.status(), 400);
  let body: Value =
    serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
  assert_eq!(
    body,
    json!({"success":false,"error":{"REQUEST_INVALID":"The request could not be processed."}})
  );
  state.db.close().await;
}

#[tokio::test]
async fn unknown_api_route_uses_required_error_map() {
  let (_directory, state, router) = test_app().await;
  let (status, body, _) = call(&router, "GET", "/api/v1/unknown", None, None).await;
  assert_eq!(status, 404);
  assert_eq!(
    body,
    json!({"success":false,"error":{"REQUEST_INVALID":"The requested API route was not found."}})
  );
  state.db.close().await;
}

#[tokio::test]
async fn end_to_end_admin_and_runner_flow() {
  let (_directory, state, router) = test_app().await;
  let setup = state.setup.read().await.token.clone().unwrap();
  let (status, _, _) = call(
    &router,
    "POST",
    "/api/v1/bootstrap/admin",
    None,
    Some(json!({"setupToken":setup,"email":"admin@example.com","password":"correct-horse-123"})),
  )
  .await;
  assert_eq!(status, 201);
  let (status, login, _) = call(
    &router,
    "POST",
    "/api/v1/auth/login",
    None,
    Some(json!({"email":"admin@example.com","password":"correct-horse-123","sessionKind":"cli"})),
  )
  .await;
  assert_eq!(status, 200);
  let admin_token = login["data"]["token"].as_str().unwrap();
  let (status, project, _) = call(
    &router,
    "POST",
    "/api/v1/projects",
    Some(admin_token),
    Some(json!({"name":"payment-service"})),
  )
  .await;
  assert_eq!(status, 201);
  assert_eq!(project["data"]["name"], "payment-service");
  let (status, environment, _) = call(
    &router,
    "POST",
    "/api/v1/projects/payment-service/environments",
    Some(admin_token),
    Some(json!({"name":"production"})),
  )
  .await;
  assert_eq!(status, 201);
  let environment_id = environment["data"]["id"].as_str().unwrap();
  let secret_path = format!("/api/v1/environments/{environment_id}/secrets/DATABASE_URL");
  let (status, _, _) = call(
    &router,
    "PUT",
    &secret_path,
    Some(admin_token),
    Some(json!({"value":"postgres://private"})),
  )
  .await;
  assert_eq!(status, 200);
  let (status, reveal, _) = call(
    &router,
    "POST",
    &format!("{secret_path}/reveal"),
    Some(admin_token),
    None,
  )
  .await;
  assert_eq!(status, 200);
  assert_eq!(reveal["data"]["value"], "postgres://private");
  let (status, created, _) = call(
    &router,
    "POST",
    &format!("/api/v1/environments/{environment_id}/tokens"),
    Some(admin_token),
    Some(json!({"name":"production-server","role":"runner"})),
  )
  .await;
  assert_eq!(status, 201);
  let runner = created["data"]["plaintextToken"].as_str().unwrap();
  let (status, runtime, _) = call(
    &router,
    "GET",
    &format!("/api/v1/environments/{environment_id}/secrets/runtime"),
    Some(runner),
    None,
  )
  .await;
  assert_eq!(status, 200);
  assert_eq!(runtime["data"]["entries"][0]["value"], "postgres://private");
  let (status, denied, _) = call(&router, "GET", "/api/v1/projects", Some(runner), None).await;
  assert_eq!(status, 403);
  assert!(denied["error"]["AUTHORIZATION_DENIED"].is_string());
  let (status, audit, _) = call(
    &router,
    "GET",
    "/api/v1/audit-events?limit=10",
    Some(admin_token),
    None,
  )
  .await;
  assert_eq!(status, 200);
  assert!(!audit["data"]["items"].as_array().unwrap().is_empty());
  state.db.close().await;
}

#[tokio::test]
async fn env_layout_persists_with_import_and_omits_values() {
  let (_directory, state, router) = test_app().await;
  let setup = state.setup.read().await.token.clone().unwrap();
  let (status, _, _) = call(
    &router,
    "POST",
    "/api/v1/bootstrap/admin",
    None,
    Some(json!({"setupToken":setup,"email":"admin@example.com","password":"correct-horse-123"})),
  )
  .await;
  assert_eq!(status, 201);
  let (status, login, _) = call(
    &router,
    "POST",
    "/api/v1/auth/login",
    None,
    Some(json!({"email":"admin@example.com","password":"correct-horse-123","sessionKind":"cli"})),
  )
  .await;
  assert_eq!(status, 200);
  let token = login["data"]["token"].as_str().unwrap();
  let (status, _, _) = call(
    &router,
    "POST",
    "/api/v1/projects",
    Some(token),
    Some(json!({"name":"layout-service"})),
  )
  .await;
  assert_eq!(status, 201);
  let (status, environment, _) = call(
    &router,
    "POST",
    "/api/v1/projects/layout-service/environments",
    Some(token),
    Some(json!({"name":"production"})),
  )
  .await;
  assert_eq!(status, 201);
  let environment_id = environment["data"]["id"].as_str().unwrap();
  let base = format!("/api/v1/environments/{environment_id}/secrets");

  // No layout is stored until the editor saves one.
  let (status, body, _) = call(&router, "GET", &format!("{base}/layout"), Some(token), None).await;
  assert_eq!(status, 200);
  assert!(body["data"]["layout"].is_null());

  // A dry-run import carrying a layout must not persist it.
  let layout_text = "# connection\nDATABASE_URL=\n\n# app\nAPI_KEY=\n";
  let (status, dry, _) = call(
    &router,
    "POST",
    &format!("{base}/import"),
    Some(token),
    Some(json!({
        "mode":"replace",
        "dryRun":true,
        "entries":[
            {"key":"DATABASE_URL","value":"postgres://private"},
            {"key":"API_KEY","value":"k-123"}
        ],
        "envLayout":layout_text
    })),
  )
  .await;
  assert_eq!(status, 200);
  assert_eq!(dry["data"]["addedKeys"].as_array().unwrap().len(), 2);
  let (status, body, _) = call(&router, "GET", &format!("{base}/layout"), Some(token), None).await;
  assert_eq!(status, 200);
  assert!(body["data"]["layout"].is_null());

  // Applying the import persists the layout exactly as submitted — and
  // never with secret values inside.
  let (status, _, _) = call(
    &router,
    "POST",
    &format!("{base}/import"),
    Some(token),
    Some(json!({
        "mode":"replace",
        "dryRun":false,
        "entries":[
            {"key":"DATABASE_URL","value":"postgres://private"},
            {"key":"API_KEY","value":"k-123"}
        ],
        "envLayout":layout_text
    })),
  )
  .await;
  assert_eq!(status, 200);
  let (status, body, _) = call(&router, "GET", &format!("{base}/layout"), Some(token), None).await;
  assert_eq!(status, 200);
  assert_eq!(body["data"]["layout"], layout_text);
  assert!(
    !body["data"]["layout"]
      .as_str()
      .unwrap()
      .contains("postgres")
  );

  // A layout beyond the size limit is rejected.
  let huge = "x".repeat(64 * 1024 + 1);
  let (status, _, _) = call(
    &router,
    "POST",
    &format!("{base}/import"),
    Some(token),
    Some(json!({
        "mode":"replace",
        "dryRun":false,
        "entries":[{"key":"DATABASE_URL","value":"v"}],
        "envLayout":huge
    })),
  )
  .await;
  assert_eq!(status, 422);
  state.db.close().await;
}
