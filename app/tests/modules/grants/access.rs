use super::*;

/// How a test request signs in.
#[derive(Clone, Copy)]
pub(super) enum As<'a> {
  Token(&'a str),
  Browser(&'a str, &'a str),
}

pub(super) async fn send(
  router: &Router,
  method: &str,
  path: &str,
  auth: As<'_>,
  body: Option<Value>,
) -> (u16, Value) {
  let mut request = Request::builder().method(method).uri(path);
  request = match auth {
    As::Token(token) => request.header(header::AUTHORIZATION, format!("Bearer {token}")),
    As::Browser(cookie, csrf) => request
      .header(header::COOKIE, cookie)
      .header("x-dopbase-csrf", csrf),
  };
  if body.is_some() {
    request = request.header(header::CONTENT_TYPE, "application/json");
  }
  let response = router
    .clone()
    .oneshot(
      request
        .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
        .unwrap(),
    )
    .await
    .unwrap();
  let status = response.status().as_u16();
  let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
  (
    status,
    serde_json::from_slice(&bytes).unwrap_or(Value::Null),
  )
}

/// A signed-in account: its id, CLI token, and Admin UI cookie and CSRF.
pub(super) struct Person {
  pub id: String,
  pub token: String,
  pub cookie: String,
  pub csrf: String,
}

impl Person {
  pub fn cli(&self) -> As<'_> {
    As::Token(&self.token)
  }

  pub fn browser(&self) -> As<'_> {
    As::Browser(&self.cookie, &self.csrf)
  }
}

pub(super) async fn sign_in(
  router: &Router,
  email: &str,
) -> Person {
  let login = |kind: &'static str| {
    call(
      router,
      "POST",
      "/api/v1/auth/login",
      None,
      Some(json!({"email":email,"password":"correct-horse-123","sessionKind":kind})),
    )
  };
  let (status, cli, _) = login("cli").await;
  assert_eq!(status, 200, "{cli}");
  let (status, browser, headers) = login("browser").await;
  assert_eq!(status, 200, "{browser}");
  Person {
    id: browser["data"]["adminId"].as_str().unwrap().to_owned(),
    token: cli["data"]["token"].as_str().unwrap().to_owned(),
    cookie: headers
      .get(header::SET_COOKIE)
      .unwrap()
      .to_str()
      .unwrap()
      .split(';')
      .next()
      .unwrap()
      .to_owned(),
    csrf: browser["data"]["csrfToken"].as_str().unwrap().to_owned(),
  }
}

/// Root, signed in, with projects `payments` (dev, prod) and `search` (dev).
/// Returns root and the environment ids by `project/name`.
pub(super) async fn instance(
  state: &app::state::AppState,
  router: &Router,
) -> (Person, std::collections::HashMap<String, String>) {
  bootstrap_admin(state, router).await;
  let root = sign_in(router, "admin@example.com").await;
  let mut environments = std::collections::HashMap::new();
  for (project, names) in [
    ("payments", ["dev", "prod"].as_slice()),
    ("search", &["dev"]),
  ] {
    let (status, body) = send(
      router,
      "POST",
      "/api/v1/projects",
      root.cli(),
      Some(json!({"name":project})),
    )
    .await;
    assert_eq!(status, 201, "{body}");
    for name in names {
      let (status, body) = send(
        router,
        "POST",
        &format!("/api/v1/projects/{project}/environments"),
        root.cli(),
        Some(json!({"name":name})),
      )
      .await;
      assert_eq!(status, 201, "{body}");
      environments.insert(
        format!("{project}/{name}"),
        body["data"]["id"].as_str().unwrap().to_owned(),
      );
    }
  }
  (root, environments)
}

