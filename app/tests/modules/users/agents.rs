use super::audit_scope::with_role;
use super::grants_access::{As, Person, grant, instance, person, send};
use super::*;

/// Creates an AI agent as `by` and returns its id.
async fn agent(
  router: &Router,
  by: &Person,
  name: &str,
) -> String {
  let (status, body) = send(
    router,
    "POST",
    "/api/v1/users",
    by.browser(),
    Some(json!({"email":name,"role":"ai_agent"})),
  )
  .await;
  assert_eq!(status, 201, "{body}");
  assert_eq!(body["data"]["ownerId"], by.id);
  body["data"]["id"].as_str().unwrap().to_owned()
}

/// The emails of the accounts `who` may list.
async fn listed(
  router: &Router,
  who: &Person,
) -> Vec<String> {
  let (status, body) = send(router, "GET", "/api/v1/users", who.browser(), None).await;
  assert_eq!(status, 200, "{body}");
  body["data"]
    .as_array()
    .unwrap()
    .iter()
    .map(|user| user["email"].as_str().unwrap().to_owned())
    .collect()
}

async fn custom_role(
  router: &Router,
  root: &Person,
  name: &str,
  capabilities: &[&str],
) -> String {
  custom_role_at(router, root, name, capabilities, "write").await
}

async fn custom_role_at(
  router: &Router,
  root: &Person,
  name: &str,
  capabilities: &[&str],
  max_level: &str,
) -> String {
  let (status, body) = send(
    router,
    "POST",
    "/api/v1/roles",
    root.browser(),
    Some(json!({"name":name,"capabilities":capabilities,"maxLevel":max_level})),
  )
  .await;
  assert_eq!(status, 201, "{body}");
  body["data"]["id"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn a_member_gives_their_own_agent_no_more_than_they_reach() {
  let (_dir, state, router) = test_app().await;
  let (root, env) = instance(&state, &router).await;
  // Three projects: payments and search from the instance, and billing.
  let (status, body) = send(
    &router,
    "POST",
    "/api/v1/projects",
    root.cli(),
    Some(json!({"name":"billing"})),
  )
  .await;
  assert_eq!(status, 201, "{body}");
  let maya = person(&router, &root, "maya@example.com", "member").await;
  grant(
    &router,
    &root,
    ("user", &maya.id),
    ("project", Some("payments")),
    "write",
  )
  .await;

  // Maya creates an agent and owns it; it is the only account she sees.
  let bot = agent(&router, &maya, "ci-bot").await;
  assert_eq!(listed(&router, &maya).await, ["ci-bot"]);
  let (_, user) = send(
    &router,
    "GET",
    &format!("/api/v1/users/{bot}"),
    maya.browser(),
    None,
  )
  .await;
  assert_eq!(user["data"]["ownerEmail"], "maya@example.com");

  // On payments she gives it Names only, Read or Write, never more than
  // her own Write.
  for level in ["metadata", "read", "write"] {
    let (status, body) = grant(
      &router,
      &maya,
      ("user", &bot),
      ("project", Some("payments")),
      level,
    )
    .await;
    assert_eq!(status, 200, "{level}: {body}");
  }
  for level in ["manage", "project_manager"] {
    let (status, body) = grant(
      &router,
      &maya,
      ("user", &bot),
      ("project", Some("payments")),
      level,
    )
    .await;
    assert_eq!(status, 403, "{level}: {body}");
  }
  // The other two projects don't exist for her.
  for project in ["search", "billing"] {
    let (status, body) = grant(
      &router,
      &maya,
      ("user", &bot),
      ("project", Some(project)),
      "read",
    )
    .await;
    assert_eq!(status, 404, "{project}: {body}");
  }
  // Nor does all projects.
  let (status, _) = grant(&router, &maya, ("user", &bot), ("all", None), "read").await;
  assert_eq!(status, 403);

  // Once prod is protected her project Write stops there, so she can't
  // give it to the agent either; dev still works.
  let prod = &env["payments/prod"];
  let (status, body) = send(
    &router,
    "PATCH",
    &format!("/api/v1/environments/{prod}"),
    root.cli(),
    Some(json!({"protected":true})),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  let (status, _) = grant(
    &router,
    &maya,
    ("user", &bot),
    ("environment", Some(prod)),
    "read",
  )
  .await;
  assert_eq!(status, 404);
  let (status, body) = grant(
    &router,
    &maya,
    ("user", &bot),
    ("environment", Some(&env["payments/dev"])),
    "read",
  )
  .await;
  assert_eq!(status, 200, "{body}");

  // She reads the agent's access, gives it a token, and the token reaches
  // payments only.
  let (status, access) = send(
    &router,
    "GET",
    &format!("/api/v1/users/{bot}/access"),
    maya.browser(),
    None,
  )
  .await;
  assert_eq!(status, 200, "{access}");
  assert_eq!(access["data"]["grants"].as_array().unwrap().len(), 2);
  // Maya backs both grants in full, so neither carries an owner level.
  assert_eq!(access["data"]["ownerEmail"], "maya@example.com");
  for grant in access["data"]["grants"].as_array().unwrap() {
    assert!(grant.get("ownerLevel").is_none(), "{grant}");
  }
  let (status, body) = send(
    &router,
    "POST",
    &format!("/api/v1/users/{bot}/tokens"),
    maya.browser(),
    Some(json!({"name":"ci","level":"read"})),
  )
  .await;
  assert_eq!(status, 201, "{body}");
  let token = body["data"]["plaintextToken"].as_str().unwrap().to_owned();
  let token_id = body["data"]["token"]["id"].as_str().unwrap().to_owned();
  let runtime = |id: &str| format!("/api/v1/environments/{id}/secrets/runtime");
  let (status, _) = send(
    &router,
    "GET",
    &runtime(&env["payments/dev"]),
    As::Token(&token),
    None,
  )
  .await;
  assert_eq!(status, 200);
  let (status, _) = send(
    &router,
    "GET",
    &runtime(&env["search/dev"]),
    As::Token(&token),
    None,
  )
  .await;
  assert_eq!(status, 404);

  // Cutting Maya to Names only cuts the agent too, though its grant says
  // Write.
  grant(
    &router,
    &root,
    ("user", &maya.id),
    ("project", Some("payments")),
    "metadata",
  )
  .await;
  let (status, _) = send(
    &router,
    "GET",
    &runtime(&env["payments/dev"]),
    As::Token(&token),
    None,
  )
  .await;
  assert_eq!(status, 403);
  // The access report says so: each grant shows the owner's lower level.
  let (status, capped) = send(
    &router,
    "GET",
    &format!("/api/v1/users/{bot}/access"),
    maya.browser(),
    None,
  )
  .await;
  assert_eq!(status, 200, "{capped}");
  for grant in capped["data"]["grants"].as_array().unwrap() {
    assert_eq!(grant["ownerLevel"], "metadata", "{grant}");
  }

  // She renames, disables and enables it, revokes its token and takes its
  // access away, but can't hand it to someone else.
  let item = format!("/api/v1/users/{bot}");
  let (status, body) = send(
    &router,
    "PATCH",
    &item,
    maya.browser(),
    Some(json!({"email":"deploy-bot"})),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  for action in ["disable", "enable"] {
    let (status, body) = send(
      &router,
      "POST",
      &format!("{item}/{action}"),
      maya.browser(),
      None,
    )
    .await;
    assert_eq!(status, 200, "{action}: {body}");
  }
  let (status, body) = send(
    &router,
    "POST",
    &format!("{item}/tokens/{token_id}/revoke"),
    maya.browser(),
    None,
  )
  .await;
  assert_eq!(status, 200, "{body}");
  let other = person(&router, &root, "other@example.com", "member").await;
  let (status, body) = send(
    &router,
    "PATCH",
    &item,
    maya.browser(),
    Some(json!({"ownerId":other.id})),
  )
  .await;
  assert_eq!(status, 403, "{body}");
  let grant_id = access["data"]["grants"][0]["id"].as_str().unwrap();
  let (status, body) = send(
    &router,
    "DELETE",
    &format!("/api/v1/grants/{grant_id}"),
    maya.browser(),
    None,
  )
  .await;
  assert_eq!(status, 200, "{body}");

  // Every grant she gave is in the audit log under her.
  let given: i64 = sqlx::query_scalar(
    "SELECT COUNT(*) FROM audit_events WHERE action = 'grant.set' AND actor_id = ?",
  )
  .bind(&maya.id)
  .fetch_one(state.db.pool())
  .await
  .unwrap();
  assert_eq!(given, 4);

  let (status, _) = send(&router, "DELETE", &item, maya.browser(), None).await;
  assert_eq!(status, 200);
  state.db.close().await;
}

#[tokio::test]
async fn an_owner_reaches_only_their_own_agents_and_never_people() {
  let (_dir, state, router) = test_app().await;
  let (root, _) = instance(&state, &router).await;
  let maya = person(&router, &root, "maya@example.com", "member").await;
  grant(
    &router,
    &root,
    ("user", &maya.id),
    ("project", Some("payments")),
    "write",
  )
  .await;
  let sam = person(&router, &root, "sam@example.com", "admin").await;
  let theirs = agent(&router, &sam, "sam-bot").await;
  let item = format!("/api/v1/users/{theirs}");

  // Someone else's agent, and every person, doesn't exist for her.
  for id in [theirs.as_str(), sam.id.as_str()] {
    let (status, _) = send(
      &router,
      "GET",
      &format!("/api/v1/users/{id}"),
      maya.browser(),
      None,
    )
    .await;
    assert_eq!(status, 404, "{id}");
  }
  // She can't change it, read or make its tokens, or give it access, even
  // on payments where she has Write: only a project manager grants to
  // agents she doesn't own.
  for (method, path, body) in [
    ("PATCH", item.clone(), Some(json!({"email":"mine-now"}))),
    ("POST", format!("{item}/disable"), None),
    ("DELETE", item.clone(), None),
    ("GET", format!("{item}/tokens"), None),
    (
      "POST",
      format!("{item}/tokens"),
      Some(json!({"name":"x","level":"read"})),
    ),
    ("GET", format!("{item}/access"), None),
  ] {
    let (status, response) = send(&router, method, &path, maya.browser(), body).await;
    assert_eq!(status, 403, "{method} {path}: {response}");
  }
  let (status, _) = grant(
    &router,
    &maya,
    ("user", &theirs),
    ("project", Some("payments")),
    "read",
  )
  .await;
  assert_eq!(status, 403);
  // Nor does agents.view add people.
  let (status, body) = send(
    &router,
    "POST",
    "/api/v1/users",
    maya.browser(),
    Some(json!({"email":"new@example.com","password":"correct-horse-123","role":"member"})),
  )
  .await;
  assert_eq!(status, 403, "{body}");
  // And a CLI session can't create agents.
  let (status, _) = send(
    &router,
    "POST",
    "/api/v1/users",
    maya.cli(),
    Some(json!({"email":"cli-bot","role":"ai_agent"})),
  )
  .await;
  assert_eq!(status, 403);
  state.db.close().await;
}

#[tokio::test]
async fn seeing_every_agent_and_managing_them_are_separate_capabilities() {
  let (_dir, state, router) = test_app().await;
  let (root, _) = instance(&state, &router).await;
  let maya = person(&router, &root, "maya@example.com", "member").await;
  let bot = agent(&router, &maya, "maya-bot").await;
  let item = format!("/api/v1/users/{bot}");

  // agents.view_all sees every agent and its owner, but no people, and
  // changes nothing.
  let lead_role = custom_role(
    &router,
    &root,
    "Agent lead",
    &["tokens.personal", "agents.view", "agents.view_all"],
  )
  .await;
  let lead = with_role(&router, &root, "lead@example.com", &lead_role).await;
  assert_eq!(listed(&router, &lead).await, ["maya-bot"]);
  let (_, user) = send(&router, "GET", &item, lead.browser(), None).await;
  assert_eq!(user["data"]["ownerEmail"], "maya@example.com");
  for path in [format!("{item}/access"), format!("{item}/tokens")] {
    let (status, body) = send(&router, "GET", &path, lead.browser(), None).await;
    assert_eq!(status, 200, "{path}: {body}");
  }
  for (method, path, body) in [
    ("PATCH", item.clone(), Some(json!({"email":"lead-bot"}))),
    ("POST", format!("{item}/disable"), None),
    (
      "POST",
      format!("{item}/tokens"),
      Some(json!({"name":"x","level":"read"})),
    ),
    ("DELETE", item.clone(), None),
  ] {
    let (status, response) = send(&router, method, &path, lead.browser(), body).await;
    assert_eq!(status, 403, "{method} {path}: {response}");
  }

  // users.view and users.manage no longer reach agents.
  let people_role = custom_role(
    &router,
    &root,
    "People admin",
    &["users.view", "users.manage"],
  )
  .await;
  // The upgrade gives custom roles agent capabilities; a role made now
  // has only what it is given.
  let people = with_role(&router, &root, "people@example.com", &people_role).await;
  assert!(
    !listed(&router, &people)
      .await
      .contains(&"maya-bot".to_owned())
  );
  let (status, _) = send(&router, "GET", &item, people.browser(), None).await;
  assert_eq!(status, 404);
  let (status, body) = send(
    &router,
    "PATCH",
    &item,
    people.browser(),
    Some(json!({"email":"people-bot"})),
  )
  .await;
  assert_eq!(status, 403, "{body}");
  let (status, body) = send(
    &router,
    "POST",
    "/api/v1/users",
    people.browser(),
    Some(json!({"email":"people-bot","role":"ai_agent"})),
  )
  .await;
  assert_eq!(status, 403, "{body}");

  // agents.manage changes and transfers anyone's agent. The new owner must
  // hold agents.view: the lead does, an auditor doesn't.
  let sam = person(&router, &root, "sam@example.com", "admin").await;
  let (status, body) = send(
    &router,
    "PATCH",
    &item,
    sam.browser(),
    Some(json!({"email":"renamed-bot"})),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  let auditor = with_role(&router, &root, "auditor@example.com", "rol_auditor").await;
  let (status, body) = send(
    &router,
    "PATCH",
    &item,
    sam.browser(),
    Some(json!({"ownerId":auditor.id})),
  )
  .await;
  assert_eq!(status, 400, "{body}");
  assert!(body["error"]["OWNER_INVALID"].is_string());
  let (status, body) = send(
    &router,
    "PATCH",
    &item,
    sam.browser(),
    Some(json!({"ownerId":lead.id})),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["ownerEmail"], "lead@example.com");
  // It is the lead's now, so Maya no longer sees it.
  assert!(listed(&router, &maya).await.is_empty());
  let (status, body) = send(
    &router,
    "POST",
    &format!("{item}/tokens"),
    sam.browser(),
    Some(json!({"name":"ops","level":"read"})),
  )
  .await;
  assert_eq!(status, 201, "{body}");
  state.db.close().await;
}

#[tokio::test]
async fn built_in_roles_hold_the_agent_capabilities_they_should() {
  let (_dir, state, router) = test_app().await;
  let (root, _) = instance(&state, &router).await;
  for (role, expected) in [
    (
      "rol_admin",
      ["agents.manage", "agents.view", "agents.view_all"].as_slice(),
    ),
    ("rol_member", &["agents.view"]),
    ("rol_auditor", &["agents.view_all"]),
  ] {
    let who = with_role(&router, &root, &format!("{role}@example.com"), role).await;
    let (status, session) = send(&router, "GET", "/api/v1/auth/session", who.browser(), None).await;
    assert_eq!(status, 200, "{session}");
    let mut held: Vec<String> = session["data"]["capabilities"]
      .as_array()
      .unwrap()
      .iter()
      .filter_map(|capability| capability.as_str())
      .filter(|capability| capability.starts_with("agents."))
      .map(str::to_owned)
      .collect();
    held.sort();
    assert_eq!(held, expected, "{role}");
  }
  state.db.close().await;
}

#[tokio::test]
async fn only_agents_manage_gives_someone_elses_agent_access() {
  let (_dir, state, router) = test_app().await;
  let (root, _) = instance(&state, &router).await;
  let maya = person(&router, &root, "maya@example.com", "member").await;
  let bot = agent(&router, &maya, "maya-bot").await;

  // Both manage payments; one holds users.*, the other agents.manage.
  let people_role = custom_role_at(
    &router,
    &root,
    "People admin",
    &["users.view", "users.manage"],
    "project_manager",
  )
  .await;
  let lead_role = custom_role_at(
    &router,
    &root,
    "Agent lead",
    &["tokens.personal", "agents.view_all", "agents.manage"],
    "project_manager",
  )
  .await;
  let people = with_role(&router, &root, "people@example.com", &people_role).await;
  let lead = with_role(&router, &root, "lead@example.com", &lead_role).await;
  for who in [&people, &lead] {
    let (status, body) = grant(
      &router,
      &root,
      ("user", &who.id),
      ("project", Some("payments")),
      "project_manager",
    )
    .await;
    assert_eq!(status, 200, "{body}");
  }

  // users.view doesn't reach the agent; agents.manage does.
  let (status, body) = grant(
    &router,
    &people,
    ("user", &bot),
    ("project", Some("payments")),
    "read",
  )
  .await;
  assert_eq!(status, 422, "{body}");
  assert!(
    body["error"]["GRANT_PRINCIPAL_INVALID"].is_string(),
    "{body}"
  );
  let (status, body) = grant(
    &router,
    &lead,
    ("user", &bot),
    ("project", Some("payments")),
    "read",
  )
  .await;
  assert_eq!(status, 200, "{body}");
  // agents.manage doesn't reach people, and the people admin still does.
  let (status, body) = grant(
    &router,
    &lead,
    ("user", &maya.id),
    ("project", Some("payments")),
    "read",
  )
  .await;
  assert_eq!(status, 422, "{body}");
  assert!(
    body["error"]["GRANT_PRINCIPAL_INVALID"].is_string(),
    "{body}"
  );
  let (status, body) = grant(
    &router,
    &people,
    ("user", &maya.id),
    ("project", Some("payments")),
    "read",
  )
  .await;
  assert_eq!(status, 200, "{body}");
  state.db.close().await;
}

#[tokio::test]
async fn agents_manage_hands_agents_over_but_creates_none() {
  let (_dir, state, router) = test_app().await;
  let (root, _) = instance(&state, &router).await;
  let maya = person(&router, &root, "maya@example.com", "member").await;
  let other = person(&router, &root, "other@example.com", "member").await;
  with_role(&router, &root, "auditor@example.com", "rol_auditor").await;
  let bot = agent(&router, &maya, "maya-bot").await;
  let lead_role = custom_role(
    &router,
    &root,
    "Agent lead",
    &["tokens.personal", "agents.view_all", "agents.manage"],
  )
  .await;
  let lead = with_role(&router, &root, "lead@example.com", &lead_role).await;

  // Whoever creates an agent owns it, and owning one takes agents.view.
  let (status, body) = send(
    &router,
    "POST",
    "/api/v1/users",
    lead.browser(),
    Some(json!({"email":"lead-bot","role":"ai_agent"})),
  )
  .await;
  assert_eq!(status, 403, "{body}");

  // The lead sees the people who could own an agent, so the console can
  // offer them, but not the ones who couldn't.
  let seen = listed(&router, &lead).await;
  for email in ["maya@example.com", "other@example.com", "maya-bot"] {
    assert!(seen.contains(&email.to_owned()), "{email}: {seen:?}");
  }
  assert!(
    !seen.contains(&"auditor@example.com".to_owned()),
    "{seen:?}"
  );
  // Maya gave her agent a token before it changes hands.
  let (status, body) = send(
    &router,
    "POST",
    &format!("/api/v1/users/{bot}/tokens"),
    maya.browser(),
    Some(json!({"name":"ci","level":"read"})),
  )
  .await;
  assert_eq!(status, 201, "{body}");
  let token = body["data"]["plaintextToken"].as_str().unwrap().to_owned();
  let me = "/api/v1/users/me/access";
  // Naming the same owner again hands nothing over, so the token lives.
  let (status, body) = send(
    &router,
    "PATCH",
    &format!("/api/v1/users/{bot}"),
    lead.browser(),
    Some(json!({"ownerId":maya.id})),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  let (status, _) = send(&router, "GET", me, As::Token(&token), None).await;
  assert_eq!(status, 200);

  let (status, body) = send(
    &router,
    "PATCH",
    &format!("/api/v1/users/{bot}"),
    lead.browser(),
    Some(json!({"ownerId":other.id})),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["ownerEmail"], "other@example.com");
  // A new owner starts with no tokens, so Maya's copy stops working.
  let (status, _) = send(&router, "GET", me, As::Token(&token), None).await;
  assert_eq!(status, 401);
  let (status, body) = send(
    &router,
    "GET",
    &format!("/api/v1/users/{bot}/tokens"),
    other.browser(),
    None,
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert!(body["data"][0]["revokedAt"].is_string(), "{body}");
  let revoked: i64 = sqlx::query_scalar(
    "SELECT COUNT(*) FROM audit_events WHERE action = 'personal_token.revoked' AND actor_id = ? AND metadata LIKE '%owner_changed%'",
  )
  .bind(&lead.id)
  .fetch_one(state.db.pool())
  .await
  .unwrap();
  assert_eq!(revoked, 1);
  state.db.close().await;
}

#[tokio::test]
async fn an_owner_doesnt_undo_a_disable_by_agents_manage() {
  let (_dir, state, router) = test_app().await;
  let (root, _) = instance(&state, &router).await;
  let maya = person(&router, &root, "maya@example.com", "member").await;
  let sam = person(&router, &root, "sam@example.com", "admin").await;
  let bot = agent(&router, &maya, "maya-bot").await;
  let item = format!("/api/v1/users/{bot}");
  // Sam turns it off, and it stays off until agents.manage turns it on.
  // Her own disable she undoes herself.
  for (who, action, expected) in [
    (&sam, "disable", 200),
    (&maya, "enable", 403),
    (&sam, "enable", 200),
    (&maya, "disable", 200),
    (&maya, "enable", 200),
  ] {
    let (status, body) = send(
      &router,
      "POST",
      &format!("{item}/{action}"),
      who.browser(),
      None,
    )
    .await;
    assert_eq!(status, expected, "{action}: {body}");
  }
  state.db.close().await;
}

#[tokio::test]
async fn an_agent_owner_cant_probe_other_accounts() {
  let (_dir, state, router) = test_app().await;
  let (root, _) = instance(&state, &router).await;
  let maya = person(&router, &root, "maya@example.com", "member").await;
  let sam = person(&router, &root, "sam@example.com", "admin").await;
  agent(&router, &maya, "maya-bot").await;

  // Root, a person and a missing id answer alike, whatever she asks.
  for id in [root.id.as_str(), sam.id.as_str(), "usr_missing"] {
    let item = format!("/api/v1/users/{id}");
    for (method, path, body) in [
      (
        "PATCH",
        item.clone(),
        Some(json!({"email":"x@example.com"})),
      ),
      ("POST", format!("{item}/disable"), None),
      ("POST", format!("{item}/enable"), None),
      ("DELETE", item.clone(), None),
    ] {
      let (status, response) = send(&router, method, &path, maya.browser(), body).await;
      assert_eq!(status, 403, "{method} {path}: {response}");
      assert!(
        response["error"]["AUTHORIZATION_DENIED"].is_string(),
        "{method} {path}: {response}"
      );
    }
  }
  state.db.close().await;
}
