use super::audit_scope::with_role;
use super::grants_access::{Person, instance, send};
use super::*;

async fn status(
  router: &Router,
  who: &Person,
) -> Value {
  let (status, body) = send(router, "GET", "/api/v1/status", who.cli(), None).await;
  assert_eq!(status, 200, "{body}");
  body["data"].clone()
}

fn check<'a>(
  data: &'a Value,
  id: &str,
) -> &'a Value {
  data["checks"]
    .as_array()
    .unwrap()
    .iter()
    .find(|check| check["id"] == id)
    .unwrap_or_else(|| panic!("no {id} check in {data}"))
}

async fn put_secret(
  router: &Router,
  root: &Person,
  environment: &str,
  key: &str,
) {
  let (status, body) = send(
    router,
    "PUT",
    &format!("/api/v1/environments/{environment}/secrets/{key}"),
    root.cli(),
    Some(json!({ "value": "s3cr3t" })),
  )
  .await;
  assert_eq!(status, 200, "{body}");
}

#[tokio::test]
async fn status_reports_every_check_with_the_readings_behind_the_cards() {
  let (_dir, state, router) = test_app().await;
  let (root, _) = instance(&state, &router).await;
  let data = status(&router, &root).await;

  assert!(data.get("databaseHealth").is_none(), "{data}");
  assert!(data.get("keyAvailability").is_none(), "{data}");
  assert_eq!(data["platform"]["os"], std::env::consts::OS);
  assert_eq!(data["platform"]["arch"], std::env::consts::ARCH);

  let ids: Vec<&str> = data["checks"]
    .as_array()
    .unwrap()
    .iter()
    .map(|check| check["id"].as_str().unwrap())
    .collect();
  for id in [
    "database.ping",
    "master_key.file",
    "audit.chain",
    "backup.age",
    "tokens.expiring",
    "audit.retention",
    "config.public_url",
    "config.https",
    "config.docs",
    "people.managers",
  ] {
    assert!(ids.contains(&id), "missing {id} in {ids:?}");
  }
  // Most severe first.
  let rank = |status: &str| {
    ["crit", "warn", "tip", "ok"]
      .iter()
      .position(|s| *s == status)
  };
  let ranks: Vec<_> = data["checks"]
    .as_array()
    .unwrap()
    .iter()
    .map(|check| rank(check["status"].as_str().unwrap()).unwrap())
    .collect();
  assert!(ranks.is_sorted(), "{ranks:?}");
  let summary = &data["summary"];
  let total = ["critical", "warnings", "tips", "passed"]
    .iter()
    .map(|key| summary[key].as_u64().unwrap())
    .sum::<u64>();
  assert_eq!(total as usize, ids.len());

  assert_eq!(check(&data, "database.ping")["status"], "ok");
  assert_eq!(data["health"]["database"]["status"], "ok");
  assert!(data["health"]["database"]["sizeBytes"].as_u64().unwrap() > 0);
  assert_eq!(check(&data, "master_key.file")["status"], "ok");
  assert_eq!(data["health"]["masterKey"]["file"], "matches");
  assert_eq!(check(&data, "audit.chain")["status"], "ok");
  assert_eq!(data["health"]["auditLog"]["intact"], true);
  // The test app turns the API docs on and keeps no audit retention.
  assert_eq!(check(&data, "config.docs")["status"], "tip");
  assert_eq!(check(&data, "audit.retention")["status"], "tip");
  assert!(
    check(&data, "audit.retention")["fixPath"]
      .as_str()
      .unwrap()
      .starts_with("https://")
  );
  // Only root can manage the instance until an admin exists.
  assert_eq!(check(&data, "people.managers")["status"], "tip");
  with_role(&router, &root, "second@example.com", "rol_admin").await;
  let data = status(&router, &root).await;
  assert_eq!(check(&data, "people.managers")["status"], "ok");
  assert_eq!(check(&data, "people.managers")["value"], "root + 1 admin");
  if cfg!(unix) {
    assert!(data["health"]["disk"]["totalBytes"].as_u64().unwrap() > 0);
    assert!(
      check(&data, "disk.space")["value"]
        .as_str()
        .unwrap()
        .ends_with(" free")
    );
  }
  state.db.close().await;
}

