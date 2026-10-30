use super::grants_access::{As, Person, grant, instance, person, send, sign_in};
use super::*;

/// Creates an account with a role picked by id and signs it in.
pub(super) async fn with_role(
  router: &Router,
  creator: &Person,
  email: &str,
  role_id: &str,
) -> Person {
  let (status, body) = send(
    router,
    "POST",
    "/api/v1/users",
    creator.browser(),
    Some(json!({"email":email,"password":"correct-horse-123","roleId":role_id})),
  )
  .await;
  assert_eq!(status, 201, "{body}");
  sign_in(router, email).await
}

async fn projects_seen(
  router: &Router,
  auth: As<'_>,
) -> std::collections::BTreeSet<String> {
  let (status, body) = send(router, "GET", "/api/v1/audit-events?limit=200", auth, None).await;
  assert_eq!(status, 200, "{body}");
  body["data"]["items"]
    .as_array()
    .unwrap()
    .iter()
    .map(|event| event["projectId"].as_str().unwrap_or("instance").to_owned())
    .collect()
}

#[tokio::test]
async fn each_reader_sees_the_audit_events_their_role_and_grants_allow() {
  let (_dir, state, router) = test_app().await;
  let (root, _) = instance(&state, &router).await;
  let (_, payments) = send(
    &router,
    "GET",
    "/api/v1/projects/payments",
    root.cli(),
    None,
  )
  .await;
  let (_, search) = send(&router, "GET", "/api/v1/projects/search", root.cli(), None).await;
  let payments = payments["data"]["id"].as_str().unwrap().to_owned();
  let search = search["data"]["id"].as_str().unwrap().to_owned();

  let auditor = with_role(&router, &root, "auditor@example.com", "rol_auditor").await;
  let lead = person(&router, &root, "lead@example.com", "admin").await;
  let member = person(&router, &root, "member@example.com", "member").await;
  // The lead manages payments only.
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

  // The auditor and root read everything.
  for auth in [auditor.cli(), root.cli()] {
    let seen = projects_seen(&router, auth).await;
    assert!(
      seen.contains(&payments) && seen.contains(&search),
      "{seen:?}"
    );
  }
  // The lead reads instance events and payments, never search.
  let seen = projects_seen(&router, lead.cli()).await;
  assert!(
    seen.contains(&payments) && seen.contains("instance"),
    "{seen:?}"
  );
  assert!(!seen.contains(&search), "{seen:?}");
  // The member reads only their own actions.
  let (_, body) = send(&router, "GET", "/api/v1/audit-events", member.cli(), None).await;
  assert!(
    body["data"]["items"]
      .as_array()
      .unwrap()
      .iter()
      .all(|event| event["actorId"] == member.id.as_str()),
    "{body}"
  );
  let (status, _) = send(
    &router,
    "GET",
    "/api/v1/audit-events/verify",
    member.cli(),
    None,
  )
  .await;
  assert_eq!(status, 403);

  // An auditor's token reads the audit log for a SIEM and no project data.
  let (status, body) = send(
    &router,
    "POST",
    "/api/v1/users/me/tokens",
    auditor.browser(),
    Some(json!({"name":"siem","scopeType":"all","level":"read"})),
  )
  .await;
  assert_eq!(status, 422, "{body}");
  let (status, body) = send(
    &router,
    "POST",
    "/api/v1/users/me/tokens",
    auditor.browser(),
    Some(json!({"name":"siem","scopeType":"audit","expiresIn":"90d"})),
  )
  .await;
  assert_eq!(status, 201, "{body}");
  assert_eq!(body["data"]["token"]["scopeType"], "audit");
  let siem = body["data"]["plaintextToken"].as_str().unwrap().to_owned();
  let seen = projects_seen(&router, As::Token(&siem)).await;
  assert!(seen.contains(&search), "{seen:?}");
  let (status, _) = send(
    &router,
    "GET",
    "/api/v1/audit-events/verify",
    As::Token(&siem),
    None,
  )
  .await;
  assert_eq!(status, 200);
  let (status, body) = send(&router, "GET", "/api/v1/projects", As::Token(&siem), None).await;
  assert_eq!(status, 200);
  assert!(body["data"].as_array().unwrap().is_empty(), "{body}");
  state.db.close().await;
}
