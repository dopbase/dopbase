use super::secrets_import::{
  browser_call, browser_login, expire_confirmation, metadata_agent_token,
};
use super::*;

async fn put_secret(
  router: &Router,
  token: &str,
  environment_id: &str,
  key: &str,
  value: &str,
) {
  let (status, body, _) = call(
    router,
    "PUT",
    &format!("/api/v1/environments/{environment_id}/secrets/{key}"),
    Some(token),
    Some(json!({ "value": value })),
  )
  .await;
  assert_eq!(status, 200, "{body}");
}

async fn history(
  router: &Router,
  token: &str,
  environment_id: &str,
  key: &str,
  query: &str,
) -> Value {
  let (status, body, _) = call(
    router,
    "GET",
    &format!("/api/v1/environments/{environment_id}/secrets/{key}/versions{query}"),
    Some(token),
    None,
  )
  .await;
  assert_eq!(status, 200, "{body}");
  body["data"].clone()
}

async fn rollback(
  router: &Router,
  token: &str,
  environment_id: &str,
  key: &str,
  body: Value,
) -> (u16, Value) {
  let (status, body, _) = call(
    router,
    "POST",
    &format!("/api/v1/environments/{environment_id}/secrets/{key}/rollback"),
    Some(token),
    Some(body),
  )
  .await;
  (status, body)
}

async fn runtime_value(
  router: &Router,
  token: &str,
  environment_id: &str,
  key: &str,
) -> String {
  let (status, body, _) = call(
    router,
    "GET",
    &format!("/api/v1/environments/{environment_id}/secrets/runtime"),
    Some(token),
    None,
  )
  .await;
  assert_eq!(status, 200, "{body}");
  body["data"]["entries"]
    .as_array()
    .unwrap()
    .iter()
    .find(|entry| entry["key"] == key)
    .map(|entry| entry["value"].as_str().unwrap().to_owned())
    .unwrap()
}

fn versions(page: &Value) -> Vec<i64> {
  page["items"]
    .as_array()
    .unwrap()
    .iter()
    .map(|item| item["version"].as_i64().unwrap())
    .collect()
}

#[tokio::test]
async fn every_write_records_a_version_with_its_change_and_actor() {
  let (_dir, state, router, token, id) = admin_environment().await;
  put_secret(&router, &token, &id, "API_KEY", "first-value").await;
  put_secret(&router, &token, &id, "API_KEY", "second-value").await;
  let (status, body, _) = call(
    &router,
    "POST",
    &format!("/api/v1/environments/{id}/secrets/import"),
    Some(&token),
    Some(json!({"mode":"merge","entries":[{"key":"API_KEY","value":"third-value"}]})),
  )
  .await;
  assert_eq!(status, 200, "{body}");

  let page = history(&router, &token, &id, "API_KEY", "").await;
  assert_eq!(versions(&page), [3, 2, 1]);
  let changes: Vec<&str> = page["items"]
    .as_array()
    .unwrap()
    .iter()
    .map(|item| item["changeType"].as_str().unwrap())
    .collect();
  assert_eq!(changes, ["imported", "updated", "created"]);
  for item in page["items"].as_array().unwrap() {
    assert_eq!(item["actorType"], "root");
    assert_eq!(item["actorLabel"], "admin@example.com");
    assert_eq!(item["restorable"], true);
    assert_eq!(item["current"], item["version"] == 3);
    assert!(item.get("value").is_none(), "history never returns values");
  }
  assert_eq!(page["nextBefore"], Value::Null);
  assert_eq!(page["retainedVersions"], 20);
  assert!(!page.to_string().contains("-value"));

  let first = history(&router, &token, &id, "API_KEY", "?limit=2").await;
  assert_eq!(versions(&first), [3, 2]);
  assert_eq!(first["nextBefore"], 2);
  let second = history(&router, &token, &id, "API_KEY", "?limit=2&before=2").await;
  assert_eq!(versions(&second), [1]);
  assert_eq!(second["nextBefore"], Value::Null);

  // Project init writes its secrets through the same path.
  let (status, initialized, _) = call(
    &router,
    "POST",
    "/api/v1/projects/init",
    Some(&token),
    Some(json!({"projectName":"init-history","environmentName":"dev","entries":[{"key":"DB_URL","value":"postgres://init"}]})),
  )
  .await;
  assert_eq!(status, 201);
  let init_id = initialized["data"]["environmentId"].as_str().unwrap();
  let page = history(&router, &token, init_id, "DB_URL", "").await;
  assert_eq!(versions(&page), [1]);
  assert_eq!(page["items"][0]["changeType"], "created");

  let (status, missing, _) = call(
    &router,
    "GET",
    &format!("/api/v1/environments/{id}/secrets/MISSING/versions"),
    Some(&token),
    None,
  )
  .await;
  assert_eq!(status, 404);
  assert!(missing["error"]["SECRET_NOT_FOUND"].is_string());
  state.db.close().await;
}

