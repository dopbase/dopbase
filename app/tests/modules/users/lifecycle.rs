use super::grants_access::{As, Person, grant, instance, person, send};
use super::*;

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

async fn password_login(
  router: &Router,
  email: &str,
) -> u16 {
  let (status, _, _) = call(
    router,
    "POST",
    "/api/v1/auth/login",
    None,
    Some(json!({"email":email,"password":"correct-horse-123","sessionKind":"cli"})),
  )
  .await;
  status
}

async fn audit_count(
  state: &app::state::AppState,
  action: &str,
  subject: &str,
) -> i64 {
  sqlx::query_scalar("SELECT COUNT(*) FROM audit_events WHERE action = ? AND resource_id = ?")
    .bind(action)
    .bind(subject)
    .fetch_one(state.db.pool())
    .await
    .unwrap()
}

async fn last_sign_in(
  router: &Router,
  by: &Person,
  user: &str,
) -> Value {
  let (status, body) = send(
    router,
    "GET",
    &format!("/api/v1/users/{user}"),
    by.browser(),
    None,
  )
  .await;
  assert_eq!(status, 200, "{body}");
  body["data"]["lastSignInAt"].clone()
}

async fn forget_sign_in(
  state: &app::state::AppState,
  user: &str,
) {
  sqlx::query("UPDATE admins SET last_sign_in_at = NULL WHERE id = ?")
    .bind(user)
    .execute(state.db.pool())
    .await
    .unwrap();
}

#[tokio::test]
async fn a_disabled_account_is_shut_out_until_it_is_enabled() {
  let (_dir, state, router) = test_app().await;
  let (root, _) = instance(&state, &router).await;
  let admin = person(&router, &root, "lead@example.com", "admin").await;
  let member = person(&router, &root, "member@example.com", "member").await;
  let (status, body) = send(
    &router,
    "POST",
    "/api/v1/users/me/tokens",
    member.browser(),
    Some(json!({"name":"laptop","level":"read"})),
  )
  .await;
  assert_eq!(status, 201, "{body}");
  let token = body["data"]["plaintextToken"].as_str().unwrap().to_owned();
  let projects = |auth| send(&router, "GET", "/api/v1/projects", auth, None);
  assert_eq!(projects(As::Token(&token)).await.0, 200);

  let (status, body) = set_status(&router, &admin, &member.id, "disable").await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["status"], "disabled");
  // Sessions, tokens, and new sign-ins are all refused.
  assert_eq!(projects(member.cli()).await.0, 401);
  assert_eq!(projects(member.browser()).await.0, 401);
  assert_eq!(projects(As::Token(&token)).await.0, 401);
  assert_eq!(password_login(&router, "member@example.com").await, 401);
  let live: i64 =
    sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE admin_id = ? AND revoked_at IS NULL")
      .bind(&member.id)
      .fetch_one(state.db.pool())
      .await
      .unwrap();
  assert_eq!(live, 0);
  // Disabling again changes nothing and records nothing.
  let (status, body) = set_status(&router, &admin, &member.id, "disable").await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["status"], "disabled");
  assert_eq!(audit_count(&state, "user.disabled", &member.id).await, 1);

  let (status, body) = set_status(&router, &admin, &member.id, "enable").await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["status"], "active");
  // The token works again; the ended sessions stay ended.
  assert_eq!(projects(As::Token(&token)).await.0, 200);
  assert_eq!(projects(member.cli()).await.0, 401);
  let again = super::grants_access::sign_in(&router, "member@example.com").await;
  assert_eq!(projects(again.cli()).await.0, 200);
  let (status, _) = set_status(&router, &admin, &member.id, "enable").await;
  assert_eq!(status, 200);
  assert_eq!(audit_count(&state, "user.enabled", &member.id).await, 1);
  state.db.close().await;
}

#[tokio::test]
async fn only_someone_who_reaches_further_disables_an_account() {
  let (_dir, state, router) = test_app().await;
  let (root, _) = instance(&state, &router).await;
  let lead = person(&router, &root, "lead@example.com", "admin").await;
  let ops = person(&router, &root, "ops@example.com", "admin").await;
  let member = person(&router, &root, "member@example.com", "member").await;

  for (target, code) in [
    (&root.id, "ROOT_PROTECTED"),
    (&lead.id, "SELF_DISABLE_FORBIDDEN"),
  ] {
    let (status, body) = set_status(&router, &lead, target, "disable").await;
    assert_eq!(status, 403, "{body}");
    assert!(body["error"][code].is_string(), "{body}");
  }
  // Without users.manage there is nothing to disable.
  let (status, body) = set_status(&router, &member, &lead.id, "disable").await;
  assert_eq!(status, 403, "{body}");
  assert!(body["error"]["AUTHORIZATION_DENIED"].is_string());
  let (status, _) = send(
    &router,
    "POST",
    &format!("/api/v1/users/{}/disable", member.id),
    As::Browser(&lead.cookie, "wrong"),
    None,
  )
  .await;
  assert_eq!(status, 403);

  // Cut the lead to managing payments: ops, on all projects, is out of reach,
  // and stays out of reach once root disables it.
  let (_, access) = send(
    &router,
    "GET",
    &format!("/api/v1/users/{}/access", lead.id),
    root.browser(),
    None,
  )
  .await;
  let all = access["data"]["grants"][0]["id"]
    .as_str()
    .unwrap()
    .to_owned();
  send(
    &router,
    "DELETE",
    &format!("/api/v1/grants/{all}"),
    root.browser(),
    None,
  )
  .await;
  grant(
    &router,
    &root,
    ("user", &lead.id),
    ("project", Some("payments")),
    "project_manager",
  )
  .await;
  let (status, body) = set_status(&router, &lead, &ops.id, "disable").await;
  assert_eq!(status, 403, "{body}");
  assert!(body["error"]["ACCESS_EXCEEDS_YOURS"].is_string());
  let (status, _) = set_status(&router, &root, &ops.id, "disable").await;
  assert_eq!(status, 200);
  let (status, body) = set_status(&router, &lead, &ops.id, "enable").await;
  assert_eq!(status, 403, "{body}");
  assert!(body["error"]["ACCESS_EXCEEDS_YOURS"].is_string());
  assert_eq!(audit_count(&state, "user.enabled", &ops.id).await, 0);
  state.db.close().await;
}

