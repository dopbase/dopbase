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

async fn compare(
  router: &Router,
  auth: As<'_>,
  base: &str,
  with: &[&str],
) -> (u16, Value) {
  send(
    router,
    "GET",
    &format!(
      "/api/v1/environments/{base}/compare?with={}",
      with.join(",")
    ),
    auth,
    None,
  )
  .await
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

fn row<'a>(
  body: &'a Value,
  key: &str,
) -> &'a Value {
  body["data"]["keys"]
    .as_array()
    .unwrap()
    .iter()
    .find(|row| row["key"] == key)
    .unwrap_or_else(|| panic!("{key} missing from {body}"))
}

#[tokio::test]
async fn compare_lines_up_keys_and_says_only_whether_values_match() {
  let (_dir, state, router) = test_app().await;
  let (root, env) = instance(&state, &router).await;
  let (dev, prod) = (&env["payments/dev"], &env["payments/prod"]);
  put(&router, &root, prod, "API_URL", "https://api.example.com").await;
  put(&router, &root, dev, "API_URL", "https://api.example.com").await;
  put(&router, &root, prod, "STRIPE_KEY", "sk_live_prod").await;
  put(&router, &root, dev, "STRIPE_KEY", "sk_test_dev").await;
  put(&router, &root, dev, "STRIPE_KEY", "sk_test_dev_2").await;
  put(&router, &root, dev, "ONLY_DEV", "x").await;
  put(&router, &root, prod, "ONLY_PROD", "y").await;

  let (status, body) = compare(&router, root.cli(), prod, &[dev]).await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["base"]["name"], "prod");
  assert_eq!(body["data"]["base"]["valuesCompared"], true);
  assert_eq!(body["data"]["environments"][0]["name"], "dev");
  assert_eq!(body["data"]["environments"][0]["valuesCompared"], true);
  let keys: Vec<&str> = body["data"]["keys"]
    .as_array()
    .unwrap()
    .iter()
    .map(|row| row["key"].as_str().unwrap())
    .collect();
  assert_eq!(keys, ["API_URL", "ONLY_DEV", "ONLY_PROD", "STRIPE_KEY"]);

  assert_eq!(row(&body, "API_URL")["others"][0]["sameAsBase"], true);
  let stripe = row(&body, "STRIPE_KEY");
  assert_eq!(stripe["base"], 1);
  assert_eq!(stripe["others"][0]["version"], 2);
  assert_eq!(stripe["others"][0]["sameAsBase"], false);
  let only_dev = row(&body, "ONLY_DEV");
  assert!(only_dev["base"].is_null());
  assert_eq!(only_dev["others"][0]["version"], 1);
  assert!(only_dev["others"][0]["sameAsBase"].is_null());
  assert!(row(&body, "ONLY_PROD")["others"][0]["version"].is_null());

  // Nothing from a value travels back.
  let raw = body.to_string();
  for value in ["api.example.com", "sk_live_prod", "sk_test_dev"] {
    assert!(!raw.contains(value), "{value} leaked: {raw}");
  }
  state.db.close().await;
}

#[tokio::test]
async fn names_only_shows_which_keys_exist_without_comparing_values() {
  let (_dir, state, router) = test_app().await;
  let (root, env) = instance(&state, &router).await;
  let (dev, prod) = (&env["payments/dev"], &env["payments/prod"]);
  put(&router, &root, prod, "API_URL", "same").await;
  put(&router, &root, dev, "API_URL", "same").await;
  let member = person(&router, &root, "member@example.com", "member").await;
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

  let (status, body) = compare(&router, member.cli(), dev, &[prod]).await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["base"]["valuesCompared"], true);
  assert_eq!(body["data"]["environments"][0]["valuesCompared"], false);
  assert_eq!(body["data"]["environments"][0]["level"], "metadata");
  let api = row(&body, "API_URL");
  assert_eq!(api["others"][0]["version"], 1);
  assert!(api["others"][0]["sameAsBase"].is_null());

  // With names only on the base, no column compares values.
  let (status, body) = compare(&router, member.cli(), prod, &[dev]).await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["base"]["valuesCompared"], false);
  assert_eq!(body["data"]["environments"][0]["valuesCompared"], false);
  state.db.close().await;
}

