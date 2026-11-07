use super::grants_access::{As, Person, grant, instance, person, send};
use super::*;

async fn put(
  router: &Router,
  who: &Person,
  environment: &str,
  key: &str,
  value: &str,
) {
  let (status, body) = send(
    router,
    "PUT",
    &format!("/api/v1/environments/{environment}/secrets/{key}"),
    who.cli(),
    Some(json!({ "value": value })),
  )
  .await;
  assert_eq!(status, 200, "{body}");
}

async fn add_environment(
  router: &Router,
  root: &Person,
  name: &str,
) -> String {
  let (status, body) = send(
    router,
    "POST",
    "/api/v1/projects/payments/environments",
    root.cli(),
    Some(json!({ "name": name })),
  )
  .await;
  assert_eq!(status, 201, "{body}");
  body["data"]["id"].as_str().unwrap().to_owned()
}

async fn copy_many(
  router: &Router,
  auth: As<'_>,
  target: &str,
  body: Value,
) -> (u16, Value) {
  send(
    router,
    "POST",
    &format!("/api/v1/environments/{target}/copy-secrets"),
    auth,
    Some(body),
  )
  .await
}

async fn reveal(
  router: &Router,
  root: &Person,
  environment: &str,
  key: &str,
) -> Value {
  let (status, body) = send(
    router,
    "POST",
    &format!("/api/v1/environments/{environment}/secrets/{key}/reveal"),
    root.cli(),
    None,
  )
  .await;
  assert_eq!(status, 200, "{body}");
  body["data"]["value"].clone()
}

