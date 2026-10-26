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
  let state = server::build_setup_state(config).await.unwrap();
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
  assert_eq!(status, 201);
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
  assert_eq!(status, 201);
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
  assert_eq!(status, 200);
  assert_eq!(body["data"].as_array().unwrap().len(), 2);
  let (status, listed, _) = call(&router, "GET", "/api/v1/users", Some(&cookie), None, None).await;
  assert_eq!(status, 200);
  assert_eq!(listed["data"].as_array().unwrap().len(), 2);
  // The root user's actions are recorded as `root`; other administrators
  // stay `admin`.
  for (action, actor_label, expected) in [
    ("admin.bootstrapped", "root@example.com", "root"),
    ("admin.created", "root@example.com", "root"),
    ("login.succeeded", "admin@example.com", "admin"),
  ] {
    let actor_type: String = sqlx::query_scalar(
      "SELECT actor_type FROM audit_events WHERE action=? AND actor_label=? ORDER BY seq DESC LIMIT 1",
    )
    .bind(action)
    .bind(actor_label)
    .fetch_one(state.db.pool())
    .await
    .unwrap();
    assert_eq!(actor_type, expected, "{action} by {actor_label}");
  }
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
  let (status, _, _) = call(&router, "POST", "/api/v1/instance/factory-reset", Some(&cookie), Some(&csrf), Some(json!({"currentPassword":"correct-horse-123","confirmation":"FACTORY RESET","acknowledged":true}))).await;
  assert_eq!(status, 200);
  let (status, body, _) = call(&router, "GET", "/api/v1/bootstrap/status", None, None, None).await;
  assert_eq!(status, 200);
  assert_eq!(body["data"]["state"], "setupRequired");
  state.db.close().await;
}

#[tokio::test]
async fn accounts_keep_their_role_row_and_starting_grant_in_step() {
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
  assert_eq!(status, 201);
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
    Some(json!({"email":"teammate@example.com","password":"correct-horse-456","role":"member"})),
  )
  .await;
  assert_eq!(status, 201);
  let id = created["data"]["id"].as_str().unwrap().to_owned();

  let (status, session, _) = call(
    &router,
    "GET",
    "/api/v1/auth/session",
    Some(&cookie),
    None,
    None,
  )
  .await;
  assert_eq!(status, 200);
  assert_eq!(session["data"]["roleName"], "Root");
  assert_eq!(session["data"]["maxLevel"], "project_manager");
  assert!(
    session["data"]["capabilities"]
      .as_array()
      .unwrap()
      .contains(&json!("instance.backup"))
  );

  let account = |id: String| {
    let pool = state.db.pool().clone();
    async move {
      let row: (i64, Option<String>, Option<String>) =
        sqlx::query_as("SELECT is_root, role_id, created_by FROM admins WHERE id = ?")
          .bind(&id)
          .fetch_one(&pool)
          .await
          .unwrap();
      let grants: Vec<(String, String)> = sqlx::query_as(
        "SELECT scope_type, level FROM grants WHERE principal_type = 'user' AND principal_id = ?",
      )
      .bind(&id)
      .fetch_all(&pool)
      .await
      .unwrap();
      (row, grants)
    }
  };
  let root_id: String = sqlx::query_scalar("SELECT id FROM admins WHERE is_root = 1")
    .fetch_one(state.db.pool())
    .await
    .unwrap();
  assert_eq!(account(root_id.clone()).await, ((1, None, None), vec![]));
  // A new member reaches nothing until someone grants it a project.
  assert_eq!(
    account(id.clone()).await,
    (
      (0, Some("rol_member".into()), Some(root_id.clone())),
      vec![]
    )
  );

  let (status, _, _) = call(
    &router,
    "PATCH",
    &format!("/api/v1/users/{id}"),
    Some(&cookie),
    Some(&csrf),
    Some(json!({"role":"admin"})),
  )
  .await;
  assert_eq!(status, 200);
  // Root makes admins project managers of everything.
  assert_eq!(
    account(id.clone()).await,
    (
      (0, Some("rol_admin".into()), Some(root_id.clone())),
      vec![("all".into(), "project_manager".into())]
    )
  );
  let (status, _, _) = call(
    &router,
    "PATCH",
    &format!("/api/v1/users/{id}"),
    Some(&cookie),
    Some(&csrf),
    Some(json!({"role":"member"})),
  )
  .await;
  assert_eq!(status, 200);
  assert_eq!(
    account(id.clone()).await,
    ((0, Some("rol_member".into()), Some(root_id)), vec![])
  );

  let (status, _, _) = call(
    &router,
    "DELETE",
    &format!("/api/v1/users/{id}"),
    Some(&cookie),
    Some(&csrf),
    None,
  )
  .await;
  assert_eq!(status, 200);
  let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM grants")
    .fetch_one(state.db.pool())
    .await
    .unwrap();
  assert_eq!(left, 0);
  state.db.close().await;
}