#[tokio::test]
async fn interactive_sign_ins_set_the_last_sign_in_and_nothing_else_does() {
  let (_dir, state, router) = test_app().await;
  let (root, _) = instance(&state, &router).await;
  let member = person(&router, &root, "member@example.com", "member").await;
  let (_, body) = send(
    &router,
    "GET",
    &format!("/api/v1/users/{}", member.id),
    root.browser(),
    None,
  )
  .await;
  assert_eq!(body["data"]["status"], "active");
  assert!(body["data"]["lastSignInAt"].is_string(), "{body}");

  forget_sign_in(&state, &member.id).await;
  assert_eq!(password_login(&router, "member@example.com").await, 200);
  assert!(last_sign_in(&router, &root, &member.id).await.is_string());

  // Confirming a session or using a token is not signing in.
  forget_sign_in(&state, &member.id).await;
  let (status, body) = send(
    &router,
    "POST",
    "/api/v1/auth/reauthenticate",
    member.browser(),
    Some(json!({"password":"correct-horse-123"})),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  let (_, body) = send(
    &router,
    "POST",
    "/api/v1/users/me/tokens",
    member.browser(),
    Some(json!({"name":"laptop","level":"read"})),
  )
  .await;
  let token = body["data"]["plaintextToken"].as_str().unwrap().to_owned();
  let (status, _) = send(&router, "GET", "/api/v1/projects", As::Token(&token), None).await;
  assert_eq!(status, 200);
  assert!(last_sign_in(&router, &root, &member.id).await.is_null());

  // The CLI browser login issues a session, so it counts.
  let (status, device, _) = call(
    &router,
    "POST",
    "/api/v1/auth/cli/device",
    None,
    Some(json!({"hostname":"laptop"})),
  )
  .await;
  assert_eq!(status, 201, "{device}");
  let (status, body) = send(
    &router,
    "POST",
    "/api/v1/auth/cli/approve",
    member.browser(),
    Some(json!({"userCode":device["data"]["userCode"]})),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  let (status, body, _) = call(
    &router,
    "POST",
    "/api/v1/auth/cli/token",
    None,
    Some(json!({"deviceCode":device["data"]["deviceCode"]})),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["status"], "approved");
  assert!(last_sign_in(&router, &root, &member.id).await.is_string());
  state.db.close().await;
}

#[tokio::test]
async fn users_list_the_groups_they_belong_to() {
  let (_dir, state, router) = test_app().await;
  let (root, _) = instance(&state, &router).await;
  let member = person(&router, &root, "member@example.com", "member").await;
  let mut ids = Vec::new();
  for name in ["Ops", "Backend"] {
    let (status, body) = send(
      &router,
      "POST",
      "/api/v1/groups",
      root.browser(),
      Some(json!({"name":name})),
    )
    .await;
    assert_eq!(status, 201, "{body}");
    let id = body["data"]["group"]["id"].as_str().unwrap().to_owned();
    let (status, _) = send(
      &router,
      "PUT",
      &format!("/api/v1/groups/{id}/members/{}", member.id),
      root.browser(),
      None,
    )
    .await;
    assert_eq!(status, 200);
    ids.push(id);
  }
  let (status, body) = send(&router, "GET", "/api/v1/users", root.browser(), None).await;
  assert_eq!(status, 200, "{body}");
  let users = body["data"].as_array().unwrap();
  let listed = users.iter().find(|user| user["id"] == member.id).unwrap();
  assert_eq!(
    listed["groups"],
    json!([{"id":ids[1],"name":"Backend"},{"id":ids[0],"name":"Ops"}])
  );
  let root_row = users.iter().find(|user| user["id"] == root.id).unwrap();
  assert_eq!(root_row["groups"], json!([]));
  let (_, body) = send(
    &router,
    "GET",
    &format!("/api/v1/users/{}", member.id),
    root.browser(),
    None,
  )
  .await;
  assert_eq!(body["data"]["groups"], listed["groups"]);
  state.db.close().await;
}