#[tokio::test]
async fn backup_checks_follow_age_changes_and_downloads() {
  let (_dir, state, router) = test_app().await;
  let (root, environments) = instance(&state, &router).await;
  let dev = &environments["payments/dev"];

  // No secrets yet: a backup is only suggested.
  let data = status(&router, &root).await;
  assert_eq!(check(&data, "backup.age")["status"], "tip");
  assert!(
    data["checks"]
      .as_array()
      .unwrap()
      .iter()
      .all(|check| check["id"] != "backup.offsite")
  );

  put_secret(&router, &root, dev, "API_KEY").await;
  let data = status(&router, &root).await;
  assert_eq!(check(&data, "backup.age")["status"], "crit");
  assert_eq!(check(&data, "backup.age")["fixPath"], "/backups");

  let (status_code, created) = send(
    &router,
    "POST",
    "/api/v1/backups",
    root.cli(),
    Some(json!({})),
  )
  .await;
  assert_eq!(status_code, 201, "{created}");
  let key = created["data"]["key"].as_str().unwrap().to_owned();
  let data = status(&router, &root).await;
  assert_eq!(check(&data, "backup.age")["status"], "ok");
  assert_eq!(
    check(&data, "backup.age")["title"],
    "Latest backup has every change"
  );
  assert_eq!(check(&data, "backup.age")["value"], "made today");
  assert_eq!(check(&data, "backup.offsite")["status"], "warn");

  let (status_code, _) = send(
    &router,
    "GET",
    &format!("/api/v1/backups/{key}"),
    root.cli(),
    None,
  )
  .await;
  assert_eq!(status_code, 200);
  let data = status(&router, &root).await;
  assert_eq!(check(&data, "backup.offsite")["status"], "ok");
  assert_eq!(check(&data, "backup.offsite")["value"], "today");

  // Twelve days on, with a secret changed since, the backup is stale.
  let archive = state.config.data_dir.join("backups").join(&key);
  let twelve_days_ago =
    std::time::SystemTime::now() - std::time::Duration::from_secs(12 * 24 * 60 * 60);
  std::fs::File::options()
    .write(true)
    .open(&archive)
    .unwrap()
    .set_modified(twelve_days_ago)
    .unwrap();
  let data = status(&router, &root).await;
  let age = check(&data, "backup.age");
  assert_eq!(age["status"], "crit");
  assert_eq!(age["title"], "No backup in 12 days");
  assert!(
    age["detail"]
      .as_str()
      .unwrap()
      .ends_with("Secrets changed 1 time since then."),
    "{age}"
  );
  state.db.close().await;
}

#[tokio::test]
async fn master_key_check_compares_the_file_with_the_key_in_use() {
  let (_dir, state, router) = test_app().await;
  let (root, _) = instance(&state, &router).await;
  let path = state.config.master_key.path.clone();
  let original = std::fs::read(&path).unwrap();

  std::fs::write(&path, [7_u8; 32]).unwrap();
  let data = status(&router, &root).await;
  assert_eq!(check(&data, "master_key.file")["status"], "crit");
  assert_eq!(
    check(&data, "master_key.file")["title"],
    "The key file no longer matches the key in use"
  );
  assert_eq!(data["health"]["masterKey"]["file"], "changed");
  assert!(data["summary"]["critical"].as_u64().unwrap() >= 1);

  std::fs::remove_file(&path).unwrap();
  let data = status(&router, &root).await;
  assert_eq!(data["health"]["masterKey"]["file"], "missing");
  assert_eq!(check(&data, "master_key.file")["fixPath"], "/backups");

  std::fs::write(&path, original).unwrap();
  let data = status(&router, &root).await;
  assert_eq!(data["health"]["masterKey"]["file"], "matches");
  state.db.close().await;
}

