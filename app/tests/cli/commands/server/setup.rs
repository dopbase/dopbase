use super::{command, failure, success};
use app::{
  config::{EnvironmentOverrides, ServerConfig, ServerOverrides, sqlite_url},
  services::db::DbClient,
};
use serde_json::{Value, json};
use std::{
  fs,
  path::Path,
  process::{Child, Stdio},
  time::{Duration, Instant},
};
use tempfile::TempDir;

fn generated(data: &Path) -> Value {
  success(
    command(data)
      .args(["--json", "server", "setup", "--email", "ROOT@EXAMPLE.COM"])
      .stdin(Stdio::null())
      .output()
      .unwrap(),
  )
}
fn config(data: &Path) -> ServerConfig {
  ServerConfig::load_with_environment(
    &ServerOverrides {
      data_dir: Some(data.to_path_buf()),
      ..Default::default()
    },
    EnvironmentOverrides::default(),
  )
  .unwrap()
}
async fn count(
  db: &DbClient,
  table: &str,
) -> i64 {
  sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
    .fetch_one(db.pool())
    .await
    .unwrap()
}

#[test]
fn uninitialized_start_rejects_every_launch_mode_without_provisioning() {
  let directory = TempDir::new().unwrap();
  for flags in [vec![], vec!["--background"], vec!["--supervised"]] {
    let data = directory.path().join("missing");
    failure(
      command(&data)
        .args(["server", "start"])
        .args(&flags)
        .output()
        .unwrap(),
      "dopbase server setup",
    );
    assert!(!data.exists());
  }
  let data = directory.path().join("json-missing");
  let output = command(&data)
    .args(["--json", "server", "start", "--background"])
    .output()
    .unwrap();
  assert_eq!(output.status.code(), Some(1));
  assert!(output.stdout.is_empty());
  let error: Value = serde_json::from_slice(&output.stderr).unwrap();
  assert!(error["error"]["SETUP_REQUIRED"].is_string());
  assert!(!data.exists());
}

#[test]
fn setup_rejects_invalid_modes_before_creating_state() {
  let directory = TempDir::new().unwrap();
  let data = directory.path().join("missing");
  for (args, message) in [
    (vec!["server", "setup"], "requires a terminal"),
    (vec!["--json", "server", "setup"], "Pass --email"),
    (
      vec!["--json", "server", "setup", "--web"],
      "--json cannot be used",
    ),
    (
      vec![
        "--server",
        "https://example.com",
        "server",
        "setup",
        "--email",
        "root@example.com",
      ],
      "--server cannot be used",
    ),
    (vec!["server", "setup", "--email", "invalid"], "valid email"),
    (
      vec![
        "server",
        "setup",
        "--email",
        "root@example.com",
        "--port",
        "9000",
      ],
      "listener options require",
    ),
  ] {
    let output = command(&data)
      .args(args)
      .stdin(Stdio::null())
      .output()
      .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains(message));
    assert!(!data.exists());
  }
  for args in [
    vec!["server", "setup", "--web", "--email", "root@example.com"],
    vec!["server", "setup", "--background"],
    vec!["setup"],
  ] {
    assert_eq!(
      command(&data).args(args).output().unwrap().status.code(),
      Some(2)
    );
    assert!(!data.exists());
  }
}

