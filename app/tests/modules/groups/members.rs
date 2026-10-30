use super::grants_access::{grant, instance, names, person, send};
use super::*;

#[tokio::test]
async fn group_members_reach_what_the_group_is_granted() {
  let (_dir, state, router) = test_app().await;
  let (root, env) = instance(&state, &router).await;
  let admin = person(&router, &root, "lead@example.com", "admin").await;
  let member = person(&router, &root, "member@example.com", "member").await;

  let (status, body) = send(
    &router,
    "POST",
    "/api/v1/groups",
    admin.browser(),
    Some(json!({"name":"Backend","description":"Payments team"})),
  )
  .await;
  assert_eq!(status, 201, "{body}");
  let group = body["data"]["group"]["id"].as_str().unwrap().to_owned();
  let (status, _) = send(
    &router,
    "POST",
    "/api/v1/groups",
    admin.browser(),
    Some(json!({"name":"backend"})),
  )
  .await;
  assert_eq!(status, 409);

  let members = format!("/api/v1/groups/{group}/members");
  let (status, body) = send(
    &router,
    "PUT",
    &format!("{members}/{}", member.id),
    admin.browser(),
    None,
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["group"]["memberCount"], 1);
  let (status, body) = send(
    &router,
    "PUT",
    &format!("{members}/{}", root.id),
    admin.browser(),
    None,
  )
  .await;
  assert_eq!(status, 403, "{body}");

  // Joining gives nothing until the group has grants.
  assert!(names(&router, &member, "/api/v1/projects").await.is_empty());
  let (status, body) = grant(
    &router,
    &admin,
    ("group", &group),
    ("project", Some("payments")),
    "read",
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["principalLabel"], "Backend");
  assert_eq!(
    names(&router, &member, "/api/v1/environments").await,
    ["payments/dev", "payments/prod"]
  );
  let (status, _) = send(
    &router,
    "GET",
    &format!(
      "/api/v1/environments/{}/secrets/runtime",
      env["payments/dev"]
    ),
    member.cli(),
    None,
  )
  .await;
  assert_eq!(status, 200);
  let (_, body) = send(
    &router,
    "GET",
    &format!("/api/v1/users/{}/access", member.id),
    member.cli(),
    None,
  )
  .await;
  assert_eq!(body["data"]["groups"][0]["name"], "Backend");
  assert_eq!(body["data"]["grants"][0]["principalType"], "group");

  // Members can't see or change groups.
  let (status, _) = send(&router, "GET", "/api/v1/groups", member.browser(), None).await;
  assert_eq!(status, 403);

  // Leaving the group takes the access with it on the next request.
  let (status, _) = send(
    &router,
    "DELETE",
    &format!("{members}/{}", member.id),
    admin.browser(),
    None,
  )
  .await;
  assert_eq!(status, 200);
  assert!(names(&router, &member, "/api/v1/projects").await.is_empty());
  let (status, _) = send(
    &router,
    "DELETE",
    &format!("{members}/{}", member.id),
    admin.browser(),
    None,
  )
  .await;
  assert_eq!(status, 404);

  // Once root grants the group a project the admin doesn't manage, the
  // group is out of the admin's hands.
  let (_, access) = send(
    &router,
    "GET",
    &format!("/api/v1/users/{}/access", admin.id),
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
    ("user", &admin.id),
    ("project", Some("payments")),
    "project_manager",
  )
  .await;
  let (status, _) = grant(
    &router,
    &root,
    ("group", &group),
    ("project", Some("search")),
    "read",
  )
  .await;
  assert_eq!(status, 200);
  let (status, body) = send(
    &router,
    "PUT",
    &format!("{members}/{}", member.id),
    admin.browser(),
    None,
  )
  .await;
  assert_eq!(status, 403, "{body}");
  assert!(body["error"]["ACCESS_EXCEEDS_YOURS"].is_string());

  // Deleting the group deletes its grants.
  let (status, _) = send(
    &router,
    "DELETE",
    &format!("/api/v1/groups/{group}"),
    root.browser(),
    None,
  )
  .await;
  assert_eq!(status, 200);
  let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM grants WHERE principal_type = 'group'")
    .fetch_one(state.db.pool())
    .await
    .unwrap();
  assert_eq!(left, 0);
  state.db.close().await;
}