#[tokio::test]
async fn expiring_tokens_are_named_only_to_viewers_who_manage_tokens() {
  let (_dir, state, router) = test_app().await;
  let (root, environments) = instance(&state, &router).await;
  let (status_code, body) = send(
    &router,
    "POST",
    &format!(
      "/api/v1/environments/{}/tokens",
      environments["payments/prod"]
    ),
    root.cli(),
    Some(json!({"name":"deploy","role":"runner","expiresIn":"3d"})),
  )
  .await;
  assert_eq!(status_code, 201, "{body}");

  let data = status(&router, &root).await;
  let tokens = check(&data, "tokens.expiring");
  assert_eq!(tokens["status"], "warn");
  assert_eq!(tokens["title"], "1 token expires this week");
  assert!(
    tokens["detail"]
      .as_str()
      .unwrap()
      .starts_with("deploy · prod stops working on"),
    "{tokens}"
  );
  assert!(
    tokens["fixPath"]
      .as_str()
      .unwrap()
      .ends_with(&format!("/e/{}/tokens", environments["payments/prod"]))
  );

  // Auditors see the checks but not token names.
  let auditor = with_role(&router, &root, "auditor@example.com", "rol_auditor").await;
  let data = status(&router, &auditor).await;
  let tokens = check(&data, "tokens.expiring");
  assert!(
    tokens["detail"]
      .as_str()
      .unwrap()
      .starts_with("The first one stops working on"),
    "{tokens}"
  );
  assert!(!tokens.to_string().contains("deploy"));
  state.db.close().await;
}

#[tokio::test]
async fn one_broken_query_leaves_the_rest_of_the_status() {
  let (_dir, state, router) = test_app().await;
  let (root, _) = instance(&state, &router).await;
  sqlx::query("DROP TABLE personal_tokens")
    .execute(state.db.pool())
    .await
    .unwrap();

  let data = status(&router, &root).await;
  assert_eq!(data["activeAgentTokens"], Value::Null);
  assert_eq!(data["projects"], 2);
  let tokens = check(&data, "tokens.expiring");
  assert_eq!(tokens["status"], "warn");
  assert_eq!(tokens["title"], "Couldn't check token expiry");
  assert!(!tokens.to_string().contains("personal_tokens"));
  assert_eq!(check(&data, "database.ping")["status"], "ok");
  state.db.close().await;
}

#[tokio::test]
async fn activity_counts_reads_failed_sign_ins_refusals_and_sessions() {
  let (_dir, state, router) = test_app().await;
  let (root, environments) = instance(&state, &router).await;
  let dev = &environments["payments/dev"];
  put_secret(&router, &root, dev, "API_KEY").await;
  let (status_code, body) = send(
    &router,
    "POST",
    &format!("/api/v1/environments/{dev}/secrets/API_KEY/reveal"),
    root.cli(),
    None,
  )
  .await;
  assert_eq!(status_code, 200, "{body}");
  // Five wrong passwords lock the account; the sixth try is refused.
  for _ in 0..6 {
    call(
      &router,
      "POST",
      "/api/v1/auth/login",
      None,
      Some(json!({"email":"admin@example.com","password":"wrong-horse-123","sessionKind":"cli"})),
    )
    .await;
  }

  let data = status(&router, &root).await;
  let activity = &data["activity"];
  assert_eq!(activity["hours"].as_array().unwrap().len(), 24);
  assert_eq!(activity["secretReads"]["total"], 1);
  assert_eq!(activity["secretReads"]["hourly"][23], 1);
  assert_eq!(activity["secretReads"]["people"], 1);
  assert_eq!(activity["secretReads"]["runnerTokens"], 0);
  assert_eq!(activity["failedSignIns"]["total"], 5);
  assert_eq!(
    activity["failedSignIns"]["accounts"],
    json!(["admin@example.com"])
  );
  assert!(activity["failedSignIns"]["addresses"].is_array());
  assert_eq!(activity["rateLimited"]["total"], 1);
  assert_eq!(
    activity["rateLimited"]["hourly"].as_array().unwrap().len(),
    24
  );
  // Bootstrap and instance() leave root signed in both ways.
  assert!(activity["activeSessions"]["browser"].as_i64().unwrap() >= 1);
  assert!(activity["activeSessions"]["cli"].as_i64().unwrap() >= 1);

  // Without audit.read, who failed to sign in and from where stays hidden.
  let admin = with_role(&router, &root, "admin2@example.com", "rol_admin").await;
  sqlx::query(
    "DELETE FROM role_capabilities WHERE role_id = 'rol_admin' AND capability = 'audit.read'",
  )
  .execute(state.db.pool())
  .await
  .unwrap();
  let data = status(&router, &admin).await;
  assert_eq!(data["activity"]["failedSignIns"]["total"], 5);
  assert_eq!(data["activity"]["failedSignIns"]["accounts"], Value::Null);
  assert_eq!(data["activity"]["failedSignIns"]["addresses"], Value::Null);
  state.db.close().await;
}