#[tokio::test]
async fn members_and_agents_cannot_create_projects() {
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
  let (status, _, _) = call(
    &router,
    "POST",
    "/api/v1/projects",
    Some(&member_cookie),
    Some(&member_csrf),
    Some(json!({"name":"member-project"})),
  )
  .await;
  assert_eq!(status, 403);
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
    "/api/v1/users",
    Some(&root_cookie),
    Some(&root_csrf),
    Some(json!({"email":"build-agent","role":"ai_agent"})),
  )
  .await;
  assert_eq!(status, 201);
  let agent_id = agent["data"]["id"].as_str().unwrap();
  let (status, token, _) = call(
    &router,
    "POST",
    &format!("/api/v1/users/{agent_id}/tokens"),
    Some(&root_cookie),
    Some(&root_csrf),
    Some(json!({"name":"default","level":"metadata"})),
  )
  .await;
  assert_eq!(status, 201);
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

#[tokio::test]
async fn ai_agent_accounts_hold_expiring_tokens_within_their_role() {
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
  let cookie = headers
    .get(header::SET_COOKIE)
    .unwrap()
    .to_str()
    .unwrap()
    .split(';')
    .next()
    .unwrap()
    .to_string();
  let csrf = bootstrap["data"]["csrfToken"].as_str().unwrap();
  let root_id = bootstrap["data"]["adminId"].as_str().unwrap();
  let create = |body: Value| {
    call(
      &router,
      "POST",
      "/api/v1/users",
      Some(&cookie),
      Some(csrf),
      Some(body),
    )
  };
  // Agents have a name, never a password, and the creator as owner.
  let (status, _, _) =
    create(json!({"email":"build-agent","role":"ai_agent","password":"correct-horse-123"})).await;
  assert_eq!(status, 400);
  let (status, _, _) = create(json!({"email":"agent@example.com","role":"ai_agent"})).await;
  assert_eq!(status, 422);
  let (status, account, _) = create(json!({"email":"build-agent","role":"ai_agent"})).await;
  assert_eq!(status, 201);
  assert_eq!(account["data"]["role"], "ai_agent");
  assert_eq!(account["data"]["ownerId"], root_id);
  let id = account["data"]["id"].as_str().unwrap();
  let path = format!("/api/v1/users/{id}/tokens");
  let token = |body: Value| {
    call(
      &router,
      "POST",
      &path,
      Some(&cookie),
      Some(csrf),
      Some(body),
    )
  };

  // Agent tokens always expire, within 90 days, and reach at most read.
  for (body, status) in [
    (
      json!({"name":"never","level":"metadata","expiresIn":"never"}),
      422,
    ),
    (
      json!({"name":"long","level":"metadata","expiresIn":"91d"}),
      422,
    ),
    (json!({"name":"writer","level":"write"}), 422),
    (
      json!({"name":"limit","level":"read","expiresIn":"90d"}),
      201,
    ),
  ] {
    let (actual, body, _) = token(body).await;
    assert_eq!(actual, status, "{body}");
  }
  let (status, created, _) = token(json!({"name":"default","level":"metadata"})).await;
  assert_eq!(status, 201);
  let bearer = created["data"]["plaintextToken"].as_str().unwrap();
  assert!(bearer.starts_with("dpp_"));
  let token_id = created["data"]["token"]["id"].as_str().unwrap();
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
  // Tokens never manage people or tokens.
  let (status, _, _) = call(
    &router,
    "GET",
    "/api/v1/users/me/tokens",
    Some(&format!("Bearer {bearer}")),
    None,
    None,
  )
  .await;
  assert_eq!(status, 403);
  sqlx::query("UPDATE personal_tokens SET expires_at=? WHERE id=?")
    .bind((chrono::Utc::now() - chrono::Duration::seconds(1)).to_rfc3339())
    .bind(token_id)
    .execute(state.db.pool())
    .await
    .unwrap();
  let (status, _, _) = call(
    &router,
    "GET",
    "/api/v1/status",
    Some(&format!("Bearer {bearer}")),
    None,
    None,
  )
  .await;
  assert_eq!(status, 401);

  // Root has no tokens of its own.
  let (status, _, _) = call(
    &router,
    "POST",
    "/api/v1/users/me/tokens",
    Some(&cookie),
    Some(csrf),
    Some(json!({"name":"mine","level":"read"})),
  )
  .await;
  assert_eq!(status, 403);
  state.db.close().await;
}