/// Creates an account as `creator` and signs it in.
pub(super) async fn person(
  router: &Router,
  creator: &Person,
  email: &str,
  role: &str,
) -> Person {
  let (status, body) = send(
    router,
    "POST",
    "/api/v1/users",
    creator.browser(),
    Some(json!({"email":email,"password":"correct-horse-123","role":role})),
  )
  .await;
  assert_eq!(status, 201, "{body}");
  let person = sign_in(router, email).await;
  assert_eq!(person.id, body["data"]["id"]);
  person
}

pub(super) async fn grant(
  router: &Router,
  by: &Person,
  principal: (&str, &str),
  scope: (&str, Option<&str>),
  level: &str,
) -> (u16, Value) {
  send(
    router,
    "PUT",
    "/api/v1/grants",
    by.browser(),
    Some(json!({
      "principalType": principal.0,
      "principalId": principal.1,
      "scopeType": scope.0,
      "scope": scope.1,
      "level": level,
    })),
  )
  .await
}

pub(super) async fn names(
  router: &Router,
  who: &Person,
  path: &str,
) -> Vec<String> {
  let (status, body) = send(router, "GET", path, who.cli(), None).await;
  assert_eq!(status, 200, "{body}");
  body["data"]
    .as_array()
    .unwrap()
    .iter()
    .map(|item| {
      item["projectName"].as_str().map_or_else(
        || item["name"].as_str().unwrap().to_owned(),
        |project| format!("{project}/{}", item["name"].as_str().unwrap()),
      )
    })
    .collect()
}

