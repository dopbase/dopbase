use super::grants_access::{Person, instance, person, send};
use super::*;
use app::services::features::Features;

/// A router over the same data with a people limit of `max`.
fn limited(
  state: &app::state::AppState,
  max: u32,
) -> Router {
  with_features(
    state,
    Features {
      max_users: max,
      ..Features::all()
    },
  )
}

async fn add(
  router: &Router,
  by: &Person,
  email: &str,
  role: &str,
) -> (u16, Value) {
  let body = match role {
    "ai_agent" => json!({"email":email,"role":role}),
    // Auditor has no legacy role name, only its id.
    "auditor" => json!({"email":email,"password":"correct-horse-123","roleId":"rol_auditor"}),
    _ => json!({"email":email,"password":"correct-horse-123","role":role}),
  };
  send(router, "POST", "/api/v1/users", by.browser(), Some(body)).await
}

async fn set_status(
  router: &Router,
  by: &Person,
  user: &str,
  action: &str,
) -> (u16, Value) {
  send(
    router,
    "POST",
    &format!("/api/v1/users/{user}/{action}"),
    by.browser(),
    None,
  )
  .await
}

/// Asserts a request was refused because the user limit is full.
fn limit_reached(
  status: u16,
  body: &Value,
) {
  assert_eq!(status, 409, "{body}");
  let message = body["error"]["USER_LIMIT_REACHED"].as_str();
  assert!(
    message.is_some_and(|message| message.starts_with("This instance allows up to")
      && !message.contains("DOPBASE_RBAC_MAX_USERS")),
    "{body}"
  );
}

async fn user_limit_check(
  router: &Router,
  root: &Person,
) -> Option<Value> {
  let (status, body) = send(router, "GET", "/api/v1/status", root.cli(), None).await;
  assert_eq!(status, 200, "{body}");
  body["data"]["checks"]
    .as_array()
    .unwrap()
    .iter()
    .find(|check| check["id"] == "users.limit")
    .cloned()
}

async fn max_users(
  router: &Router,
  who: &Person,
) -> Value {
  let (status, session) = send(router, "GET", "/api/v1/auth/session", who.browser(), None).await;
  assert_eq!(status, 200, "{session}");
  session["data"]["maxUsers"].clone()
}