#[tokio::test]
async fn rollback_saves_the_earlier_value_as_a_new_version() {
  let (_dir, state, router, token, id) = admin_environment().await;
  for value in ["value-one", "value-two", "value-three"] {
    put_secret(&router, &token, &id, "API_KEY", value).await;
  }
  let before = history(&router, &token, &id, "API_KEY", "").await;

  let (status, body) = rollback(
    &router,
    &token,
    &id,
    "API_KEY",
    json!({"version":1,"expectedVersion":3}),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["version"], 4);
  assert_eq!(body["data"]["sourceVersion"], 1);
  assert_eq!(body["data"]["previousVersion"], 3);
  assert_eq!(
    runtime_value(&router, &token, &id, "API_KEY").await,
    "value-one"
  );

  let after = history(&router, &token, &id, "API_KEY", "").await;
  assert_eq!(versions(&after), [4, 3, 2, 1]);
  assert_eq!(after["items"][0]["changeType"], "rolled_back");
  assert_eq!(after["items"][0]["sourceVersion"], 1);
  assert_eq!(after["items"][0]["current"], true);
  // Earlier versions are untouched apart from no longer being current.
  let mut unchanged = before["items"].as_array().unwrap().clone();
  unchanged[0]["current"] = json!(false);
  assert_eq!(after["items"].as_array().unwrap()[1..], unchanged[..]);

  // Rolling back to the version that was rolled away from also works.
  let (status, body) = rollback(&router, &token, &id, "API_KEY", json!({"version":3})).await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["version"], 5);
  assert_eq!(
    runtime_value(&router, &token, &id, "API_KEY").await,
    "value-three"
  );

  let (status, revealed, _) = call(
    &router,
    "POST",
    &format!("/api/v1/environments/{id}/secrets/API_KEY/versions/2/reveal"),
    Some(&token),
    None,
  )
  .await;
  assert_eq!(status, 200, "{revealed}");
  assert_eq!(revealed["data"]["value"], "value-two");
  assert_eq!(revealed["data"]["version"], 2);

  let audit: Vec<(String, String)> = sqlx::query_as(
    "SELECT action,metadata FROM audit_events WHERE action IN ('secret.rolled_back','secret.version_revealed') ORDER BY created_at,id",
  )
  .fetch_all(state.db.pool())
  .await
  .unwrap();
  assert_eq!(
    audit
      .iter()
      .map(|(action, _)| action.as_str())
      .collect::<Vec<_>>(),
    [
      "secret.rolled_back",
      "secret.rolled_back",
      "secret.version_revealed"
    ]
  );
  let first: Value = serde_json::from_str(&audit[0].1).unwrap();
  assert_eq!(
    first,
    json!({"key":"API_KEY","sourceVersion":1,"previousVersion":3,"version":4})
  );
  for (_, metadata) in &audit {
    assert!(!metadata.contains("value-"), "audit stores no values");
  }
  state.db.close().await;
}