#[tokio::test]
async fn grants_decide_what_a_member_reaches() {
  let (_dir, state, router) = test_app().await;
  let (root, env) = instance(&state, &router).await;
  let member = person(&router, &root, "member@example.com", "member").await;
  let dev = &env["payments/dev"];
  let prod = &env["payments/prod"];
  let secrets = |id: &str| format!("/api/v1/environments/{id}/secrets");

  // A new member reaches nothing, and what it can't see doesn't exist.
  assert!(names(&router, &member, "/api/v1/projects").await.is_empty());
  assert!(
    names(&router, &member, "/api/v1/environments")
      .await
      .is_empty()
  );
  let (status, _) = send(&router, "GET", &secrets(dev), member.cli(), None).await;
  assert_eq!(status, 404);
  let (status, _) = send(
    &router,
    "GET",
    "/api/v1/projects/payments",
    member.cli(),
    None,
  )
  .await;
  assert_eq!(status, 404);

  let (status, body) = grant(
    &router,
    &root,
    ("user", &member.id),
    ("project", Some("payments")),
    "write",
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["scopeLabel"], "payments");
  assert_eq!(
    names(&router, &member, "/api/v1/projects").await,
    ["payments"]
  );
  assert_eq!(
    names(&router, &member, "/api/v1/environments").await,
    ["payments/dev", "payments/prod"]
  );
  let (status, body) = send(
    &router,
    "PUT",
    &format!("{}/API_KEY", secrets(dev)),
    member.cli(),
    Some(json!({"value":"member-wrote-this"})),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  let (status, _) = send(
    &router,
    "GET",
    &format!("{}/runtime", secrets(dev)),
    member.cli(),
    None,
  )
  .await;
  assert_eq!(status, 200);
  // Write is not manage: no runner tokens, renames, or new environments.
  for (method, path, body) in [
    ("GET", format!("/api/v1/environments/{dev}/tokens"), None),
    (
      "PATCH",
      format!("/api/v1/environments/{dev}"),
      Some(json!({"name":"renamed"})),
    ),
    (
      "POST",
      "/api/v1/projects/payments/environments".to_owned(),
      Some(json!({"name":"staging"})),
    ),
    (
      "POST",
      "/api/v1/projects".to_owned(),
      Some(json!({"name":"mine"})),
    ),
  ] {
    let (status, response) = send(&router, method, &path, member.cli(), body).await;
    assert_eq!(status, 403, "{method} {path}: {response}");
  }

  // Protecting prod cuts the project grant off there.
  let (status, body) = send(
    &router,
    "PATCH",
    &format!("/api/v1/environments/{prod}"),
    root.cli(),
    Some(json!({"protected":true})),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["protected"], true);
  assert_eq!(
    names(&router, &member, "/api/v1/environments").await,
    ["payments/dev"]
  );
  let (status, _) = send(&router, "GET", &secrets(prod), member.cli(), None).await;
  assert_eq!(status, 404);

  // An environment grant reaches it again, here names only.
  let (status, _) = grant(
    &router,
    &root,
    ("user", &member.id),
    ("environment", Some(prod)),
    "metadata",
  )
  .await;
  assert_eq!(status, 200);
  let (status, _) = send(&router, "GET", &secrets(prod), member.cli(), None).await;
  assert_eq!(status, 200);
  let (status, _) = send(
    &router,
    "GET",
    &format!("{}/runtime", secrets(prod)),
    member.cli(),
    None,
  )
  .await;
  assert_eq!(status, 403);

  // `none` on one environment cuts it out of the project grant.
  let (status, _) = grant(
    &router,
    &root,
    ("user", &member.id),
    ("environment", Some(dev)),
    "none",
  )
  .await;
  assert_eq!(status, 200);
  assert_eq!(
    names(&router, &member, "/api/v1/environments").await,
    ["payments/prod"]
  );

  // The member's own view of where that comes from.
  let (status, body) = send(
    &router,
    "GET",
    &format!("/api/v1/users/{}/access", member.id),
    member.cli(),
    None,
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["maxLevel"], "manage");
  assert_eq!(body["data"]["grants"].as_array().unwrap().len(), 3);
  let (status, _) = send(
    &router,
    "GET",
    &format!("/api/v1/users/{}/access", root.id),
    member.cli(),
    None,
  )
  .await;
  assert_eq!(status, 403);

  for action in ["grant.set", "environment.protected"] {
    let (status, body) = send(
      &router,
      "GET",
      &format!("/api/v1/audit-events?action={action}"),
      root.cli(),
      None,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert!(
      !body["data"]["items"].as_array().unwrap().is_empty(),
      "{action}"
    );
  }
  state.db.close().await;
}

#[tokio::test]
async fn grant_requests_are_checked_before_they_change_anything() {
  let (_dir, state, router) = test_app().await;
  let (root, env) = instance(&state, &router).await;
  let member = person(&router, &root, "member@example.com", "member").await;
  let dev = env["payments/dev"].clone();

  for (scope, level, code) in [
    (
      ("environment", Some(dev.as_str())),
      "project_manager",
      "GRANT_LEVEL_INVALID",
    ),
    (("project", Some("payments")), "none", "GRANT_LEVEL_INVALID"),
    (("project", None), "read", "GRANT_SCOPE_INVALID"),
    // A member's role stops at manage.
    (
      ("project", Some("payments")),
      "project_manager",
      "GRANT_LEVEL_INVALID",
    ),
  ] {
    let (status, body) = grant(&router, &root, ("user", &member.id), scope, level).await;
    assert_eq!(status, 422, "{body}");
    assert!(body["error"][code].is_string(), "{body}");
  }
  let (status, body) = grant(
    &router,
    &root,
    ("user", &root.id),
    ("project", Some("payments")),
    "read",
  )
  .await;
  assert_eq!(status, 403, "{body}");
  let (status, _) = grant(
    &router,
    &root,
    ("group", "grp_missing"),
    ("project", Some("payments")),
    "read",
  )
  .await;
  assert_eq!(status, 404);

  // Grants are changed from the Admin UI only.
  let (status, body) = send(
    &router,
    "PUT",
    "/api/v1/grants",
    root.cli(),
    Some(json!({"principalType":"user","principalId":member.id,"scopeType":"project","scope":"payments","level":"read"})),
  )
  .await;
  assert_eq!(status, 403, "{body}");
  // And not by the member, who manages nothing.
  let (status, _) = grant(
    &router,
    &member,
    ("user", &member.id),
    ("project", Some("payments")),
    "read",
  )
  .await;
  assert_eq!(status, 404);

  let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM grants WHERE principal_id = ?")
    .bind(&member.id)
    .fetch_one(state.db.pool())
    .await
    .unwrap();
  assert_eq!(count, 0);
  state.db.close().await;
}

#[tokio::test]
async fn a_scoped_admin_cannot_reach_past_its_own_access() {
  let (_dir, state, router) = test_app().await;
  let (root, env) = instance(&state, &router).await;
  // Root's admins start with every project; this one is cut to payments.
  let admin = person(&router, &root, "scoped@example.com", "admin").await;
  let peer = person(&router, &root, "peer@example.com", "admin").await;
  let member = person(&router, &root, "member@example.com", "member").await;
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
  assert_eq!(access["data"]["grants"][0]["scopeType"], "all");
  let (status, _) = send(
    &router,
    "DELETE",
    &format!("/api/v1/grants/{all}"),
    root.browser(),
    None,
  )
  .await;
  assert_eq!(status, 200);
  let (status, _) = grant(
    &router,
    &root,
    ("user", &admin.id),
    ("project", Some("payments")),
    "project_manager",
  )
  .await;
  assert_eq!(status, 200);
  assert_eq!(
    names(&router, &admin, "/api/v1/projects").await,
    ["payments"]
  );

  // Within payments the admin hands out access freely.
  let (status, body) = grant(
    &router,
    &admin,
    ("user", &member.id),
    ("environment", Some(&env["payments/dev"])),
    "read",
  )
  .await;
  assert_eq!(status, 200, "{body}");
  let (status, body) = send(
    &router,
    "GET",
    "/api/v1/projects/payments/access",
    admin.cli(),
    None,
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["environments"].as_array().unwrap().len(), 2);
  assert_eq!(body["data"]["grants"].as_array().unwrap().len(), 2);

  // Outside it, nothing.
  let (status, _) = grant(
    &router,
    &admin,
    ("user", &member.id),
    ("project", Some("search")),
    "read",
  )
  .await;
  assert_eq!(status, 404);
  let (status, body) = grant(&router, &admin, ("user", &member.id), ("all", None), "read").await;
  assert_eq!(status, 403, "{body}");
  let (status, body) = grant(
    &router,
    &admin,
    ("user", &admin.id),
    ("project", Some("payments")),
    "write",
  )
  .await;
  assert_eq!(status, 403, "{body}");
  assert!(body["error"]["SELF_GRANT"].is_string());

  // A peer with every project is out of reach, the member is not.
  let (status, body) = send(
    &router,
    "DELETE",
    &format!("/api/v1/users/{}", peer.id),
    admin.browser(),
    None,
  )
  .await;
  assert_eq!(status, 403, "{body}");
  assert!(body["error"]["ACCESS_EXCEEDS_YOURS"].is_string());
  let (status, body) = send(
    &router,
    "PATCH",
    &format!("/api/v1/users/{}", member.id),
    admin.browser(),
    Some(json!({"email":"renamed@example.com"})),
  )
  .await;
  assert_eq!(status, 200, "{body}");

  // Admins an admin creates start with no data access.
  let made = person(&router, &admin, "made@example.com", "admin").await;
  assert!(names(&router, &made, "/api/v1/projects").await.is_empty());

  // A project the admin creates is theirs to manage.
  let (status, body) = send(
    &router,
    "POST",
    "/api/v1/projects",
    admin.cli(),
    Some(json!({"name":"billing"})),
  )
  .await;
  assert_eq!(status, 201, "{body}");
  assert_eq!(body["data"]["level"], "project_manager");
  assert_eq!(
    names(&router, &admin, "/api/v1/projects").await,
    ["billing", "payments"]
  );
  state.db.close().await;
}
