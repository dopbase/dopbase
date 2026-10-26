use super::audit_scope::with_role;
use super::grants_access::{grant, instance, person, send};
use super::*;

#[tokio::test]
async fn root_shapes_custom_roles_that_others_give_within_their_own() {
  let (_dir, state, router) = test_app().await;
  let (root, env) = instance(&state, &router).await;
  let lead = person(&router, &root, "lead@example.com", "admin").await;

  // Only root creates roles, and never with root's own powers.
  let viewer = json!({"name":"Viewer","capabilities":["tokens.personal"],"maxLevel":"read"});
  let (status, _) = send(
    &router,
    "POST",
    "/api/v1/roles",
    lead.browser(),
    Some(viewer.clone()),
  )
  .await;
  assert_eq!(status, 403);
  for (body, code) in [
    (
      json!({"name":"Bad","capabilities":["instance.reset"],"maxLevel":"read"}),
      "ROLE_CAPABILITY_INVALID",
    ),
    (
      json!({"name":"Bad","maxLevel":"read","tokenScope":"data"}),
      "ROLE_TOKENS_INVALID",
    ),
    (
      json!({"name":"Bad","capabilities":["tokens.personal"],"maxLevel":"metadata","tokenMaxLevel":"write"}),
      "ROLE_TOKENS_INVALID",
    ),
  ] {
    let (status, response) =
      send(&router, "POST", "/api/v1/roles", root.browser(), Some(body)).await;
    assert_eq!(status, 422, "{response}");
    assert!(response["error"][code].is_string(), "{response}");
  }
  let (status, body) = send(
    &router,
    "POST",
    "/api/v1/roles",
    root.browser(),
    Some(viewer),
  )
  .await;
  assert_eq!(status, 201, "{body}");
  let viewer_id = body["data"]["id"].as_str().unwrap().to_owned();
  assert_eq!(body["data"]["tokenMaxLevel"], "read");

  // The lead may give Viewer, which is within an admin, but not Auditor,
  // which reads every audit event.
  let (_, roles) = send(&router, "GET", "/api/v1/roles", lead.cli(), None).await;
  let assignable = |id: &str| {
    roles["data"]
      .as_array()
      .unwrap()
      .iter()
      .find(|role| role["id"] == id)
      .unwrap()["assignable"]
      .clone()
  };
  assert_eq!(assignable(&viewer_id), true);
  assert_eq!(assignable("rol_auditor"), false);
  let (status, body) = send(
    &router,
    "POST",
    "/api/v1/users",
    lead.browser(),
    Some(
      json!({"email":"audit@example.com","password":"correct-horse-123","roleId":"rol_auditor"}),
    ),
  )
  .await;
  assert_eq!(status, 403, "{body}");
  let reader = with_role(&router, &lead, "reader@example.com", &viewer_id).await;
  let (_, user) = send(
    &router,
    "GET",
    &format!("/api/v1/users/{}", reader.id),
    root.browser(),
    None,
  )
  .await;
  assert_eq!(user["data"]["roleName"], "Viewer");
  assert_eq!(user["data"]["roleId"], viewer_id);

  // Grants stay within the role's ceiling, and the ceiling applies live.
  let (status, _) = grant(
    &router,
    &root,
    ("user", &reader.id),
    ("project", Some("payments")),
    "write",
  )
  .await;
  assert_eq!(status, 422);
  grant(
    &router,
    &root,
    ("user", &reader.id),
    ("project", Some("payments")),
    "read",
  )
  .await;
  let runtime = format!(
    "/api/v1/environments/{}/secrets/runtime",
    env["payments/dev"]
  );
  let (status, _) = send(&router, "GET", &runtime, reader.cli(), None).await;
  assert_eq!(status, 200);
  let (status, body) = send(
    &router,
    "PATCH",
    &format!("/api/v1/roles/{viewer_id}"),
    root.browser(),
    Some(json!({"maxLevel":"metadata"})),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  let (status, _) = send(&router, "GET", &runtime, reader.cli(), None).await;
  assert_eq!(status, 403);

  // Built-in roles never change, and a role in use can't go.
  let (status, _) = send(
    &router,
    "PATCH",
    "/api/v1/roles/rol_member",
    root.browser(),
    Some(json!({"maxLevel":"read"})),
  )
  .await;
  assert_eq!(status, 403);
  let delete = format!("/api/v1/roles/{viewer_id}");
  let (status, _) = send(&router, "DELETE", &delete, root.browser(), None).await;
  assert_eq!(status, 409);
  let (status, _) = send(
    &router,
    "PATCH",
    &format!("/api/v1/users/{}", reader.id),
    root.browser(),
    Some(json!({"roleId":"rol_member"})),
  )
  .await;
  assert_eq!(status, 200);
  let (status, _) = send(&router, "DELETE", &delete, root.browser(), None).await;
  assert_eq!(status, 200);

  // A token-only custom role works like an AI agent: no password, an owner.
  let (status, body) = send(
    &router,
    "POST",
    "/api/v1/roles",
    root.browser(),
    Some(json!({"name":"CI bot","capabilities":["tokens.personal"],"maxLevel":"read","signIn":"token_only","tokenMaxTtlDays":30})),
  )
  .await;
  assert_eq!(status, 201, "{body}");
  assert_eq!(body["data"]["requiresOwner"], true);
  let bot_role = body["data"]["id"].as_str().unwrap().to_owned();
  let (status, body) = send(
    &router,
    "POST",
    "/api/v1/users",
    lead.browser(),
    Some(json!({"email":"deploy-bot","roleId":bot_role})),
  )
  .await;
  assert_eq!(status, 201, "{body}");
  assert_eq!(body["data"]["ownerId"], lead.id);
  assert_eq!(body["data"]["role"], "ai_agent");
  let bot = body["data"]["id"].as_str().unwrap().to_owned();
  let (status, body) = send(
    &router,
    "POST",
    &format!("/api/v1/users/{bot}/tokens"),
    lead.browser(),
    Some(json!({"name":"ci","level":"read","expiresIn":"31d"})),
  )
  .await;
  assert_eq!(status, 422, "{body}");
  state.db.close().await;
}