#[tokio::test]
async fn checks_and_activity_need_instance_view() {
  let (_dir, state, router) = test_app().await;
  let (root, _) = instance(&state, &router).await;
  let member = with_role(&router, &root, "member@example.com", "rol_member").await;
  let data = status(&router, &member).await;
  assert_eq!(data["projects"], 2);
  for key in ["summary", "checks", "health", "activity"] {
    assert_eq!(data[key], Value::Null, "{key} leaked: {data}");
  }
  let (status_code, _) = send(
    &router,
    "POST",
    "/api/v1/status/checks/run",
    member.cli(),
    None,
  )
  .await;
  assert_eq!(status_code, 403);
  state.db.close().await;
}

#[tokio::test]
async fn run_checks_rewalks_the_cached_audit_chain_once_per_interval() {
  let (_dir, state, router) = test_app().await;
  let (root, _) = instance(&state, &router).await;
  let data = status(&router, &root).await;
  assert_eq!(check(&data, "audit.chain")["status"], "ok");

  // The guard trigger only stops application bugs, so an attacker drops it.
  sqlx::query("DROP TRIGGER audit_events_sealed_immutable")
    .execute(state.db.pool())
    .await
    .unwrap();
  sqlx::query(
    "UPDATE audit_events SET metadata = '{\"tampered\":true}' WHERE seq = (SELECT MIN(seq) FROM audit_events)",
  )
  .execute(state.db.pool())
  .await
  .unwrap();
  // The page reads the cached walk until it is refreshed.
  let data = status(&router, &root).await;
  assert_eq!(check(&data, "audit.chain")["status"], "ok");

  // The Admin UI needs its CSRF header.
  let (status_code, _) = send(
    &router,
    "POST",
    "/api/v1/status/checks/run",
    super::grants_access::As::Browser(&root.cookie, "wrong"),
    None,
  )
  .await;
  assert_eq!(status_code, 403);

  let (status_code, body) = send(
    &router,
    "POST",
    "/api/v1/status/checks/run",
    root.browser(),
    None,
  )
  .await;
  assert_eq!(status_code, 200, "{body}");
  let chain = check(&body["data"], "audit.chain");
  assert_eq!(chain["status"], "crit");
  assert_eq!(chain["fixPath"], "/audit");
  assert_eq!(body["data"]["health"]["auditLog"]["intact"], false);
  let data = status(&router, &root).await;
  assert_eq!(check(&data, "audit.chain")["status"], "crit");

  let (status_code, _) = send(
    &router,
    "POST",
    "/api/v1/status/checks/run",
    root.cli(),
    None,
  )
  .await;
  assert_eq!(status_code, 429);
  state.db.close().await;
}