#[tokio::test]
async fn generated_setup_creates_one_root_and_audit_without_sessions_or_login() {
  let directory = TempDir::new().unwrap();
  let data = directory.path().join("instance");
  let result = generated(&data);
  assert_eq!(result["initialized"], true);
  assert_eq!(result["email"], "root@example.com");
  assert_eq!(result["data_dir"], data.to_str().unwrap());
  let password = result["password"].as_str().unwrap();
  assert_eq!(password.len(), 43);
  let db = DbClient::connect(&sqlite_url(&data.join("dopbase.db")))
    .await
    .unwrap();
  let (id, email, hash, role): (String, String, String, String) =
    sqlx::query_as("SELECT id,email,password_hash,role FROM admins")
      .fetch_one(db.pool())
      .await
      .unwrap();
  assert_eq!(result["admin_id"], id);
  assert_eq!(email, "root@example.com");
  assert_eq!(role, "root");
  assert!(
    argon2::PasswordVerifier::verify_password(
      &argon2::Argon2::default(),
      password.as_bytes(),
      &argon2::PasswordHash::new(&hash).unwrap()
    )
    .is_ok()
  );
  assert_eq!(count(&db, "admins").await, 1);
  assert_eq!(count(&db, "sessions").await, 0);
  let events: i64 =
    sqlx::query_scalar("SELECT COUNT(*) FROM audit_events WHERE action='admin.bootstrapped'")
      .fetch_one(db.pool())
      .await
      .unwrap();
  assert_eq!(events, 1);
  db.close().await;
  let runtime = app::server::build_state(config(&data)).await.unwrap();
  assert!(runtime.setup.read().await.token.is_none());
  runtime.db.close().await;
  assert!(!data.join("serve.log").exists());
  assert!(!data.join("dopbase.pid").exists());
  for name in ["server.toml", "config.toml"] {
    let contents = fs::read_to_string(data.join(name)).unwrap();
    assert!(!contents.contains(password));
    assert!(!contents.contains("root@example.com"));
  }
}

#[test]
fn human_generated_output_displays_password_once() {
  let directory = TempDir::new().unwrap();
  let output = command(directory.path())
    .args(["server", "setup", "--email", "root@example.com"])
    .env("RUST_LOG", "debug")
    .output()
    .unwrap();
  assert!(output.status.success());
  let text = String::from_utf8(output.stdout).unwrap();
  let password = text
    .lines()
    .find_map(|line| line.strip_prefix("Password (shown once): "))
    .unwrap();
  assert_eq!(password.len(), 43);
  assert_eq!(text.matches(password).count(), 1);
  assert!(!String::from_utf8_lossy(&output.stderr).contains(password));
  assert!(text.contains("dopbase server start"));
  assert!(text.contains("dopbase login"));
}