#[tokio::test]
async fn copy_many_creates_keys_with_or_without_values_and_skips_existing() {
  let (_dir, state, router) = test_app().await;
  let (root, env) = instance(&state, &router).await;
  let dev = &env["payments/dev"];
  put(&router, &root, dev, "API_KEY", "dev-key").await;
  put(&router, &root, dev, "DATABASE_URL", "postgres://dev").await;
  put(&router, &root, dev, "LOG_LEVEL", "debug").await;
  let staging = add_environment(&router, &root, "staging").await;
  put(&router, &root, &staging, "LOG_LEVEL", "info").await;

  // Keys only: every key starts empty, and staging's own LOG_LEVEL stays.
  let (status, body) = copy_many(
    &router,
    root.browser(),
    &staging,
    json!({ "from": dev, "keys": ["DATABASE_URL", "API_KEY", "LOG_LEVEL", "API_KEY"] }),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(
    body["data"]["copiedKeys"],
    json!(["API_KEY", "DATABASE_URL"])
  );
  assert_eq!(body["data"]["skippedKeys"], json!(["LOG_LEVEL"]));
  assert_eq!(reveal(&router, &root, &staging, "API_KEY").await, "");
  assert_eq!(reveal(&router, &root, &staging, "LOG_LEVEL").await, "info");

  let (status, body) = send(
    &router,
    "GET",
    "/api/v1/environments?project=payments",
    root.cli(),
    None,
  )
  .await;
  assert_eq!(status, 200, "{body}");
  let count = |name: &str| {
    body["data"]
      .as_array()
      .unwrap()
      .iter()
      .find(|environment| environment["name"] == name)
      .unwrap()["secretCount"]
      .clone()
  };
  assert_eq!(count("dev"), 3);
  assert_eq!(count("staging"), 3);
  assert_eq!(count("prod"), 0);
  // A single environment counts too, by id or by name.
  for path in [
    format!("/api/v1/environments/{staging}"),
    "/api/v1/environments/resolve?reference=payments/staging".to_owned(),
    format!("/api/v1/environments/resolve?reference={staging}"),
  ] {
    let (status, body) = send(&router, "GET", &path, root.cli(), None).await;
    assert_eq!(status, 200, "{path}: {body}");
    assert_eq!(body["data"]["secretCount"], 3, "{path}");
  }

  // With values: copied on the server, never returned.
  let qa = add_environment(&router, &root, "qa").await;
  let (status, body) = copy_many(
    &router,
    root.browser(),
    &qa,
    json!({ "from": dev, "keys": ["DATABASE_URL"], "values": true }),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert!(!body.to_string().contains("postgres://dev"), "{body}");
  assert_eq!(
    reveal(&router, &root, &qa, "DATABASE_URL").await,
    "postgres://dev"
  );

  let (status, body) = send(
    &router,
    "GET",
    "/api/v1/audit-events?action=secret.bulk_copied",
    root.cli(),
    None,
  )
  .await;
  assert_eq!(status, 200, "{body}");
  let events = body["data"]["items"].as_array().unwrap();
  assert_eq!(events.len(), 2, "{body}");
  let keys_only = events
    .iter()
    .find(|event| event["environmentId"] == staging.as_str())
    .unwrap();
  assert_eq!(keys_only["metadata"]["fromEnvironmentId"], dev.as_str());
  assert_eq!(keys_only["metadata"]["values"], false);
  assert_eq!(
    keys_only["metadata"]["copied"],
    json!({ "API_KEY": 1, "DATABASE_URL": 1 })
  );
  assert_eq!(keys_only["metadata"]["skipped"], json!(["LOG_LEVEL"]));
  assert!(!body.to_string().contains("postgres://dev"), "{body}");
  state.db.close().await;
}

#[tokio::test]
async fn copy_many_needs_names_only_for_keys_and_read_for_values() {
  let (_dir, state, router) = test_app().await;
  let (root, env) = instance(&state, &router).await;
  let (dev, prod, search) = (
    &env["payments/dev"],
    &env["payments/prod"],
    &env["search/dev"],
  );
  put(&router, &root, prod, "STRIPE_KEY", "sk_live").await;
  put(&router, &root, search, "INDEX", "products").await;
  let member = person(&router, &root, "member@dopbase.com", "member").await;
  grant(
    &router,
    &root,
    ("user", &member.id),
    ("environment", Some(dev)),
    "write",
  )
  .await;
  grant(
    &router,
    &root,
    ("user", &member.id),
    ("environment", Some(prod)),
    "metadata",
  )
  .await;

  // Names only on prod is enough to copy its keys, not its values.
  let (status, body) = copy_many(
    &router,
    member.browser(),
    dev,
    json!({ "from": prod, "keys": ["STRIPE_KEY"], "values": true }),
  )
  .await;
  assert_eq!(status, 403, "{body}");
  let (status, body) = copy_many(
    &router,
    member.browser(),
    dev,
    json!({ "from": prod, "keys": ["STRIPE_KEY"] }),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(reveal(&router, &root, dev, "STRIPE_KEY").await, "");

  // Writing needs Write on the target.
  let (status, body) = copy_many(
    &router,
    member.browser(),
    prod,
    json!({ "from": dev, "keys": ["STRIPE_KEY"] }),
  )
  .await;
  assert_eq!(status, 403, "{body}");

  let staging = add_environment(&router, &root, "staging").await;
  let refused = [
    (
      json!({ "from": prod, "keys": [] }),
      422,
      "COPY_KEYS_REQUIRED",
    ),
    (
      json!({ "from": staging, "keys": ["STRIPE_KEY"] }),
      422,
      "COPY_SOURCE_INVALID",
    ),
    (
      json!({ "from": search, "keys": ["INDEX"] }),
      422,
      "COPY_SOURCE_INVALID",
    ),
    (
      json!({ "from": prod, "keys": ["MISSING"] }),
      404,
      "SECRET_NOT_FOUND",
    ),
  ];
  for (request, expected, code) in refused {
    let (status, body) = copy_many(&router, root.browser(), &staging, request).await;
    assert_eq!(status, expected, "{body}");
    assert!(body["error"][code].is_string(), "{body}");
  }
  state.db.close().await;
}

#[tokio::test]
async fn copy_many_copies_the_source_as_it_is_when_it_writes() {
  let (_dir, state, router) = test_app().await;
  let (root, env) = instance(&state, &router).await;
  let dev = &env["payments/dev"];
  put(&router, &root, dev, "DB_PASSWORD", "old").await;
  put(&router, &root, dev, "DB_PASSWORD", "rotated").await;
  put(&router, &root, dev, "GONE", "soon").await;
  let staging = add_environment(&router, &root, "staging").await;

  // A key deleted from the source is not found, and nothing is copied.
  let (status, _) = send(
    &router,
    "DELETE",
    &format!("/api/v1/environments/{dev}/secrets/GONE"),
    root.cli(),
    None,
  )
  .await;
  assert_eq!(status, 200);
  let (status, body) = copy_many(
    &router,
    root.browser(),
    &staging,
    json!({ "from": dev, "keys": ["DB_PASSWORD", "GONE"], "values": true }),
  )
  .await;
  assert_eq!(status, 404, "{body}");
  let (_, list) = send(
    &router,
    "GET",
    &format!("/api/v1/environments/{staging}/secrets"),
    root.cli(),
    None,
  )
  .await;
  assert_eq!(list["data"], json!([]));

  // The copy and its audit event carry the source's current version.
  let (status, body) = copy_many(
    &router,
    root.browser(),
    &staging,
    json!({ "from": dev, "keys": ["DB_PASSWORD"], "values": true }),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(
    reveal(&router, &root, &staging, "DB_PASSWORD").await,
    "rotated"
  );
  let (_, events) = send(
    &router,
    "GET",
    "/api/v1/audit-events?action=secret.bulk_copied",
    root.cli(),
    None,
  )
  .await;
  assert_eq!(
    events["data"]["items"][0]["metadata"]["copied"],
    json!({ "DB_PASSWORD": 2 })
  );
  state.db.close().await;
}

#[tokio::test]
async fn copies_stop_at_the_environment_limits() {
  let (_dir, state, router) = test_app().await;
  let (root, env) = instance(&state, &router).await;
  let dev = &env["payments/dev"];
  let block = "y".repeat(64 * 1024);
  put(&router, &root, dev, "EXTRA", "x").await;
  put(&router, &root, dev, "K00", &block).await;
  let import = |target: String, entries: Vec<Value>| {
    let router = router.clone();
    let auth = root.cli();
    async move {
      let (status, body) = send(
        &router,
        "POST",
        &format!("/api/v1/environments/{target}/secrets/import"),
        auth,
        Some(json!({ "mode": "merge", "entries": entries })),
      )
      .await;
      assert_eq!(status, 200, "{body}");
    }
  };
  let copy_one = |target: String, key: &'static str, expected: Option<i64>| {
    let router = router.clone();
    let auth = root.browser();
    let body = match expected {
      Some(version) => json!({ "from": dev, "expectedVersion": version }),
      None => json!({ "from": dev }),
    };
    async move {
      send(
        &router,
        "POST",
        &format!("/api/v1/environments/{target}/secrets/{key}/copy"),
        auth,
        Some(body),
      )
      .await
    }
  };
  let limited = |(status, body): (u16, Value)| {
    assert_eq!(status, 422, "{body}");
    assert!(body["error"]["SECRET_LIMIT_EXCEEDED"].is_string(), "{body}");
  };
  let keys = |target: String| {
    let state = state.clone();
    async move {
      sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM secrets WHERE environment_id = ?")
        .bind(target)
        .fetch_one(state.db.pool())
        .await
        .unwrap()
    }
  };

  // Staging holds exactly the 2 MiB a collection may.
  let staging = add_environment(&router, &root, "staging").await;
  import(
    staging.clone(),
    (0..32)
      .map(|index| json!({ "key": format!("K{index:02}"), "value": block }))
      .collect(),
  )
  .await;

  // One more byte is refused, in bulk and alone, and nothing is written.
  limited(
    copy_many(
      &router,
      root.browser(),
      &staging,
      json!({ "from": dev, "keys": ["EXTRA"], "values": true }),
    )
    .await,
  );
  limited(copy_one(staging.clone(), "EXTRA", None).await);
  assert_eq!(keys(staging.clone()).await, 32);
  // Replacing a value with one of the same size keeps it within the cap.
  let (status, body) = copy_one(staging.clone(), "K00", Some(1)).await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["version"], 2);

  // A full environment takes no new key from a single copy either.
  let qa = add_environment(&router, &root, "qa").await;
  import(
    qa.clone(),
    (0..1000)
      .map(|index| json!({ "key": format!("K{index:04}"), "value": "v" }))
      .collect(),
  )
  .await;
  limited(copy_one(qa.clone(), "EXTRA", None).await);
  assert_eq!(keys(qa).await, 1000);
  state.db.close().await;
}