#[tokio::test]
async fn the_people_limit_counts_active_people_only() {
  let (_dir, state, router) = test_app().await;
  let (root, _) = instance(&state, &router).await;
  let two = limited(&state, 2);

  // No limit by default.
  assert_eq!(max_users(&router, &root).await, Value::Null);
  assert!(user_limit_check(&router, &root).await.is_none());
  assert_eq!(max_users(&two, &root).await, 2);

  // Root and AI agents never take a spot.
  let (status, body) = add(&two, &root, "deploy-bot", "ai_agent").await;
  assert_eq!(status, 201, "{body}");
  let member = person(&two, &root, "member@example.com", "member").await;
  let check = user_limit_check(&two, &root).await.unwrap();
  assert_eq!(check["status"], "ok", "{check}");
  assert_eq!(check["title"], "Active users within limit");
  assert_eq!(check["value"], "1 of 2");
  let (status, body) = add(&two, &root, "auditor@example.com", "auditor").await;
  assert_eq!(status, 201, "{body}");

  // Full: nobody else is added, but AI agents still are.
  let (status, body) = add(&two, &root, "third@example.com", "admin").await;
  limit_reached(status, &body);
  let (status, body) = send(
    &two,
    "GET",
    "/api/v1/users/identities",
    root.browser(),
    None,
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert!(!body.to_string().contains("third@example.com"), "{body}");
  let (status, body) = add(&two, &root, "backup-bot", "ai_agent").await;
  assert_eq!(status, 201, "{body}");
  let check = user_limit_check(&two, &root).await.unwrap();
  assert_eq!(check["status"], "warn", "{check}");
  assert_eq!(check["title"], "User limit reached");
  assert_eq!(check["value"], "2 of 2");
  assert_eq!(check["fixPath"], "/users");

  // A disabled person frees their spot, and takes it back only if it's free.
  let (status, body) = set_status(&two, &root, &member.id, "disable").await;
  assert_eq!(status, 200, "{body}");
  let (status, newcomer) = add(&two, &root, "newcomer@example.com", "member").await;
  assert_eq!(status, 201, "{newcomer}");
  let (status, body) = set_status(&two, &root, &member.id, "enable").await;
  limit_reached(status, &body);
  let (status, body) = send(
    &two,
    "GET",
    &format!("/api/v1/users/{}", member.id),
    root.browser(),
    None,
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["status"], "disabled");
  let (status, body) = send(
    &two,
    "DELETE",
    &format!("/api/v1/users/{}", newcomer["data"]["id"].as_str().unwrap()),
    root.browser(),
    None,
  )
  .await;
  assert!(status < 300, "{status} {body}");
  let (status, body) = set_status(&two, &root, &member.id, "enable").await;
  assert_eq!(status, 200, "{body}");

  // Lowering the limit below the count shuts nobody out; it only stops
  // new people.
  let one = limited(&state, 1);
  let (status, body, _) = call(
    &one,
    "POST",
    "/api/v1/auth/login",
    None,
    Some(json!({"email":"member@example.com","password":"correct-horse-123","sessionKind":"cli"})),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  let (status, body) = add(&one, &root, "late@example.com", "member").await;
  limit_reached(status, &body);
  let check = user_limit_check(&one, &root).await.unwrap();
  assert_eq!(check["status"], "warn", "{check}");
  assert_eq!(check["title"], "User limit exceeded");
  assert!(
    check["detail"].as_str().is_some_and(
      |detail| detail.starts_with("This instance has 2 active users, above its limit of 1.")
    ),
    "{check}"
  );
  assert_eq!(check["value"], "2 of 1");

  // The limit needs RBAC, like adding people does.
  let without_rbac = with_features(
    &state,
    Features {
      rbac: false,
      max_users: 1,
      ..Features::all()
    },
  );
  assert_eq!(max_users(&without_rbac, &root).await, Value::Null);
  assert!(user_limit_check(&without_rbac, &root).await.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn people_added_at_the_same_time_never_pass_the_limit() {
  let (_dir, state, router) = test_app().await;
  let (root, _) = instance(&state, &router).await;
  let one = limited(&state, 1);
  let root = std::sync::Arc::new(root);
  let barrier = std::sync::Arc::new(Barrier::new(6));
  let mut tasks = Vec::new();
  for index in 0..6 {
    let (router, root, barrier) = (one.clone(), root.clone(), barrier.clone());
    tasks.push(tokio::spawn(async move {
      barrier.wait().await;
      add(&router, &root, &format!("p{index}@example.com"), "member").await
    }));
  }
  let mut added = 0;
  for task in tasks {
    let (status, body) = task.await.unwrap();
    if status == 201 {
      added += 1;
    } else {
      limit_reached(status, &body);
    }
  }
  assert_eq!(added, 1);
  let people: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admins WHERE is_root = 0")
    .fetch_one(state.db.pool())
    .await
    .unwrap();
  assert_eq!(people, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn people_enabled_at_the_same_time_never_pass_the_limit() {
  let (_dir, state, router) = test_app().await;
  let (root, _) = instance(&state, &router).await;
  let mut ids = Vec::new();
  for index in 0..4 {
    let (status, body) = add(&router, &root, &format!("p{index}@example.com"), "member").await;
    assert_eq!(status, 201, "{body}");
    let id = body["data"]["id"].as_str().unwrap().to_owned();
    let (status, body) = set_status(&router, &root, &id, "disable").await;
    assert_eq!(status, 200, "{body}");
    ids.push(id);
  }
  let one = limited(&state, 1);
  let root = std::sync::Arc::new(root);
  let barrier = std::sync::Arc::new(Barrier::new(ids.len()));
  let mut tasks = Vec::new();
  for id in ids {
    let (router, root, barrier) = (one.clone(), root.clone(), barrier.clone());
    tasks.push(tokio::spawn(async move {
      barrier.wait().await;
      set_status(&router, &root, &id, "enable").await
    }));
  }
  let mut enabled = 0;
  for task in tasks {
    let (status, body) = task.await.unwrap();
    if status == 200 {
      enabled += 1;
    } else {
      limit_reached(status, &body);
    }
  }
  assert_eq!(enabled, 1);
}