#[tokio::test]
async fn repeated_and_competing_setup_preserve_the_first_account() {
  let directory = TempDir::new().unwrap();
  let data = directory.path().join("data");
  let mut children = Vec::new();
  for _ in 0..2 {
    children.push(
      command(&data)
        .args(["--json", "server", "setup", "--email", "root@example.com"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap(),
    );
  }
  let results: Vec<_> = children
    .into_iter()
    .map(|child| child.wait_with_output().unwrap())
    .collect();
  assert_eq!(
    results
      .iter()
      .filter(|output| output.status.success())
      .count(),
    1
  );
  let original = fs::read(data.join("dopbase.db")).unwrap();
  let key = fs::read(data.join("master.key")).unwrap();
  failure(
    command(&data)
      .args(["server", "setup", "--email", "other@example.com"])
      .output()
      .unwrap(),
    "already been initialized",
  );
  failure(
    command(&data)
      .args(["server", "setup", "--web"])
      .output()
      .unwrap(),
    "already been initialized",
  );
  assert_eq!(fs::read(data.join("dopbase.db")).unwrap(), original);
  assert_eq!(fs::read(data.join("master.key")).unwrap(), key);
  let db = DbClient::connect(&sqlite_url(&data.join("dopbase.db")))
    .await
    .unwrap();
  assert_eq!(count(&db, "admins").await, 1);
  assert_eq!(count(&db, "audit_events").await, 1);
  db.close().await;
}

#[tokio::test]
async fn reference_write_failure_is_retryable_without_rotating_the_key() {
  let directory = TempDir::new().unwrap();
  let data = directory.path().join("data");
  let blocked = directory.path().join("blocked");
  fs::write(&blocked, "keep").unwrap();
  let cfg = blocked.join("server.toml");
  failure(
    command(&data)
      .args(["server", "setup", "--email", "root@example.com", "--config"])
      .arg(&cfg)
      .output()
      .unwrap(),
    "blocked",
  );
  let key = fs::read(data.join("master.key")).unwrap();
  let db = DbClient::connect(&sqlite_url(&data.join("dopbase.db")))
    .await
    .unwrap();
  assert_eq!(count(&db, "admins").await, 0);
  db.close().await;
  fs::remove_file(&blocked).unwrap();
  assert!(
    command(&data)
      .args(["server", "setup", "--email", "root@example.com", "--config"])
      .arg(&cfg)
      .output()
      .unwrap()
      .status
      .success()
  );
  assert_eq!(fs::read(data.join("master.key")).unwrap(), key);
  assert!(cfg.exists());
}

#[test]
fn setup_respects_the_existing_instance_lock() {
  let directory = TempDir::new().unwrap();
  let url = sqlite_url(&directory.path().join("dopbase.db"));
  let _lock = app::server::InstanceLock::acquire(&url).unwrap();
  failure(
    command(directory.path())
      .args(["server", "setup", "--email", "root@example.com"])
      .output()
      .unwrap(),
    "instance is in use",
  );
  assert!(!directory.path().join("dopbase.db").exists());
  assert!(!directory.path().join("master.key").exists());
}

#[test]
fn corrupt_storage_is_not_treated_as_a_fresh_instance() {
  let directory = TempDir::new().unwrap();
  let database = directory.path().join("dopbase.db");
  fs::write(&database, "corrupt database fixture").unwrap();
  for args in [
    vec!["server", "start"],
    vec!["server", "setup", "--email", "root@example.com"],
  ] {
    failure(
      command(directory.path()).args(args).output().unwrap(),
      "failed to inspect SQLite initialization",
    );
    assert_eq!(
      fs::read_to_string(&database).unwrap(),
      "corrupt database fixture"
    );
    assert!(!directory.path().join("master.key").exists());
  }
}

#[tokio::test]
async fn incorrect_key_during_setup_preserves_uninitialized_storage() {
  let directory = TempDir::new().unwrap();
  let state = app::server::build_setup_state(config(directory.path()))
    .await
    .unwrap();
  state.db.close().await;
  let key = fs::read(directory.path().join("master.key")).unwrap();
  let wrong_key = directory.path().join("wrong.key");
  fs::write(&wrong_key, [0xff; 32]).unwrap();
  failure(
    command(directory.path())
      .args([
        "server",
        "setup",
        "--email",
        "root@example.com",
        "--master-key-file",
      ])
      .arg(&wrong_key)
      .output()
      .unwrap(),
    "configured master key does not match",
  );
  assert_eq!(fs::read(directory.path().join("master.key")).unwrap(), key);
  let db = DbClient::connect(&sqlite_url(&directory.path().join("dopbase.db")))
    .await
    .unwrap();
  assert_eq!(count(&db, "admins").await, 0);
  db.close().await;
  generated(directory.path());
}

#[tokio::test]
async fn only_explicit_setup_resumes_pending_factory_reset() {
  let directory = TempDir::new().unwrap();
  generated(directory.path());
  let key = fs::read(directory.path().join("master.key")).unwrap();
  let marker = directory.path().join(".factory-reset.pending");
  fs::write(&marker, "reset in progress").unwrap();
  fs::create_dir(directory.path().join("backups")).unwrap();
  let backup = directory.path().join("backups/fixture.dop");
  fs::write(&backup, "old backup fixture").unwrap();
  failure(
    command(directory.path())
      .args(["server", "start"])
      .output()
      .unwrap(),
    "dopbase server setup",
  );
  assert!(marker.exists());
  assert!(backup.exists());
  let output = command(directory.path())
    .args(["server", "setup", "--email", "replacement@example.com"])
    .output()
    .unwrap();
  assert!(output.status.success());
  assert!(!marker.exists());
  assert!(!backup.exists());
  assert_eq!(fs::read(directory.path().join("master.key")).unwrap(), key);
  let db = DbClient::connect(&sqlite_url(&directory.path().join("dopbase.db")))
    .await
    .unwrap();
  let email: String = sqlx::query_scalar("SELECT email FROM admins")
    .fetch_one(db.pool())
    .await
    .unwrap();
  assert_eq!(email, "replacement@example.com");
  assert_eq!(count(&db, "admins").await, 1);
  db.close().await;
}

#[cfg(unix)]
#[tokio::test]
async fn credential_output_failure_reports_committed_initialization() {
  let directory = TempDir::new().unwrap();
  let (read, write) = nix::unistd::pipe().unwrap();
  drop(read);
  let output = command(directory.path())
    .args(["--json", "server", "setup", "--email", "root@example.com"])
    .stdout(Stdio::from(write))
    .stderr(Stdio::piped())
    .spawn()
    .unwrap()
    .wait_with_output()
    .unwrap();
  assert_eq!(output.status.code(), Some(1));
  let error: Value = serde_json::from_slice(&output.stderr).unwrap();
  assert_eq!(error["initialized"], true);
  assert!(
    error["error"]["CLI_ERROR"]
      .as_str()
      .unwrap()
      .contains("dopbase admin reset-password")
  );
  let db = DbClient::connect(&sqlite_url(&directory.path().join("dopbase.db")))
    .await
    .unwrap();
  assert_eq!(count(&db, "admins").await, 1);
  db.close().await;
  failure(
    command(directory.path())
      .args(["server", "setup", "--email", "root@example.com"])
      .output()
      .unwrap(),
    "already been initialized",
  );
}

struct Running(Child);
impl Drop for Running {
  fn drop(&mut self) {
    let _ = self.0.kill();
    let _ = self.0.wait();
  }
}

#[tokio::test]
async fn web_setup_keeps_its_token_session_redirect_contract_and_continues_serving() {
  let directory = TempDir::new().unwrap();
  let occupied = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
  let port = occupied.local_addr().unwrap().port();
  drop(occupied);
  let log_path = directory.path().join("web-output.log");
  let mut running = Running(
    command(directory.path())
      .args(["server", "setup", "--web", "--port", &port.to_string()])
      .stdout(Stdio::null())
      .stderr(Stdio::from(fs::File::create(&log_path).unwrap()))
      .spawn()
      .unwrap(),
  );
  let client = reqwest::Client::new();
  let base = format!("http://127.0.0.1:{port}");
  let deadline = Instant::now() + Duration::from_secs(10);
  loop {
    if let Ok(response) = client.get(format!("{base}/api/v1/health")).send().await
      && response.status().is_success()
    {
      break;
    }
    assert!(
      running.0.try_wait().unwrap().is_none(),
      "web setup exited before readiness"
    );
    assert!(Instant::now() < deadline, "web setup readiness timed out");
    tokio::time::sleep(Duration::from_millis(20)).await;
  }
  let text = fs::read_to_string(&log_path).unwrap();
  let token = text
    .lines()
    .find(|line| line.starts_with("setup_"))
    .unwrap();
  assert!(text.contains(&format!("/setup?token={token}")));
  assert!(
    client
      .get(format!("{base}/setup"))
      .send()
      .await
      .unwrap()
      .status()
      .is_success()
  );
  let status: Value = client
    .get(format!("{base}/api/v1/bootstrap/status"))
    .send()
    .await
    .unwrap()
    .json()
    .await
    .unwrap();
  assert_eq!(status["data"]["state"], "setupRequired");
  let invalid = client
    .post(format!("{base}/api/v1/bootstrap/admin"))
    .json(
      &json!({"setupToken":"wrong", "email":"root@example.com", "password":"fixture-password-123"}),
    )
    .send()
    .await
    .unwrap();
  assert_eq!(invalid.status().as_u16(), 401);
  let created = client
    .post(format!("{base}/api/v1/bootstrap/admin"))
    .json(
      &json!({"setupToken":token, "email":"root@example.com", "password":"fixture-password-123"}),
    )
    .send()
    .await
    .unwrap();
  assert_eq!(created.status().as_u16(), 201);
  assert!(created.headers().contains_key("set-cookie"));
  let body: Value = created.json().await.unwrap();
  assert_eq!(body["data"]["role"], "root");
  assert!(body["data"]["csrfToken"].is_string());
  let login = client
    .post(format!("{base}/api/v1/auth/login"))
    .json(
      &json!({"email":"root@example.com","password":"fixture-password-123","sessionKind":"cli"}),
    )
    .send()
    .await
    .unwrap();
  assert!(login.status().is_success());
  let status: Value = client
    .get(format!("{base}/api/v1/bootstrap/status"))
    .send()
    .await
    .unwrap()
    .json()
    .await
    .unwrap();
  assert_eq!(status["data"]["state"], "ready");
  assert!(running.0.try_wait().unwrap().is_none());
  #[cfg(unix)]
  {
    nix::sys::signal::kill(
      nix::unistd::Pid::from_raw(running.0.id() as i32),
      nix::sys::signal::Signal::SIGTERM,
    )
    .unwrap();
    assert!(running.0.wait().unwrap().success());
  }
}