/// Stands in for the GitHub latest-release API. Counts requests and answers
/// with whatever the test set last.
#[derive(Clone)]
struct MockReleases {
  url: String,
  reply: std::sync::Arc<std::sync::Mutex<(u16, Value)>>,
  hits: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl MockReleases {
  async fn start() -> Self {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mock = Self {
      url: format!("http://{}/releases/latest", listener.local_addr().unwrap()),
      reply: std::sync::Arc::new(std::sync::Mutex::new((500, Value::Null))),
      hits: std::sync::Arc::default(),
    };
    let state = mock.clone();
    let router = Router::new().route(
      "/releases/latest",
      axum::routing::get(move || {
        let state = state.clone();
        async move {
          state.hits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
          let (status, body) = state.reply.lock().unwrap().clone();
          (
            axum::http::StatusCode::from_u16(status).unwrap(),
            axum::Json(body),
          )
        }
      }),
    );
    tokio::spawn(async move {
      axum::serve(listener, router).await.unwrap();
    });
    mock
  }

  fn answer(
    &self,
    tag: &str,
  ) {
    *self.reply.lock().unwrap() = (
      200,
      json!({"tag_name": tag, "html_url": format!("https://example.test/releases/{tag}")}),
    );
  }

  fn fail(&self) {
    *self.reply.lock().unwrap() = (500, json!({"message": "down"}));
  }

  fn hits(&self) -> usize {
    self.hits.load(std::sync::atomic::Ordering::SeqCst)
  }
}

async fn app_with_update_check(
  enabled: bool
) -> (TempDir, app::state::AppState, Router, MockReleases) {
  let directory = TempDir::new().unwrap();
  let config = ServerConfig {
    data_dir: directory.path().to_path_buf(),
    database_url: format!("sqlite://{}", directory.path().join("dopbase.db").display()),
    master_key: app::config::MasterKeyConfig {
      provider: "file".into(),
      path: directory.path().join("master.key"),
    },
    update_check: enabled,
    ..ServerConfig::default()
  };
  let state = server::build_setup_state(config).await.unwrap();
  let mock = MockReleases::start().await;
  state.status.set_release_url(&mock.url);
  let router = server::router(state.clone());
  (directory, state, router, mock)
}

fn version_check(data: &Value) -> Option<&Value> {
  data["checks"]
    .as_array()
    .unwrap()
    .iter()
    .find(|check| check["id"] == "version.latest")
}

#[tokio::test]
async fn release_check_stays_off_until_turned_on() {
  let (_dir, state, router, mock) = app_with_update_check(false).await;
  let (root, _) = instance(&state, &router).await;
  mock.answer("v99.0.0");
  let data = status(&router, &root).await;
  assert!(version_check(&data).is_none(), "{data}");
  assert_eq!(data["latestRelease"], Value::Null);
  // Reading the status never calls out.
  assert_eq!(mock.hits(), 0);
  state.db.close().await;
}

/// Read the status until the release lookup that reading started has landed.
async fn status_after_release_lookup(
  router: &Router,
  who: &Person,
) -> Value {
  for _ in 0..100 {
    let data = status(router, who).await;
    if version_check(&data).is_some() {
      return data;
    }
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
  }
  panic!("the release lookup never finished");
}

#[tokio::test]
async fn reading_the_status_looks_up_releases_once_a_day() {
  let (_dir, state, router, mock) = app_with_update_check(true).await;
  let (root, _) = instance(&state, &router).await;
  assert_eq!(mock.hits(), 0, "nothing runs before the page is read");

  mock.answer("v99.0.0");
  let data = status_after_release_lookup(&router, &root).await;
  let check = version_check(&data).unwrap();
  assert_eq!(check["status"], "tip");
  assert_eq!(check["title"], "Dopbase 99.0.0 is out");
  assert_eq!(check["fixPath"], "https://example.test/releases/v99.0.0");
  assert_eq!(data["latestRelease"]["version"], "v99.0.0");
  assert_eq!(data["latestRelease"]["updateAvailable"], true);

  // A day hasn't passed, so more reads don't ask GitHub again.
  status(&router, &root).await;
  state.status.refresh_release_if_due().await;
  assert_eq!(mock.hits(), 1);
  state.db.close().await;
}

#[tokio::test]
async fn release_check_passes_on_the_running_version() {
  let (_dir, state, router, mock) = app_with_update_check(true).await;
  let (root, _) = instance(&state, &router).await;
  mock.answer(&format!("v{}", env!("CARGO_PKG_VERSION")));
  state.status.refresh_release().await;
  let data = status(&router, &root).await;
  let check = version_check(&data).unwrap();
  assert_eq!(check["status"], "ok");
  assert_eq!(check["title"], "Running the latest version");
  assert_eq!(check["value"], env!("CARGO_PKG_VERSION"));
  assert_eq!(data["latestRelease"]["updateAvailable"], false);
  state.db.close().await;
}

#[tokio::test]
async fn failed_release_lookup_keeps_the_last_answer() {
  let (_dir, state, router, mock) = app_with_update_check(true).await;
  let (root, _) = instance(&state, &router).await;
  mock.fail();
  state.status.refresh_release().await;
  let data = status(&router, &root).await;
  let check = version_check(&data).unwrap();
  assert_eq!(check["status"], "tip");
  assert_eq!(check["title"], "Couldn't check for a newer version");
  assert_eq!(data["latestRelease"], Value::Null);

  mock.answer("v99.0.0");
  state.status.refresh_release().await;
  mock.fail();
  state.status.refresh_release().await;
  let data = status(&router, &root).await;
  assert_eq!(
    version_check(&data).unwrap()["title"],
    "Dopbase 99.0.0 is out"
  );
  assert_eq!(data["latestRelease"]["version"], "v99.0.0");
  state.db.close().await;
}

async fn app_with_public_url(
  public_url: &str,
  source: app::config::PublicUrlSource,
) -> (TempDir, app::state::AppState, Router) {
  let directory = TempDir::new().unwrap();
  let config = ServerConfig {
    data_dir: directory.path().to_path_buf(),
    database_url: format!("sqlite://{}", directory.path().join("dopbase.db").display()),
    master_key: app::config::MasterKeyConfig {
      provider: "file".into(),
      path: directory.path().join("master.key"),
    },
    public_url: public_url.into(),
    public_url_source: source,
    ..ServerConfig::default()
  };
  let state = server::build_setup_state(config).await.unwrap();
  let router = server::router(state.clone());
  (directory, state, router)
}

#[tokio::test]
async fn http_and_a_missing_public_url_only_matter_off_this_machine() {
  use app::config::PublicUrlSource::{Explicit, LoopbackDefault, NetworkPlaceholder};
  for (url, source, public_url, https) in [
    // Only this machine connects: nothing to fix.
    ("http://localhost:8840", LoopbackDefault, "ok", "ok"),
    ("http://127.0.0.1:9000", Explicit, "ok", "ok"),
    // Reachable over the network without TLS: critical.
    ("http://secrets.example.com", Explicit, "ok", "crit"),
    (
      "http://SERVER_HOST:8840",
      NetworkPlaceholder,
      "warn",
      "crit",
    ),
    ("https://secrets.example.com", Explicit, "ok", "ok"),
  ] {
    let (_dir, state, router) = app_with_public_url(url, source).await;
    let (root, _) = instance(&state, &router).await;
    let data = status(&router, &root).await;
    assert_eq!(
      check(&data, "config.public_url")["status"],
      public_url,
      "{url}"
    );
    assert_eq!(check(&data, "config.https")["status"], https, "{url}");
    if https == "crit" {
      assert!(check(&data, "config.https")["fixPath"].is_string());
    }
    state.db.close().await;
  }
}
