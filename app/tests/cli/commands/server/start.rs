#![cfg(unix)]

use super::{command, failure, setup::Running, success};
use app::{config::sqlite_url, services::db::DbClient};
use serde_json::{Value, json};
use std::{
  fs,
  path::{Path, PathBuf},
  process::Stdio,
  time::{Duration, Instant},
};
use tempfile::TempDir;

fn port() -> u16 {
  std::net::TcpListener::bind("127.0.0.1:0")
    .unwrap()
    .local_addr()
    .unwrap()
    .port()
}
struct Background(PathBuf);
impl Drop for Background {
  fn drop(&mut self) {
    let _ = command(&self.0)
      .args(["server", "stop", "--timeout", "2"])
      .output();
  }
}
async fn healthy(
  base: &str,
  running: &mut Running,
) {
  let client = reqwest::Client::new();
  let deadline = Instant::now() + Duration::from_secs(10);
  loop {
    if let Ok(response) = client.get(format!("{base}/api/v1/health")).send().await
      && response.status().is_success()
    {
      return;
    }
    assert!(
      running.0.try_wait().unwrap().is_none(),
      "server exited before readiness"
    );
    assert!(Instant::now() < deadline, "server did not become ready");
    tokio::time::sleep(Duration::from_millis(20)).await;
  }
}
async fn db(data: &Path) -> DbClient {
  DbClient::connect(&sqlite_url(&data.join("dopbase.db")))
    .await
    .unwrap()
}

#[tokio::test]
async fn background_initialization_returns_credentials_only_to_the_launcher_and_preserves_them() {
  let directory = TempDir::new().unwrap();
  let data = directory.path().join("instance with spaces");
  let _cleanup = Background(data.clone());
  let port = port().to_string();
  let started = success(
    command(&data)
      .env("DOPBASE_ROOT_EMAIL", " ROOT@EXAMPLE.COM ")
      .args([
        "--json",
        "server",
        "start",
        "--background",
        "--no-web-ui",
        "--port",
        &port,
      ])
      .output()
      .unwrap(),
  );
  let setup = &started["setup"];
  assert_eq!(setup["initialized"], true);
  assert_eq!(setup["email"], "root@example.com");
  let password = setup["password"].as_str().unwrap();
  let storage = db(&data).await;
  let before: String = sqlx::query_scalar("SELECT password_hash FROM admins")
    .fetch_one(storage.pool())
    .await
    .unwrap();
  let sessions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions")
    .fetch_one(storage.pool())
    .await
    .unwrap();
  assert_eq!(sessions, 0);
  storage.close().await;
  let base = format!("http://127.0.0.1:{port}");
  let client = reqwest::Client::new();
  assert_eq!(
    client.get(&base).send().await.unwrap().status().as_u16(),
    404
  );
  assert!(
    client
      .post(format!("{base}/api/v1/auth/login"))
      .json(&json!({"email":"root@example.com","password":password,"sessionKind":"cli"}))
      .send()
      .await
      .unwrap()
      .status()
      .is_success()
  );
  for file in ["serve.log", "dopbase.pid", "server.toml", "config.toml"] {
    assert!(
      !fs::read_to_string(data.join(file))
        .unwrap()
        .contains(password),
      "password in {file}"
    );
  }
  assert!(
    !fs::read_to_string(data.join("dopbase.pid"))
      .unwrap()
      .contains("root@example.com")
  );
  assert!(
    !fs::read_to_string(data.join("serve.log"))
      .unwrap()
      .contains("Password (shown once)")
  );
  let restarted = success(
    command(&data)
      .args(["--json", "server", "restart"])
      .output()
      .unwrap(),
  );
  assert!(restarted.get("setup").is_none());
  assert!(
    command(&data)
      .args(["server", "stop"])
      .output()
      .unwrap()
      .status
      .success()
  );
  let started = success(
    command(&data)
      .env("DOPBASE_ROOT_EMAIL", "invalid")
      .args([
        "--json",
        "server",
        "start",
        "--background",
        "--no-web-ui",
        "--port",
        &port,
      ])
      .output()
      .unwrap(),
  );
  assert!(started.get("setup").is_none());
  let storage = db(&data).await;
  let after: String = sqlx::query_scalar("SELECT password_hash FROM admins")
    .fetch_one(storage.pool())
    .await
    .unwrap();
  assert_eq!(before, after);
  storage.close().await;
}

