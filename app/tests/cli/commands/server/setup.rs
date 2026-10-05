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
fn uninitialized_start_shows_setup_guidance_without_provisioning() {
  let directory = TempDir::new().unwrap();
  for flags in [vec![], vec!["--background"], vec!["--supervised"]] {
    let data = directory.path().join("missing");
    let output = command(&data)
      .args(["server", "start"])
      .args(&flags)
      .output()
      .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let info = String::from_utf8(output.stderr).unwrap();
    assert!(info.starts_with("Info:"), "{info}");
    assert!(info.contains("dopbase server setup\n"), "{info}");
    assert!(
      info.contains("dopbase server setup --email admin@example.com"),
      "{info}"
    );
    assert!(info.contains("dopbase server setup --web"), "{info}");
    assert!(info.contains("https://docs.dopbase.com"), "{info}");
    assert!(
      info.contains("same --data-dir and --config options"),
      "{info}"
    );
    assert!(info.contains("host, port, public URL"), "{info}");
    assert!(info.contains("server start --help"), "{info}");
    assert!(!info.contains("Error:"), "{info}");
    assert!(!data.exists());
  }

  let data = directory.path().join("json-missing");
  let config = directory.path().join("custom-server.toml");
  let output = command(&data)
    .args(["--json", "server", "start", "--background", "--config"])
    .arg(&config)
    .output()
    .unwrap();
  assert_eq!(output.status.code(), Some(1));
  assert!(output.stderr.is_empty());
  let value: Value = serde_json::from_slice(&output.stdout).unwrap();
  assert_eq!(value["success"], false);
  assert_eq!(value["info"]["code"], "SETUP_REQUIRED");
  assert!(
    value["info"]["message"]
      .as_str()
      .unwrap()
      .contains("https://docs.dopbase.com")
  );
  assert_eq!(value["info"]["data_dir"], data.to_str().unwrap());
  assert_eq!(value["info"]["config_file"], config.to_str().unwrap());
  assert!(value.get("error").is_none());
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
      vec!["server", "setup", "--web", "--email", "invalid"],
      "valid email",
    ),
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
  for args in [vec!["server", "setup", "--background"], vec!["setup"]] {
    assert_eq!(
      command(&data).args(args).output().unwrap().status.code(),
      Some(2)
    );
    assert!(!data.exists());
  }
}