#[tokio::test]
async fn compare_takes_at_most_three_environments_from_the_same_project() {
  let (_dir, state, router) = test_app().await;
  let (root, env) = instance(&state, &router).await;
  let (dev, prod) = (env["payments/dev"].clone(), env["payments/prod"].clone());
  let staging = add_environment(&router, &root, "staging").await;
  let qa = add_environment(&router, &root, "qa").await;
  let uat = add_environment(&router, &root, "uat").await;

  let (status, body) = compare(&router, root.cli(), &prod, &[&dev, &staging, &qa]).await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["environments"].as_array().unwrap().len(), 3);

  let (status, body) = compare(&router, root.cli(), &prod, &[&dev, &staging, &qa, &uat]).await;
  assert_eq!(status, 422, "{body}");
  assert!(
    body["error"]["COMPARE_LIMIT_EXCEEDED"].is_string(),
    "{body}"
  );

  for with in [
    vec![prod.as_str()],
    vec![&dev, &dev],
    vec![&env["search/dev"]],
  ] {
    let (status, body) = compare(&router, root.cli(), &prod, &with).await;
    assert_eq!(status, 422, "{with:?}: {body}");
    assert!(
      body["error"]["COMPARE_ENVIRONMENT_INVALID"].is_string(),
      "{body}"
    );
  }
  state.db.close().await;
}

#[tokio::test]
async fn copy_writes_a_new_version_on_the_server_and_is_audited() {
  let (_dir, state, router) = test_app().await;
  let (root, env) = instance(&state, &router).await;
  let (dev, prod) = (&env["payments/dev"], &env["payments/prod"]);
  put(&router, &root, dev, "FEATURE_FLAGS", "beta,search").await;
  let copy = |target: &str, key: &str, body: Value| {
    let path = format!("/api/v1/environments/{target}/secrets/{key}/copy");
    let router = router.clone();
    let auth = root.browser();
    async move { send(&router, "POST", &path, auth, Some(body)).await }
  };

  let (status, body) = copy(prod, "FEATURE_FLAGS", json!({ "from": dev })).await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["version"], 1);
  assert!(!body.to_string().contains("beta,search"), "{body}");

  let (status, body) = send(
    &router,
    "POST",
    &format!("/api/v1/environments/{prod}/secrets/FEATURE_FLAGS/reveal"),
    root.cli(),
    None,
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["value"], "beta,search");

  // The caller saw no value in prod, but prod has one now.
  let (status, body) = copy(prod, "FEATURE_FLAGS", json!({ "from": dev })).await;
  assert_eq!(status, 409, "{body}");
  assert!(
    body["error"]["SECRET_VERSION_CONFLICT"].is_string(),
    "{body}"
  );
  let (status, body) = copy(
    prod,
    "FEATURE_FLAGS",
    json!({ "from": dev, "expectedVersion": 1 }),
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["version"], 2);

  let (status, body) = send(
    &router,
    "GET",
    "/api/v1/audit-events?action=secret.copied",
    root.cli(),
    None,
  )
  .await;
  assert_eq!(status, 200, "{body}");
  let event = &body["data"]["items"][0];
  assert_eq!(event["environmentId"], prod.as_str());
  assert_eq!(event["metadata"]["fromEnvironmentId"], dev.as_str());
  assert_eq!(event["metadata"]["key"], "FEATURE_FLAGS");
  assert!(!body.to_string().contains("beta,search"), "{body}");
  state.db.close().await;
}

#[tokio::test]
async fn copy_needs_read_on_the_source_and_write_on_the_target() {
  let (_dir, state, router) = test_app().await;
  let (root, env) = instance(&state, &router).await;
  let (dev, prod, search) = (
    &env["payments/dev"],
    &env["payments/prod"],
    &env["search/dev"],
  );
  put(&router, &root, dev, "A", "1").await;
  put(&router, &root, prod, "B", "2").await;
  put(&router, &root, search, "C", "3").await;
  let member = person(&router, &root, "member@example.com", "member").await;
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
  let copy = |target: &str, key: &str, from: &str| {
    let path = format!("/api/v1/environments/{target}/secrets/{key}/copy");
    let router = router.clone();
    let auth = member.browser();
    let body = json!({ "from": from });
    async move { send(&router, "POST", &path, auth, Some(body)).await }
  };

  // Names only on prod: it can't be read from, or written to.
  let (status, body) = copy(dev, "B", prod).await;
  assert_eq!(status, 403, "{body}");
  let (status, body) = copy(prod, "A", dev).await;
  assert_eq!(status, 403, "{body}");

  let (status, body) = send(
    &router,
    "POST",
    &format!("/api/v1/environments/{prod}/secrets/C/copy"),
    root.browser(),
    Some(json!({ "from": search })),
  )
  .await;
  assert_eq!(status, 422, "{body}");
  assert!(body["error"]["COPY_SOURCE_INVALID"].is_string(), "{body}");
  let (status, body) = send(
    &router,
    "POST",
    &format!("/api/v1/environments/{dev}/secrets/A/copy"),
    root.browser(),
    Some(json!({ "from": dev })),
  )
  .await;
  assert_eq!(status, 422, "{body}");
  state.db.close().await;
}
