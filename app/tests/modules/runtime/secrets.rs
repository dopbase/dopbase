use super::grants_access::{As, instance, person, send};
use super::*;
use axum::http::HeaderMap;

const GATE: &str = "/api/v1/runtime/secrets";

/// `GET` the gate with a runner token and an optional `If-None-Match`.
async fn fetch(
  router: &Router,
  token: &str,
  if_none_match: Option<&str>,
) -> (u16, Value, HeaderMap) {
  fetch_path(router, GATE, token, if_none_match).await
}

async fn fetch_path(
  router: &Router,
  path: &str,
  token: &str,
  if_none_match: Option<&str>,
) -> (u16, Value, HeaderMap) {
  let mut request = Request::builder()
    .uri(path)
    .header(header::AUTHORIZATION, format!("Bearer {token}"));
  if let Some(tag) = if_none_match {
    request = request.header(header::IF_NONE_MATCH, tag);
  }
  let response = router
    .clone()
    .oneshot(request.body(Body::empty()).unwrap())
    .await
    .unwrap();
  let status = response.status().as_u16();
  let headers = response.headers().clone();
  let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
  if status == 304 {
    assert!(bytes.is_empty(), "a 304 has no body");
  }
  (
    status,
    serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    headers,
  )
}

async fn runner(
  router: &Router,
  root: &super::grants_access::Person,
  environment_id: &str,
) -> (String, String) {
  let (status, body) = send(
    router,
    "POST",
    &format!("/api/v1/environments/{environment_id}/tokens"),
    root.cli(),
    Some(json!({"name":"deploy","role":"runner"})),
  )
  .await;
  assert_eq!(status, 201, "{body}");
  (
    body["data"]["plaintextToken"].as_str().unwrap().to_owned(),
    body["data"]["token"]["id"].as_str().unwrap().to_owned(),
  )
}

async fn set_secret(
  router: &Router,
  root: &super::grants_access::Person,
  environment_id: &str,
  key: &str,
  value: &str,
) -> u16 {
  send(
    router,
    "PUT",
    &format!("/api/v1/environments/{environment_id}/secrets/{key}"),
    root.cli(),
    Some(json!({"value":value})),
  )
  .await
  .0
}

async fn runtime_reads(state: &app::state::AppState) -> i64 {
  sqlx::query_scalar("SELECT COUNT(*) FROM audit_events WHERE action = 'secret.runtime_accessed'")
    .fetch_one(state.db.pool())
    .await
    .unwrap()
}

#[tokio::test]
async fn a_runner_token_fetches_its_own_environment_and_polls_with_etags() {
  let (_dir, state, router) = test_app().await;
  let (root, env) = instance(&state, &router).await;
  let dev = &env["payments/dev"];
  assert_eq!(
    set_secret(&router, &root, dev, "DATABASE_URL", "postgres://dev").await,
    200
  );
  let (token, _) = runner(&router, &root, dev).await;

  let (status, body, headers) = fetch(&router, &token, None).await;
  assert_eq!(status, 200, "{body}");
  let data = &body["data"];
  assert_eq!(data["project"], "payments");
  assert_eq!(data["environment"], "dev");
  assert_eq!(data["environmentId"], dev.as_str());
  assert!(data["projectId"].as_str().unwrap().starts_with("prj_"));
  assert_eq!(
    data["entries"],
    json!([{"key":"DATABASE_URL","value":"postgres://dev"}])
  );
  let revision = data["revision"].as_str().unwrap().to_owned();
  let tag = format!("\"{revision}\"");
  assert_eq!(headers[header::ETAG], tag.as_str());
  assert_eq!(headers[header::CACHE_CONTROL], "no-store");
  assert_eq!(runtime_reads(&state).await, 1);

  // The same revision as the export route.
  let (status, export) = send(
    &router,
    "POST",
    &format!("/api/v1/environments/{dev}/secrets/export"),
    As::Token(&token),
    None,
  )
  .await;
  assert_eq!(status, 200, "{export}");
  assert_eq!(export["data"]["revision"], revision.as_str());

  // Unchanged: a bare 304, whichever way the tag is written, and no audit.
  for header in [tag.clone(), format!("W/{tag}"), format!("\"other\", {tag}")] {
    let (status, _, headers) = fetch(&router, &token, Some(&header)).await;
    assert_eq!(status, 304, "{header}");
    assert_eq!(headers[header::ETAG], tag.as_str());
  }
  assert_eq!(runtime_reads(&state).await, 1);

  // A stale tag gets the values again.
  let (status, _, _) = fetch(&router, &token, Some("\"stale\"")).await;
  assert_eq!(status, 200);
  assert_eq!(runtime_reads(&state).await, 2);

  // A write moves the revision, so the old tag no longer matches.
  assert_eq!(
    set_secret(&router, &root, dev, "API_KEY", "dev-key").await,
    200
  );
  let (status, body, headers) = fetch(&router, &token, Some(&tag)).await;
  assert_eq!(status, 200);
  assert_ne!(body["data"]["revision"], revision.as_str());
  assert_ne!(headers[header::ETAG], tag.as_str());
  assert_eq!(body["data"]["entries"].as_array().unwrap().len(), 2);
  state.db.close().await;
}

