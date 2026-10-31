use super::grants_access::{As, Person, grant, instance, person, send};
use super::*;

async fn token(
  router: &Router,
  by: &Person,
  user: &str,
  body: Value,
) -> (u16, Value) {
  send(
    router,
    "POST",
    &format!("/api/v1/users/{user}/tokens"),
    by.browser(),
    Some(body),
  )
  .await
}

fn plaintext(body: &Value) -> String {
  body["data"]["plaintextToken"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn a_personal_token_reaches_its_account_s_live_access_within_its_scope() {
  let (_dir, state, router) = test_app().await;
  let (root, env) = instance(&state, &router).await;
  let member = person(&router, &root, "member@example.com", "member").await;
  let dev = &env["payments/dev"];
  let prod = &env["payments/prod"];
  grant(
    &router,
    &root,
    ("user", &member.id),
    ("project", Some("payments")),
    "write",
  )
  .await;
  let runtime = |id: &str| format!("/api/v1/environments/{id}/secrets/runtime");

  // A read token on dev only.
  let (status, body) = token(
    &router,
    &member,
    "me",
    json!({"name":"laptop-agent","scopeType":"environment","scope":dev,"level":"read","expiresIn":"7d"}),
  )
  .await;
  assert_eq!(status, 201, "{body}");
  assert_eq!(body["data"]["token"]["scopeLabel"], "payments/dev");
  let laptop = plaintext(&body);
  let (status, _) = send(&router, "GET", &runtime(dev), As::Token(&laptop), None).await;
  assert_eq!(status, 200);
  let (status, _) = send(&router, "GET", &runtime(prod), As::Token(&laptop), None).await;
  assert_eq!(status, 404);
  let (status, _) = send(
    &router,
    "PUT",
    &format!("/api/v1/environments/{dev}/secrets/FROM_TOKEN"),
    As::Token(&laptop),
    Some(json!({"value":"nope"})),
  )
  .await;
  assert_eq!(status, 403);

  // A write token changes secrets, but never manages or creates anything.
  let (_, body) = token(&router, &member, "me", json!({"name":"ci","level":"write"})).await;
  let ci = plaintext(&body);
  let (status, body) = send(
    &router,
    "PUT",
    &format!("/api/v1/environments/{dev}/secrets/FROM_TOKEN"),
    As::Token(&ci),
    Some(json!({"value":"written-by-token"})),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  for (method, path) in [
    ("GET", format!("/api/v1/environments/{dev}/tokens")),
    ("GET", "/api/v1/users/me/tokens".to_owned()),
    ("POST", "/api/v1/projects".to_owned()),
  ] {
    let (status, body) = send(
      &router,
      method,
      &path,
      As::Token(&ci),
      Some(json!({"name":"x"})),
    )
    .await;
    assert_eq!(status, 403, "{method} {path}: {body}");
  }
  let (status, body) = send(
    &router,
    "GET",
    "/api/v1/users/me/access",
    As::Token(&ci),
    None,
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["email"], "member@example.com");
  assert_eq!(body["data"]["token"]["name"], "ci");
  assert_eq!(body["data"]["token"]["level"], "write");

  // The token follows the account: losing the grant loses the data.
  let (_, access) = send(
    &router,
    "GET",
    &format!("/api/v1/users/{}/access", member.id),
    root.browser(),
    None,
  )
  .await;
  let grant_id = access["data"]["grants"][0]["id"]
    .as_str()
    .unwrap()
    .to_owned();
  send(
    &router,
    "DELETE",
    &format!("/api/v1/grants/{grant_id}"),
    root.browser(),
    None,
  )
  .await;
  let (status, _) = send(&router, "GET", &runtime(dev), As::Token(&laptop), None).await;
  assert_eq!(status, 404);

  // A disabled account's tokens stop working.
  sqlx::query("UPDATE admins SET status = 'disabled' WHERE id = ?")
    .bind(&member.id)
    .execute(state.db.pool())
    .await
    .unwrap();
  let (status, _) = send(&router, "GET", "/api/v1/projects", As::Token(&ci), None).await;
  assert_eq!(status, 401);
  sqlx::query("UPDATE admins SET status = 'active' WHERE id = ?")
    .bind(&member.id)
    .execute(state.db.pool())
    .await
    .unwrap();

  // Revoking is immediate, and a member can't revoke someone else's.
  let (_, list) = send(
    &router,
    "GET",
    "/api/v1/users/me/tokens",
    member.cli(),
    None,
  )
  .await;
  assert_eq!(list["data"].as_array().unwrap().len(), 2);
  let ci_id = list["data"][0]["id"].as_str().unwrap().to_owned();
  let (status, _) = send(
    &router,
    "POST",
    &format!("/api/v1/users/{}/tokens/{ci_id}/revoke", member.id),
    root.browser(),
    None,
  )
  .await;
  assert_eq!(status, 200);
  let (status, _) = send(&router, "GET", "/api/v1/projects", As::Token(&ci), None).await;
  assert_eq!(status, 401);
  let (status, _) = send(
    &router,
    "POST",
    &format!("/api/v1/users/{}/tokens", root.id),
    member.browser(),
    Some(json!({"name":"x","level":"read"})),
  )
  .await;
  assert_eq!(status, 403);
  state.db.close().await;
}

#[tokio::test]
async fn an_ai_agent_never_reaches_more_than_its_owner() {
  let (_dir, state, router) = test_app().await;
  let (root, env) = instance(&state, &router).await;
  let owner = person(&router, &root, "lead@example.com", "admin").await;
  let (status, body) = send(
    &router,
    "POST",
    "/api/v1/users",
    owner.browser(),
    Some(json!({"email":"review-bot","role":"ai_agent"})),
  )
  .await;
  assert_eq!(status, 201, "{body}");
  assert_eq!(body["data"]["ownerId"], owner.id);
  let agent = body["data"]["id"].as_str().unwrap().to_owned();
  // Root gives the agent read on everything; the owner manages all
  // projects, so the agent reads both.
  grant(&router, &root, ("user", &agent), ("all", None), "read").await;
  let (status, body) = token(
    &router,
    &owner,
    &agent,
    json!({"name":"default","level":"read"}),
  )
  .await;
  assert_eq!(status, 201, "{body}");
  let bot = plaintext(&body);
  let runtime = |id: &str| format!("/api/v1/environments/{id}/secrets/runtime");
  let search = &env["search/dev"];
  let (status, _) = send(&router, "GET", &runtime(search), As::Token(&bot), None).await;
  assert_eq!(status, 200);
  // A person can't sign in as the agent.
  let (status, _, _) = call(
    &router,
    "POST",
    "/api/v1/auth/login",
    None,
    Some(json!({"email":"review-bot","password":"","sessionKind":"cli"})),
  )
  .await;
  assert!(status == 401 || status == 422, "{status}");

  // Cut the owner to payments, and the agent loses search too.
  let (_, access) = send(
    &router,
    "GET",
    &format!("/api/v1/users/{}/access", owner.id),
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
    ("user", &owner.id),
    ("project", Some("payments")),
    "project_manager",
  )
  .await;
  let (status, _) = send(&router, "GET", &runtime(search), As::Token(&bot), None).await;
  assert_eq!(status, 404);
  let (status, _) = send(
    &router,
    "GET",
    &runtime(&env["payments/dev"]),
    As::Token(&bot),
    None,
  )
  .await;
  assert_eq!(status, 200);

  // The owner can't leave while answering for the agent, and a disabled
  // owner disables the agent.
  let (status, body) = send(
    &router,
    "DELETE",
    &format!("/api/v1/users/{}", owner.id),
    root.browser(),
    None,
  )
  .await;
  assert_eq!(status, 409, "{body}");
  assert!(body["error"]["OWNS_AI_AGENTS"].is_string());
  sqlx::query("UPDATE admins SET status = 'disabled' WHERE id = ?")
    .bind(&owner.id)
    .execute(state.db.pool())
    .await
    .unwrap();
  let (status, _) = send(&router, "GET", "/api/v1/projects", As::Token(&bot), None).await;
  assert_eq!(status, 401);

  // Root hands the agent to itself, and the owner can go.
  let (status, body) = send(
    &router,
    "PATCH",
    &format!("/api/v1/users/{agent}"),
    root.browser(),
    Some(json!({"ownerId":root.id})),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["ownerId"], root.id);
  let (status, _) = send(
    &router,
    "DELETE",
    &format!("/api/v1/users/{}", owner.id),
    root.browser(),
    None,
  )
  .await;
  assert_eq!(status, 200);
  let (status, _) = send(&router, "GET", &runtime(search), As::Token(&bot), None).await;
  assert_eq!(status, 200);
  state.db.close().await;
}

#[tokio::test]
async fn replacing_an_agent_s_token_revokes_its_other_live_tokens_in_one_step() {
  let (_dir, state, router) = test_app().await;
  let (root, _) = instance(&state, &router).await;
  let owner = person(&router, &root, "lead@example.com", "admin").await;
  let (_, body) = send(
    &router,
    "POST",
    "/api/v1/users",
    owner.browser(),
    Some(json!({"email":"deploy-bot","role":"ai_agent"})),
  )
  .await;
  let agent = body["data"]["id"].as_str().unwrap().to_owned();
  let mut old = Vec::new();
  for name in ["first", "second", "stale"] {
    let (status, body) = token(&router, &owner, &agent, json!({"name":name,"level":"read"})).await;
    assert_eq!(status, 201, "{body}");
    old.push((
      body["data"]["token"]["id"].as_str().unwrap().to_owned(),
      plaintext(&body),
    ));
  }
  // An expired token is left as it is.
  sqlx::query("UPDATE personal_tokens SET expires_at = '2000-01-01T00:00:00+00:00' WHERE id = ?")
    .bind(&old[2].0)
    .execute(state.db.pool())
    .await
    .unwrap();
  let projects = |bearer: &str| {
    let bearer = bearer.to_owned();
    let router = router.clone();
    async move {
      send(&router, "GET", "/api/v1/projects", As::Token(&bearer), None)
        .await
        .0
    }
  };
  assert_eq!(projects(&old[0].1).await, 200);

  let (status, body) = token(
    &router,
    &owner,
    &agent,
    json!({"name":"rotated","level":"read","replaceExisting":true}),
  )
  .await;
  assert_eq!(status, 201, "{body}");
  let rotated = plaintext(&body);
  assert_eq!(projects(&rotated).await, 200);
  assert_eq!(projects(&old[0].1).await, 401);
  assert_eq!(projects(&old[1].1).await, 401);
  let revoked: Vec<String> = sqlx::query_scalar(
    "SELECT resource_id FROM audit_events WHERE action = 'personal_token.revoked' AND json_extract(metadata, '$.userId') = ? ORDER BY seq",
  )
  .bind(&agent)
  .fetch_all(state.db.pool())
  .await
  .unwrap();
  assert_eq!(revoked, [old[0].0.clone(), old[1].0.clone()]);
  let stale: Option<String> =
    sqlx::query_scalar("SELECT revoked_at FROM personal_tokens WHERE id = ?")
      .bind(&old[2].0)
      .fetch_one(state.db.pool())
      .await
      .unwrap();
  assert!(stale.is_none());

  // A person's tokens are theirs to revoke one by one.
  let member = person(&router, &root, "member@example.com", "member").await;
  let (_, body) = token(
    &router,
    &member,
    "me",
    json!({"name":"laptop","level":"read"}),
  )
  .await;
  let laptop = plaintext(&body);
  let (status, body) = token(
    &router,
    &member,
    "me",
    json!({"name":"desktop","level":"read","replaceExisting":true}),
  )
  .await;
  assert_eq!(status, 422, "{body}");
  assert!(body["error"]["TOKEN_REPLACE_INVALID"].is_string(), "{body}");
  assert_eq!(projects(&laptop).await, 200);
  state.db.close().await;
}