#[tokio::test]
async fn rollback_refuses_bad_targets_without_writing() {
  let (_dir, state, router, token, id) = admin_environment().await;
  put_secret(&router, &token, &id, "API_KEY", "value-one").await;
  put_secret(&router, &token, &id, "API_KEY", "value-two").await;

  for (body, status, code) in [
    (json!({"version":2}), 409, "SECRET_VERSION_IS_CURRENT"),
    (json!({"version":7}), 404, "SECRET_VERSION_NOT_FOUND"),
    (
      json!({"version":1,"expectedVersion":1}),
      409,
      "SECRET_VERSION_CONFLICT",
    ),
  ] {
    let (actual, response) = rollback(&router, &token, &id, "API_KEY", body.clone()).await;
    assert_eq!(actual, status, "{body}: {response}");
    assert!(response["error"][code].is_string(), "{body}: {response}");
  }
  let (status, response) = rollback(&router, &token, &id, "OTHER", json!({"version":1})).await;
  assert_eq!(status, 404);
  assert!(response["error"]["SECRET_NOT_FOUND"].is_string());
  let (status, _) = rollback(
    &router,
    &token,
    "env_missing",
    "API_KEY",
    json!({"version":1}),
  )
  .await;
  assert_eq!(status, 404);

  // Ciphertext that no longer authenticates is refused and nothing is written.
  sqlx::query(
    "UPDATE secret_versions SET ciphertext=X'00112233' WHERE key='API_KEY' AND version=1",
  )
  .execute(state.db.pool())
  .await
  .unwrap();
  let (status, _) = rollback(&router, &token, &id, "API_KEY", json!({"version":1})).await;
  assert_eq!(status, 500);

  let page = history(&router, &token, &id, "API_KEY", "").await;
  assert_eq!(versions(&page), [2, 1]);
  assert_eq!(
    runtime_value(&router, &token, &id, "API_KEY").await,
    "value-two"
  );
  let rolled_back: i64 =
    sqlx::query_scalar("SELECT COUNT(*) FROM audit_events WHERE action='secret.rolled_back'")
      .fetch_one(state.db.pool())
      .await
      .unwrap();
  assert_eq!(rolled_back, 0);
  state.db.close().await;
}

#[tokio::test]
async fn retention_keeps_values_for_the_newest_versions_only() {
  let (_dir, state, router, token, id) = admin_environment().await;
  for value in 1..=22 {
    put_secret(&router, &token, &id, "API_KEY", &format!("value-{value}")).await;
  }
  let page = history(&router, &token, &id, "API_KEY", "").await;
  assert_eq!(page["items"].as_array().unwrap().len(), 22);
  let expired: Vec<i64> = page["items"]
    .as_array()
    .unwrap()
    .iter()
    .filter(|item| item["restorable"] == false)
    .map(|item| item["version"].as_i64().unwrap())
    .collect();
  assert_eq!(expired, [2, 1]);
  let stored: i64 = sqlx::query_scalar(
    "SELECT COUNT(*) FROM secret_versions WHERE key='API_KEY' AND ciphertext IS NOT NULL",
  )
  .fetch_one(state.db.pool())
  .await
  .unwrap();
  assert_eq!(stored, 20);

  for path in [
    format!("/api/v1/environments/{id}/secrets/API_KEY/versions/1/reveal"),
    format!("/api/v1/environments/{id}/secrets/API_KEY/rollback"),
  ] {
    let body = path.ends_with("rollback").then(|| json!({"version":1}));
    let (status, response, _) = call(&router, "POST", &path, Some(&token), body).await;
    assert_eq!(status, 410, "{path}: {response}");
    assert!(response["error"]["SECRET_VERSION_EXPIRED"].is_string());
  }
  let (status, _) = rollback(&router, &token, &id, "API_KEY", json!({"version":3})).await;
  assert_eq!(status, 200);
  assert_eq!(
    runtime_value(&router, &token, &id, "API_KEY").await,
    "value-3"
  );
  state.db.close().await;
}

