use app::{config::ServerConfig, server};
use axum::{
  Router,
  body::{Body, to_bytes},
  http::{Request, header},
};
use serde_json::{Value, json};
use tempfile::TempDir;
use tower::ServiceExt;

async fn fixture() -> (TempDir, app::state::AppState, Router) {
  let dir = TempDir::new().unwrap();
  let config = ServerConfig {
    data_dir: dir.path().to_path_buf(),
    database_url: format!("sqlite://{}", dir.path().join("dopbase.db").display()),
    master_key: app::config::MasterKeyConfig {
      provider: "file".into(),
      path: dir.path().join("master.key"),
    },
    ..ServerConfig::default()
  };
  let state = server::build_state(config).await.unwrap();
  let router = server::router(state.clone());
  (dir, state, router)
}

async fn call(
  router: &Router,
  method: &str,
  path: &str,
  cookie: Option<&str>,
  csrf: Option<&str>,
  body: Option<Value>,
) -> (u16, Value, axum::http::HeaderMap) {
  let mut request = Request::builder().method(method).uri(path);
  if let Some(cookie) = cookie {
    if cookie.starts_with("Bearer ") {
      request = request.header(header::AUTHORIZATION, cookie);
    } else {
      request = request.header(header::COOKIE, cookie);
    }
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
        .body(body.map_or_else(Body::empty, |b| Body::from(b.to_string())))
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

#[tokio::test]
async fn root_can_manage_admins_but_cannot_delete_root_and_can_reset() {
  let (_dir, state, router) = fixture().await;
  let setup = state.setup.read().await.token.clone().unwrap();
  let (status, bootstrap, headers) = call(
    &router,
    "POST",
    "/api/v1/bootstrap/admin",
    None,
    None,
    Some(json!({"setupToken": setup, "email":"root@example.com", "password":"correct-horse-123"})),
  )
  .await;
  assert_eq!(status, 201, "bootstrap body: {bootstrap:?}");
  let cookie = headers
    .get(header::SET_COOKIE)
    .unwrap()
    .to_str()
    .unwrap()
    .split(';')
    .next()
    .unwrap()
    .to_string();
  let csrf = bootstrap["data"]["csrfToken"].as_str().unwrap().to_string();
  let (status, created, _) = call(
    &router,
    "POST",
    "/api/v1/users",
    Some(&cookie),
    Some(&csrf),
    Some(json!({"email":"admin@example.com","password":"correct-horse-456"})),
  )
  .await;
  assert_eq!(status, 201, "create body: {created:?}");
  assert_eq!(created["data"]["role"], "admin");
  let (status, _, admin_headers) = call(
    &router,
    "POST",
    "/api/v1/auth/login",
    None,
    None,
    Some(
      json!({"email":"admin@example.com","password":"correct-horse-456","sessionKind":"browser"}),
    ),
  )
  .await;
  assert_eq!(status, 200);
  let admin_cookie = admin_headers
    .get(header::SET_COOKIE)
    .unwrap()
    .to_str()
    .unwrap()
    .split(';')
    .next()
    .unwrap()
    .to_string();
  let (status, body, _) = call(
    &router,
    "GET",
    "/api/v1/users",
    Some(&admin_cookie),
    None,
    None,
  )
  .await;
  assert_eq!(status, 200, "admin user-list body: {body:?}");
  assert_eq!(body["data"].as_array().unwrap().len(), 2);
  let (status, listed, _) = call(&router, "GET", "/api/v1/users", Some(&cookie), None, None).await;
  assert_eq!(status, 200);
  assert_eq!(listed["data"].as_array().unwrap().len(), 2);
  let root_id = listed["data"][0]["id"].as_str().unwrap();
  let (status, body, _) = call(
    &router,
    "DELETE",
    &format!("/api/v1/users/{root_id}"),
    Some(&cookie),
    Some(&csrf),
    None,
  )
  .await;
  assert_eq!(status, 403);
  assert_eq!(
    body["error"]["SELF_DELETE_FORBIDDEN"],
    "You cannot delete your own account."
  );
  sqlx::query("INSERT INTO environment_id_reservations(id) VALUES('env_112255')")
    .execute(state.db.pool())
    .await
    .unwrap();
  let (status, _, _) = call(&router, "POST", "/api/v1/instance/factory-reset", Some(&cookie), Some(&csrf), Some(json!({"currentPassword":"correct-horse-123","confirmation":"FACTORY RESET","acknowledged":true}))).await;
  assert_eq!(status, 200);
  let reservation_count: i64 =
    sqlx::query_scalar("SELECT COUNT(*) FROM environment_id_reservations")
      .fetch_one(state.db.pool())
      .await
      .unwrap();
  assert_eq!(reservation_count, 0);
  let next_environment_number: i64 =
    sqlx::query_scalar("SELECT next_number FROM environment_id_sequence WHERE id=1")
      .fetch_one(state.db.pool())
      .await
      .unwrap();
  assert_eq!(next_environment_number, 1_000);
  let (status, body, _) = call(&router, "GET", "/api/v1/bootstrap/status", None, None, None).await;
  assert_eq!(status, 200);
  assert_eq!(body["data"]["state"], "setupRequired");
  state.db.close().await;
}

#[tokio::test]
async fn member_manages_projects_and_agent_is_metadata_only() {
  let (_dir, state, router) = fixture().await;
  let setup = state.setup.read().await.token.clone().unwrap();
  let (status, bootstrap, headers) = call(
    &router,
    "POST",
    "/api/v1/bootstrap/admin",
    None,
    None,
    Some(json!({"setupToken":setup,"email":"root@example.com","password":"correct-horse-123"})),
  )
  .await;
  assert_eq!(status, 201);
  let root_cookie = headers
    .get(header::SET_COOKIE)
    .unwrap()
    .to_str()
    .unwrap()
    .split(';')
    .next()
    .unwrap()
    .to_string();
  let root_csrf = bootstrap["data"]["csrfToken"].as_str().unwrap().to_string();
  let (status, member, _) = call(
    &router,
    "POST",
    "/api/v1/users",
    Some(&root_cookie),
    Some(&root_csrf),
    Some(json!({"email":"member@example.com","password":"correct-horse-456","role":"member"})),
  )
  .await;
  assert_eq!(status, 201);
  assert_eq!(member["data"]["role"], "member");
  let (status, login, headers) = call(
    &router,
    "POST",
    "/api/v1/auth/login",
    None,
    None,
    Some(
      json!({"email":"member@example.com","password":"correct-horse-456","sessionKind":"browser"}),
    ),
  )
  .await;
  assert_eq!(status, 200);
  let member_cookie = headers
    .get(header::SET_COOKIE)
    .unwrap()
    .to_str()
    .unwrap()
    .split(';')
    .next()
    .unwrap()
    .to_string();
  let member_csrf = login["data"]["csrfToken"].as_str().unwrap().to_string();
  let (status, project, _) = call(
    &router,
    "POST",
    "/api/v1/projects",
    Some(&member_cookie),
    Some(&member_csrf),
    Some(json!({"name":"member-project"})),
  )
  .await;
  assert_eq!(status, 201, "member project body: {project:?}");
  let (status, _, _) = call(
    &router,
    "GET",
    "/api/v1/users",
    Some(&member_cookie),
    None,
    None,
  )
  .await;
  assert_eq!(status, 403);
  let (status, agent, _) = call(
    &router,
    "POST",
    "/api/v1/service-accounts",
    Some(&root_cookie),
    Some(&root_csrf),
    Some(json!({"name":"build-agent"})),
  )
  .await;
  assert_eq!(status, 201, "agent body: {agent:?}");
  let agent_id = agent["data"]["id"].as_str().unwrap();
  let (status, token, _) = call(
    &router,
    "POST",
    &format!("/api/v1/service-accounts/{agent_id}/tokens"),
    Some(&root_cookie),
    Some(&root_csrf),
    Some(json!({"name":"default"})),
  )
  .await;
  assert_eq!(status, 201, "token body: {token:?}");
  let bearer = token["data"]["plaintextToken"].as_str().unwrap();
  let (status, _, _) = call(
    &router,
    "GET",
    "/api/v1/status",
    Some(&format!("Bearer {bearer}")),
    None,
    None,
  )
  .await;
  assert_eq!(status, 200);
  let (status, _, _) = call(
    &router,
    "POST",
    "/api/v1/projects",
    Some(&format!("Bearer {bearer}")),
    None,
    Some(json!({"name":"agent-project"})),
  )
  .await;
  assert_eq!(status, 403);
  state.db.close().await;
}