#[tokio::test]
async fn a_reference_only_checks_the_token_s_environment_and_reads_nothing_when_wrong() {
  let (_dir, state, router) = test_app().await;
  let (root, env) = instance(&state, &router).await;
  let dev = &env["payments/dev"];
  let (token, _) = runner(&router, &root, dev).await;
  let (_, body, _) = fetch(&router, &token, None).await;
  let project_id = body["data"]["projectId"].as_str().unwrap();
  let with = |reference: &str| format!("{GATE}?reference={reference}");

  for reference in [
    dev.clone(),
    "payments/dev".into(),
    format!("{project_id}/dev"),
  ] {
    let (status, body, _) = fetch_path(&router, &with(&reference), &token, None).await;
    assert_eq!(status, 200, "{reference}: {body}");
  }
  let reads = runtime_reads(&state).await;
  // Another environment, real or not, gets the same answer and no read.
  for reference in [
    "payments/prod",
    "search/dev",
    "payments/nope",
    &env["payments/prod"],
  ] {
    let (status, body, _) = fetch_path(&router, &with(reference), &token, None).await;
    assert_eq!(status, 403, "{reference}");
    assert_eq!(
      body["error"]["TOKEN_SCOPE_INVALID"],
      format!("The runner token cannot access {reference}."),
    );
  }
  assert_eq!(runtime_reads(&state).await, reads);
  state.db.close().await;
}

#[tokio::test]
async fn only_runner_tokens_reach_the_runtime_gate() {
  let (_dir, state, router) = test_app().await;
  let (root, _) = instance(&state, &router).await;
  let (status, body) = send(&router, "GET", GATE, root.cli(), None).await;
  assert_eq!(status, 403);
  assert!(body["error"]["RUNNER_TOKEN_REQUIRED"].is_string(), "{body}");

  let member = person(&router, &root, "member@example.com", "member").await;
  let (status, body) = send(
    &router,
    "POST",
    "/api/v1/users/me/tokens",
    member.browser(),
    Some(json!({"name":"ci","level":"read"})),
  )
  .await;
  assert_eq!(status, 201, "{body}");
  let personal = body["data"]["plaintextToken"].as_str().unwrap();
  let (status, body) = send(&router, "GET", GATE, As::Token(personal), None).await;
  assert_eq!(status, 403);
  assert!(body["error"]["RUNNER_TOKEN_REQUIRED"].is_string(), "{body}");

  let (status, _, _) = call(&router, "GET", GATE, None, None).await;
  assert_eq!(status, 401);
  state.db.close().await;
}

async fn last_used(
  state: &app::state::AppState,
  id: &str,
) -> Option<String> {
  sqlx::query_scalar("SELECT last_used_at FROM runner_tokens WHERE id = ?")
    .bind(id)
    .fetch_one(state.db.pool())
    .await
    .unwrap()
}

async fn set_last_used(
  state: &app::state::AppState,
  id: &str,
  seconds_ago: i64,
) -> String {
  let at = (chrono::Utc::now() - chrono::Duration::seconds(seconds_ago)).to_rfc3339();
  sqlx::query("UPDATE runner_tokens SET last_used_at = ? WHERE id = ?")
    .bind(&at)
    .bind(id)
    .execute(state.db.pool())
    .await
    .unwrap();
  at
}

#[tokio::test]
async fn a_polling_runner_writes_last_used_at_at_most_once_a_minute() {
  let (_dir, state, router) = test_app().await;
  let (root, env) = instance(&state, &router).await;
  let (token, id) = runner(&router, &root, &env["payments/dev"]).await;
  assert_eq!(fetch(&router, &token, None).await.0, 200);
  assert!(last_used(&state, &id).await.is_some());

  let recent = set_last_used(&state, &id, 30).await;
  assert_eq!(fetch(&router, &token, None).await.0, 200);
  assert_eq!(
    last_used(&state, &id).await.as_deref(),
    Some(recent.as_str())
  );

  let old = set_last_used(&state, &id, 61).await;
  assert_eq!(fetch(&router, &token, None).await.0, 200);
  assert!(last_used(&state, &id).await.unwrap() > old);
  state.db.close().await;
}