#[tokio::test]
async fn foreground_initialization_prints_password_once_and_continues_serving() {
  let directory = TempDir::new().unwrap();
  let data = directory.path().join("data");
  let output = directory.path().join("output");
  let port = port().to_string();
  let mut running = Running(
    command(&data)
      .env("DOPBASE_ROOT_EMAIL", "root@example.com")
      .args(["server", "start", "--port", &port])
      .stdout(Stdio::from(fs::File::create(&output).unwrap()))
      .stderr(Stdio::null())
      .spawn()
      .unwrap(),
  );
  let base = format!("http://127.0.0.1:{port}");
  healthy(&base, &mut running).await;
  let output = fs::read_to_string(output).unwrap();
  assert_eq!(output.matches("Password (shown once):").count(), 1);
  let password = output
    .lines()
    .find_map(|line| line.strip_prefix("Password (shown once): "))
    .unwrap();
  let client = reqwest::Client::new();
  assert!(
    client
      .post(format!("{base}/api/v1/auth/login"))
      .json(&json!({"email":"root@example.com","password":password,"sessionKind":"cli"}))
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
  assert_eq!(status["data"]["state"], "ready");
}

#[tokio::test]
async fn background_web_setup_relays_token_and_allows_claiming_without_restarting() {
  let directory = TempDir::new().unwrap();
  let _cleanup = Background(directory.path().to_path_buf());
  let port = port().to_string();
  let output = command(directory.path())
    .args(["--json", "server", "start", "--background", "--port", &port])
    .output()
    .unwrap();
  let notice = String::from_utf8(output.stderr.clone()).unwrap();
  let started = success(output);
  assert!(started.get("setup").is_none());
  let token = notice
    .lines()
    .find(|line| line.starts_with("setup_"))
    .unwrap();
  assert!(notice.contains(&format!("/setup?token={token}")));
  let base = format!("http://127.0.0.1:{port}");
  let client = reqwest::Client::new();
  let created = client
    .post(format!("{base}/api/v1/bootstrap/admin"))
    .json(&json!({"setupToken":token,"email":"root@example.com","password":"fixture-password-123"}))
    .send()
    .await
    .unwrap();
  assert_eq!(created.status().as_u16(), 201);
  assert!(created.headers().contains_key("set-cookie"));
  assert!(
    client
      .get(format!("{base}/api/v1/health"))
      .send()
      .await
      .unwrap()
      .status()
      .is_success()
  );
}

#[test]
fn automatic_start_validates_email_and_failures_before_creating_an_account() {
  for background in [false, true] {
    for email in ["invalid", ""] {
      let directory = TempDir::new().unwrap();
      let data = directory.path().join("missing");
      let mut cmd = command(&data);
      cmd
        .env("DOPBASE_ROOT_EMAIL", email)
        .args(["server", "start", "--no-web-ui"]);
      if background {
        cmd.arg("--background");
      }
      failure(
        cmd.output().unwrap(),
        if email.is_empty() {
          "dopbase server setup"
        } else {
          "DOPBASE_ROOT_EMAIL must contain a valid email address"
        },
      );
      assert!(!data.exists());
    }
  }
  use std::os::unix::ffi::OsStringExt;
  let directory = TempDir::new().unwrap();
  let data = directory.path().join("missing");
  failure(
    command(&data)
      .env(
        "DOPBASE_ROOT_EMAIL",
        std::ffi::OsString::from_vec(vec![0xff]),
      )
      .args(["server", "start"])
      .output()
      .unwrap(),
    "DOPBASE_ROOT_EMAIL must contain a valid UTF-8 email address",
  );
  assert!(!data.exists());
  let directory = TempDir::new().unwrap();
  let occupied = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
  let port = occupied.local_addr().unwrap().port().to_string();
  failure(
    command(directory.path())
      .env("DOPBASE_ROOT_EMAIL", "root@example.com")
      .args(["server", "start", "--port", &port])
      .output()
      .unwrap(),
    "failed to bind",
  );
  assert!(!directory.path().join("dopbase.db").exists());
  assert!(!directory.path().join("server.toml").exists());
}

#[tokio::test]
async fn failed_automatic_credential_delivery_preserves_initialization_and_stops_the_server() {
  for background in [false, true] {
    let directory = TempDir::new().unwrap();
    let _cleanup = Background(directory.path().to_path_buf());
    let port = port().to_string();
    let (reader, writer) = nix::unistd::pipe().unwrap();
    drop(reader);
    let mut cmd = command(directory.path());
    cmd
      .env("DOPBASE_ROOT_EMAIL", "root@example.com")
      .args(["server", "start", "--port", &port]);
    if background {
      cmd.args(["--background", "--json"]);
    }
    let output = cmd
      .stdout(Stdio::from(writer))
      .stderr(Stdio::piped())
      .output()
      .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("dopbase admin reset-password"), "{error}");
    if background {
      let error: Value = serde_json::from_str(&error).unwrap();
      assert_eq!(error["initialized"], true);
    }
    let storage = db(directory.path()).await;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admins")
      .fetch_one(storage.pool())
      .await
      .unwrap();
    assert_eq!(count, 1);
    storage.close().await;
    assert!(
      !app::server::InstanceLock::is_held(&sqlite_url(&directory.path().join("dopbase.db")))
        .unwrap()
    );
    assert!(!directory.path().join("dopbase.pid").exists());
  }
}