#[tokio::test]
async fn deleting_a_secret_removes_its_history() {
  let (_dir, state, router, token, id) = admin_environment().await;
  for key in ["DELETED", "REPLACED", "KEPT"] {
    put_secret(&router, &token, &id, key, "value-one").await;
    put_secret(&router, &token, &id, key, "value-two").await;
  }
  let (status, _, _) = call(
    &router,
    "DELETE",
    &format!("/api/v1/environments/{id}/secrets/DELETED"),
    Some(&token),
    None,
  )
  .await;
  assert_eq!(status, 200);
  let base = format!("/api/v1/environments/{id}/secrets/import");
  let request = json!({"mode":"replace","entries":[{"key":"KEPT","value":"value-two"}]});
  let (_, preview, _) = call(
    &router,
    "POST",
    &base,
    Some(&token),
    Some(json!({"mode":"replace","dryRun":true,"entries":request["entries"]})),
  )
  .await;
  let mut apply = request.clone();
  apply["expectedRevision"] = preview["data"]["revision"].clone();
  let (status, body, _) = call(&router, "POST", &base, Some(&token), Some(apply)).await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["deletedKeys"], json!(["REPLACED"]));

  let keys: Vec<(String, i64)> = sqlx::query_as(
    "SELECT key,COUNT(*) FROM secret_versions WHERE environment_id=? GROUP BY key ORDER BY key",
  )
  .bind(&id)
  .fetch_all(state.db.pool())
  .await
  .unwrap();
  assert_eq!(keys, [("KEPT".to_owned(), 2)]);

  // A secret created again under the same key starts a fresh history.
  put_secret(&router, &token, &id, "DELETED", "fresh").await;
  let page = history(&router, &token, &id, "DELETED", "").await;
  assert_eq!(versions(&page), [1]);
  state.db.close().await;
}

#[tokio::test]
async fn rollback_and_version_reveal_need_a_confirmed_project_manager() {
  let (_dir, state, router, token, id) = admin_environment().await;
  put_secret(&router, &token, &id, "API_KEY", "value-one").await;
  put_secret(&router, &token, &id, "API_KEY", "value-two").await;
  let rollback_path = format!("/api/v1/environments/{id}/secrets/API_KEY/rollback");
  let reveal_path = format!("/api/v1/environments/{id}/secrets/API_KEY/versions/1/reveal");
  let request = json!({"version":1});

  let (cookie, csrf) = browser_login(&router).await;
  for path in [&rollback_path, &reveal_path] {
    let (status, body) = browser_call(&router, path, &cookie, None, request.clone()).await;
    assert_eq!(status, 403, "{path} without CSRF: {body}");
  }
  expire_confirmation(&state).await;
  for path in [&rollback_path, &reveal_path] {
    let (status, body) = browser_call(&router, path, &cookie, Some(&csrf), request.clone()).await;
    assert_eq!(status, 403, "{path}: {body}");
    assert!(body["error"]["RECENT_AUTHENTICATION_REQUIRED"].is_string());
  }
  let (status, _) = browser_call(
    &router,
    "/api/v1/auth/reauthenticate",
    &cookie,
    Some(&csrf),
    json!({"password":"correct-horse-123"}),
  )
  .await;
  assert_eq!(status, 200);
  let (status, body) =
    browser_call(&router, &reveal_path, &cookie, Some(&csrf), request.clone()).await;
  assert_eq!(status, 200, "{body}");
  let (status, body) = browser_call(
    &router,
    &rollback_path,
    &cookie,
    Some(&csrf),
    request.clone(),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["version"], 3);

  // Machine credentials can read metadata at most, never values.
  let (_, runner, _) = call(
    &router,
    "POST",
    &format!("/api/v1/environments/{id}/tokens"),
    Some(&token),
    Some(json!({"name":"deploy","role":"runner"})),
  )
  .await;
  let runner = runner["data"]["plaintextToken"]
    .as_str()
    .or_else(|| runner["data"]["token"]["plaintextToken"].as_str())
    .map(str::to_owned)
    .unwrap_or_else(|| panic!("runner token: {runner}"));
  let agent_token =
    metadata_agent_token(&router, &cookie, &csrf, "history-agent", "concurrency-test").await;
  let agent_token = agent_token.as_str();

  history(&router, agent_token, &id, "API_KEY", "").await;
  for credential in [runner.as_str(), agent_token] {
    for path in [&rollback_path, &reveal_path] {
      let (status, body, _) = call(
        &router,
        "POST",
        path,
        Some(credential),
        Some(request.clone()),
      )
      .await;
      assert_eq!(status, 403, "{path}: {body}");
    }
  }
  let page = history(&router, &token, &id, "API_KEY", "").await;
  assert_eq!(versions(&page), [3, 2, 1]);
  state.db.close().await;
}
