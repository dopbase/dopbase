use super::{command, failure, success};
use app::{
  config::{EnvironmentOverrides, ServerConfig, ServerOverrides, sqlite_url},
  services::db::DbClient,
};
use serde_json::Value;
use std::{
  fs,
  path::Path,
  process::Stdio,
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
