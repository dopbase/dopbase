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

/// Starts a browser CLI login and has `who` approve it. Returns the device
/// code the CLI would poll with.
async fn approved_cli_login(
  router: &Router,
  who: &Person,
) -> String {
  let (status, body, _) = call(
    router,
    "POST",
    "/api/v1/auth/cli/device",
    None,
    Some(json!({})),
  )
  .await;
  assert_eq!(status, 201, "{body}");
  let (status, approval) = send(
    router,
    "POST",
    "/api/v1/auth/cli/approve",
    who.browser(),
    Some(json!({"userCode":body["data"]["userCode"]})),
  )
  .await;
  assert_eq!(status, 200, "{approval}");
  body["data"]["deviceCode"].as_str().unwrap().to_owned()
}

/// Polls like the CLI and expects the request to be refused as used up.
async fn poll_refused(
  router: &Router,
  device_code: &str,
) {
  let (status, body, _) = call(
    router,
    "POST",
    "/api/v1/auth/cli/token",
    None,
    Some(json!({"deviceCode":device_code})),
  )
  .await;
  assert_eq!(status, 400, "{body}");
  assert!(body["error"]["DEVICE_CODE_INVALID"].is_string(), "{body}");
}

async fn live_sessions(
  state: &app::state::AppState,
  user: &str,
) -> i64 {
  sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE admin_id = ? AND revoked_at IS NULL")
    .bind(user)
    .fetch_one(state.db.pool())
    .await
    .unwrap()
}

#[tokio::test]
async fn a_cli_login_approved_before_disabling_issues_no_session() {
  let (_dir, state, router) = test_app().await;
  let (root, _) = instance(&state, &router).await;
  let admin = person(&router, &root, "lead@example.com", "admin").await;
  let member = person(&router, &root, "member@example.com", "member").await;

  // Disabling cancels the approval before the CLI picks it up.
  let device_code = approved_cli_login(&router, &member).await;
  forget_sign_in(&state, &member.id).await;
  let (status, body) = set_status(&router, &admin, &member.id, "disable").await;
  assert_eq!(status, 200, "{body}");
  poll_refused(&router, &device_code).await;
  assert_eq!(live_sessions(&state, &member.id).await, 0);
  assert!(last_sign_in(&router, &admin, &member.id).await.is_null());
  // `person` signed in with a password; only a browser login would count.
  let browser_logins: i64 = sqlx::query_scalar(
    "SELECT COUNT(*) FROM audit_events WHERE action = 'login.succeeded' AND actor_id = ? AND json_extract(metadata, '$.method') = 'browser'",
  )
  .bind(&member.id)
  .fetch_one(state.db.pool())
  .await
  .unwrap();
  assert_eq!(browser_logins, 0);

  // An approval still standing when the account is disabled some other
  // way issues nothing either.
  let (status, _) = set_status(&router, &admin, &member.id, "enable").await;
  assert_eq!(status, 200);
  let member = super::grants_access::sign_in(&router, "member@example.com").await;
  let device_code = approved_cli_login(&router, &member).await;
  sqlx::query("UPDATE admins SET status = 'disabled' WHERE id = ?")
    .bind(&member.id)
    .execute(state.db.pool())
    .await
    .unwrap();
  let before = live_sessions(&state, &member.id).await;
  poll_refused(&router, &device_code).await;
  assert_eq!(live_sessions(&state, &member.id).await, before);
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

#[tokio::test]
async fn user_identities_carry_only_id_and_email() {
  let (_dir, state, router) = test_app().await;
  let (root, _) = instance(&state, &router).await;
  let member = person(&router, &root, "member@example.com", "member").await;
  let (status, body) = send(
    &router,
    "GET",
    "/api/v1/users/identities",
    root.browser(),
    None,
  )
  .await;
  assert_eq!(status, 200, "{body}");
  let root_email = body["data"]
    .as_array()
    .unwrap()
    .iter()
    .find(|user| user["id"] == root.id)
    .unwrap()["email"]
    .clone();
  let mut expected = vec![
    json!({"id":member.id,"email":"member@example.com"}),
    json!({"id":root.id,"email":root_email}),
  ];
  expected.sort_by_key(|user| user["email"].as_str().unwrap().to_owned());
  assert_eq!(body["data"], json!(expected));
  // Same gate as the full list: a browser session that can view users.
  let (status, _) = send(
    &router,
    "GET",
    "/api/v1/users/identities",
    member.browser(),
    None,
  )
  .await;
  assert_eq!(status, 403);
  let (status, _) = send(&router, "GET", "/api/v1/users/identities", root.cli(), None).await;
  assert_eq!(status, 403);
  state.db.close().await;
}

#[tokio::test]
async fn without_rbac_people_stay_as_they_are_but_can_be_offboarded() {
  use app::services::features::Features;
  let (_dir, state, router) = test_app().await;
  let (root, env) = instance(&state, &router).await;
  let admin = person(&router, &root, "lead@example.com", "admin").await;
  let member = person(&router, &root, "member@example.com", "member").await;
  grant(
    &router,
    &root,
    ("user", &member.id),
    ("project", Some("payments")),
    "read",
  )
  .await;
  let off = with_features(
    &state,
    Features {
      rbac: false,
      ..Features::all()
    },
  );

  // Nobody adds or changes people, root included.
  for by in [&root, &admin] {
    let (status, body) = send(
      &off,
      "POST",
      "/api/v1/users",
      by.browser(),
      Some(json!({"email":"new@example.com","password":"correct-horse-123","role":"member"})),
    )
    .await;
    feature_disabled(status, &body, "DOPBASE_RBAC=true");
  }
  for change in [
    json!({"role":"admin"}),
    json!({"email":"renamed@example.com"}),
  ] {
    let (status, body) = send(
      &off,
      "PATCH",
      &format!("/api/v1/users/{}", member.id),
      admin.browser(),
      Some(change),
    )
    .await;
    feature_disabled(status, &body, "DOPBASE_RBAC=true");
  }

  // Existing people keep signing in with their role and grants.
  assert_eq!(password_login(&off, "member@example.com").await, 200);
  let runtime = format!(
    "/api/v1/environments/{}/secrets/runtime",
    env["payments/dev"]
  );
  let (status, body) = send(&off, "GET", &runtime, member.cli(), None).await;
  assert_eq!(status, 200, "{body}");
  let (status, session) = send(&off, "GET", "/api/v1/auth/session", admin.browser(), None).await;
  assert_eq!(status, 200, "{session}");
  assert_eq!(session["data"]["roleName"], "Admin");
  assert_eq!(session["data"]["features"], json!(["sso"]));
  let capabilities = session["data"]["capabilities"].as_array().unwrap();
  assert!(!capabilities.contains(&json!("groups.manage")), "{session}");
  assert!(capabilities.contains(&json!("users.manage")), "{session}");

  // AI agents and offboarding keep working.
  let (status, body) = send(
    &off,
    "POST",
    "/api/v1/users",
    admin.browser(),
    Some(json!({"email":"deploy-bot","role":"ai_agent"})),
  )
  .await;
  assert_eq!(status, 201, "{body}");
  let (status, body) = set_status(&off, &admin, &member.id, "disable").await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(password_login(&off, "member@example.com").await, 401);
  let (status, body) = set_status(&off, &admin, &member.id, "enable").await;
  assert_eq!(status, 200, "{body}");
  let (status, body) = send(
    &off,
    "DELETE",
    &format!("/api/v1/users/{}", member.id),
    admin.browser(),
    None,
  )
  .await;
  assert!(status < 300, "{status} {body}");
}