#[tokio::test]
async fn generated_setup_creates_one_root_and_audit_without_sessions_or_login() {
  for environment in [None, Some("  ROOT@EXAMPLE.COM  ")] {
    let directory = TempDir::new().unwrap();
    let data = directory.path().join("instance");
    let result = match environment {
      None => generated(&data),
      Some(email) => success(
        command(&data)
          .env("DOPBASE_ROOT_EMAIL", email)
          .args(["--json", "server", "setup"])
          .stdin(Stdio::null())
          .output()
          .unwrap(),
      ),
    };
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
}

#[test]
fn human_generated_output_displays_password_once() {
  for environment in [false, true] {
    let directory = TempDir::new().unwrap();
    let mut cmd = command(directory.path());
    cmd.args(["server", "setup"]);
    if environment {
      cmd.env("DOPBASE_ROOT_EMAIL", "root@example.com");
    } else {
      cmd.args(["--email", "root@example.com"]);
    }
    let output = cmd
      .stdin(Stdio::null())
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
}

#[test]
fn environment_email_validation_and_blank_fallback_create_no_files() {
  let directory = TempDir::new().unwrap();
  let data = directory.path().join("missing");
  for email in ["", "  ", "invalid", "a@", "a@b@example.com"] {
    for web in [false, true] {
      if web && email.trim().is_empty() {
        continue;
      }
      let mut cmd = command(&data);
      cmd
        .env("DOPBASE_ROOT_EMAIL", email)
        .args(["server", "setup"]);
      if web {
        cmd.arg("--web");
      }
      let expected = if email.trim().is_empty() {
        "requires a terminal"
      } else {
        "DOPBASE_ROOT_EMAIL must contain a valid email address"
      };
      failure(cmd.stdin(Stdio::null()).output().unwrap(), expected);
      assert!(!data.exists());
    }
  }
}

#[cfg(unix)]
#[test]
fn non_unicode_environment_email_is_rejected_unless_explicitly_overridden() {
  use std::os::unix::ffi::OsStringExt;
  let directory = TempDir::new().unwrap();
  let data = directory.path().join("missing");
  for web in [false, true] {
    let mut cmd = command(&data);
    cmd
      .env(
        "DOPBASE_ROOT_EMAIL",
        std::ffi::OsString::from_vec(vec![0xff]),
      )
      .args(["server", "setup"]);
    if web {
      cmd.arg("--web");
    }
    failure(
      cmd.output().unwrap(),
      "DOPBASE_ROOT_EMAIL must contain a valid UTF-8 email address",
    );
    assert!(!data.exists());
  }
  let result = success(
    command(&data)
      .env(
        "DOPBASE_ROOT_EMAIL",
        std::ffi::OsString::from_vec(vec![0xff]),
      )
      .args(["--json", "server", "setup", "--email", "root@example.com"])
      .stdin(Stdio::null())
      .output()
      .unwrap(),
  );
  assert_eq!(result["email"], "root@example.com");
}

#[test]
fn explicit_email_overrides_environment_without_fallback() {
  let directory = TempDir::new().unwrap();
  for environment in ["other@example.com", "invalid"] {
    let instance = TempDir::new_in(directory.path()).unwrap();
    let result = success(
      command(instance.path())
        .env("DOPBASE_ROOT_EMAIL", environment)
        .args(["--json", "server", "setup", "--email", " ROOT@EXAMPLE.COM "])
        .stdin(Stdio::null())
        .output()
        .unwrap(),
    );
    assert_eq!(result["email"], "root@example.com");
    failure(
      command(instance.path())
        .env("DOPBASE_ROOT_EMAIL", "other@example.com")
        .args(["server", "setup"])
        .output()
        .unwrap(),
      "already been initialized",
    );
  }
  let missing = directory.path().join("missing");
  failure(
    command(&missing)
      .env("DOPBASE_ROOT_EMAIL", "valid@example.com")
      .args(["server", "setup", "--email", "invalid"])
      .output()
      .unwrap(),
    "--email must contain a valid email address",
  );
  assert!(!missing.exists());
}

#[test]
fn environment_email_never_initializes_server_start() {
  let directory = TempDir::new().unwrap();
  let missing = directory.path().join("missing");
  for flags in [vec![], vec!["--background"], vec!["--supervised"]] {
    failure(
      command(&missing)
        .env("DOPBASE_ROOT_EMAIL", "root@example.com")
        .args(["server", "start"])
        .args(flags)
        .output()
        .unwrap(),
      "dopbase server setup",
    );
    assert!(!missing.exists());
  }
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
  for (environment, explicit, expected_email) in [
    (None, None, None),
    (
      Some(" ROOT+SETUP@EXAMPLE.COM "),
      None,
      Some("root+setup@example.com"),
    ),
    (
      Some("invalid"),
      Some("CLI+SETUP@EXAMPLE.COM"),
      Some("cli+setup@example.com"),
    ),
  ] {
    let directory = TempDir::new().unwrap();
    let occupied = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = occupied.local_addr().unwrap().port();
    drop(occupied);
    let saved_config = "web_ui = false\n";
    fs::write(directory.path().join("server.toml"), saved_config).unwrap();
    let log_path = directory.path().join("web-output.log");
    let mut cmd = command(directory.path());
    cmd.args(["server", "setup", "--web", "--port", &port.to_string()]);
    cmd.env("DOPBASE_WEB_UI", "false");
    if let Some(email) = environment {
      cmd.env("DOPBASE_ROOT_EMAIL", email);
    }
    if let Some(email) = explicit {
      cmd.args(["--email", email]);
    }
    let mut running = Running(
      cmd
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
    let link = text
      .lines()
      .find(|line| line.contains("/setup?token="))
      .unwrap();
    let url = url::Url::parse(link).unwrap();
    let pairs: Vec<_> = url.query_pairs().collect();
    assert_eq!(pairs.len(), if expected_email.is_some() { 2 } else { 1 });
    assert_eq!(pairs[0].0, "token");
    assert_eq!(pairs[0].1, token);
    if let Some(expected) = expected_email {
      assert_eq!(pairs[1].0, "email");
      assert_eq!(pairs[1].1, expected);
      assert!(link.contains("%2B"));
    }
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
    assert!(
      client
        .get(&base)
        .send()
        .await
        .unwrap()
        .status()
        .is_success()
    );
    assert_eq!(
      fs::read_to_string(directory.path().join("server.toml")).unwrap(),
      saved_config
    );
    #[cfg(unix)]
    {
      nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(running.0.id() as i32),
        nix::sys::signal::Signal::SIGTERM,
      )
      .unwrap();
      assert!(running.0.wait().unwrap().success());
      let mut restarted = Running(
        command(directory.path())
          .args(["server", "start", "--port", &port.to_string()])
          .stdout(Stdio::null())
          .stderr(Stdio::from(fs::File::create(&log_path).unwrap()))
          .spawn()
          .unwrap(),
      );
      let client = reqwest::Client::new();
      let deadline = Instant::now() + Duration::from_secs(10);
      loop {
        if let Ok(response) = client.get(format!("{base}/api/v1/health")).send().await
          && response.status().is_success()
        {
          break;
        }
        assert!(restarted.0.try_wait().unwrap().is_none());
        assert!(
          Instant::now() < deadline,
          "normal startup readiness timed out"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
      }
      assert_eq!(
        client.get(&base).send().await.unwrap().status().as_u16(),
        404
      );
      assert!(
        fs::read_to_string(&log_path)
          .unwrap()
          .contains("Admin UI:   disabled")
      );
      nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(restarted.0.id() as i32),
        nix::sys::signal::Signal::SIGTERM,
      )
      .unwrap();
      assert!(restarted.0.wait().unwrap().success());
    }
  }
}

#[cfg(unix)]
mod guided {
  use super::*;
  use nix::{
    fcntl::{FcntlArg, OFlag, fcntl},
    pty::{Winsize, openpty},
  };
  use std::{
    io::{Read, Write},
    os::unix::process::CommandExt,
  };

  struct Terminal {
    process: Running,
    master: fs::File,
    transcript: String,
  }
  impl Terminal {
    fn new(data: &Path) -> Self {
      Self::with_email(data, None)
    }
    fn with_email(
      data: &Path,
      email: Option<&str>,
    ) -> Self {
      let pty = openpty(
        Some(&Winsize {
          ws_row: 40,
          ws_col: 120,
          ws_xpixel: 0,
          ws_ypixel: 0,
        }),
        None,
      )
      .unwrap();
      let slave = fs::File::from(pty.slave);
      let master = fs::File::from(pty.master);
      fcntl(&master, FcntlArg::F_SETFL(OFlag::O_NONBLOCK)).unwrap();
      let mut cmd = command(data);
      if let Some(email) = email {
        cmd.env("DOPBASE_ROOT_EMAIL", email);
      }
      cmd
        .env("TERM", "xterm")
        .args(["server", "setup"])
        .stdin(Stdio::from(slave.try_clone().unwrap()))
        .stdout(Stdio::from(slave.try_clone().unwrap()))
        .stderr(Stdio::from(slave));
      unsafe {
        cmd.pre_exec(|| {
          nix::unistd::setsid().map_err(std::io::Error::from)?;
          if nix::libc::ioctl(0, nix::libc::TIOCSCTTY as _, 0) == -1 {
            return Err(std::io::Error::last_os_error());
          }
          Ok(())
        });
      }
      Self {
        process: Running(cmd.spawn().unwrap()),
        master,
        transcript: String::new(),
      }
    }
    fn read(&mut self) {
      let mut buf = [0; 8192];
      if let Ok(n) = self.master.read(&mut buf) {
        let chunk = String::from_utf8_lossy(&buf[..n]);
        if chunk.contains("\x1b[6n") {
          self.master.write_all(b"\x1b[1;1R").unwrap();
        }
        self.transcript.push_str(&chunk);
      }
    }
    fn wait_for(
      &mut self,
      expected: &str,
      from: usize,
    ) {
      let deadline = Instant::now() + Duration::from_secs(10);
      loop {
        self.read();
        if self.transcript[from..].contains(expected) {
          return;
        }
        assert!(
          self.process.0.try_wait().unwrap().is_none(),
          "guided setup exited before expected prompt"
        );
        assert!(
          Instant::now() < deadline,
          "guided setup prompt timed out: {expected}"
        );
        std::thread::sleep(Duration::from_millis(10));
      }
    }
    fn send(
      &mut self,
      value: &str,
    ) -> usize {
      let from = self.transcript.len();
      self.master.write_all(value.as_bytes()).unwrap();
      from
    }
    fn finish(&mut self) -> std::process::ExitStatus {
      let deadline = Instant::now() + Duration::from_secs(10);
      loop {
        self.read();
        if let Some(status) = self.process.0.try_wait().unwrap() {
          self.read();
          return status;
        }
        assert!(Instant::now() < deadline, "guided setup exit timed out");
        std::thread::sleep(Duration::from_millis(10));
      }
    }
  }

  #[tokio::test]
  async fn guided_setup_validates_email_password_boundaries_and_confirmation() {
    let directory = TempDir::new().unwrap();
    let mut terminal = Terminal::new(directory.path());
    terminal.wait_for("Root email:", 0);
    let from = terminal.send("invalid\r");
    terminal.wait_for("valid email", from);
    let from = terminal.send(&format!("{}ROOT@EXAMPLE.COM\r", "\x7f".repeat(7)));
    terminal.wait_for("Root password:", from);
    let from = terminal.send("short\r");
    terminal.wait_for("at least 12", from);
    let from = terminal.send(&format!("{}{}\r", "\x7f".repeat(5), "x".repeat(129)));
    terminal.wait_for("at most 128", from);
    let password = "fixture-pw12";
    let from = terminal.send(&format!("{}{password}\r", "\x7f".repeat(129)));
    terminal.wait_for("Confirm root password:", from);
    let from = terminal.send("different-password\r");
    terminal.wait_for("Passwords do not match", from);
    let from = terminal.send(&format!("{password}\r"));
    terminal.wait_for("Confirm root password:", from);
    terminal.send(&format!("{password}\r"));
    assert!(terminal.finish().success());
    assert!(!terminal.transcript.contains(password));
    assert!(!terminal.transcript.contains("different-password"));
    let db = DbClient::connect(&sqlite_url(&directory.path().join("dopbase.db")))
      .await
      .unwrap();
    let (email, hash): (String, String) = sqlx::query_as("SELECT email,password_hash FROM admins")
      .fetch_one(db.pool())
      .await
      .unwrap();
    assert_eq!(email, "root@example.com");
    assert!(
      argon2::PasswordVerifier::verify_password(
        &argon2::Argon2::default(),
        password.as_bytes(),
        &argon2::PasswordHash::new(&hash).unwrap()
      )
      .is_ok()
    );
    assert_eq!(count(&db, "sessions").await, 0);
    db.close().await;
  }

  #[test]
  fn cancelling_guided_password_leaves_no_account_or_storage() {
    let directory = TempDir::new().unwrap();
    let data = directory.path().join("missing");
    let mut terminal = Terminal::with_email(&data, Some("  "));
    terminal.wait_for("Root email:", 0);
    let from = terminal.send("root@example.com\r");
    terminal.wait_for("Root password:", from);
    terminal.send("\x03");
    assert_eq!(terminal.finish().code(), Some(130));
    assert!(terminal.transcript.contains("Setup cancelled."));
    assert!(!data.exists());
  }
}
