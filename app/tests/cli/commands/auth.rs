use super::support::{command, failure, output, success};
use app::cli::{
  local_config::{ClientConfig, ResolvedServer, ServerSource},
  session,
};
use tempfile::TempDir;

#[tokio::test]
async fn login_token_saves_encrypted_credentials_and_logout_removes_them() {
  let directory = TempDir::new().unwrap();
  let token = "dbs_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
  let mut cmd = command(directory.path());
  cmd.args(["--json", "login", "--token"]);
  let result = output(cmd, Some(format!("{token}\n").as_bytes())).await;
  assert_eq!(success(&result)["authentication"], "encrypted_session");
  assert!(!String::from_utf8_lossy(&result.stdout).contains(token));
  assert!(!String::from_utf8_lossy(&result.stderr).contains(token));
  let resolved = ResolvedServer {
    url: "http://localhost:8840".into(),
    source: ServerSource::Default,
    config_path: directory.path().join("config.toml"),
    config: ClientConfig::default(),
  };
  let saved = session::load(&resolved).unwrap().unwrap();
  assert_eq!(saved.token, token);
  assert!(saved.email.is_none());
  let encrypted = std::fs::read(directory.path().join("session")).unwrap();
  assert!(
    !encrypted
      .windows(token.len())
      .any(|bytes| bytes == token.as_bytes())
  );
  for invalid in ["", "dbc_wrong-prefix", "dbs_too-short"] {
    let mut cmd = command(directory.path());
    cmd.args(["login", "--token"]);
    let result = output(cmd, Some(invalid.as_bytes())).await;
    failure(
      &result,
      if invalid.is_empty() {
        "cannot be empty"
      } else {
        "runner token"
      },
    );
    assert_eq!(
      std::fs::read(directory.path().join("session")).unwrap(),
      encrypted
    );
  }
  for _ in 0..2 {
    let mut cmd = command(directory.path());
    cmd.args(["--json", "logout"]);
    let logged_out = success(&output(cmd, None).await);
    assert_eq!(logged_out["authentication"], "none");
    // A runner token has no session to end; it is revoked in the Admin UI.
    assert_eq!(logged_out["server_session"], "not_applicable");
    assert!(!directory.path().join("session").exists());
    assert!(!directory.path().join("session-key").exists());
  }
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn human_login_saves_credentials_only_after_success() {
  use super::support::{EMAIL, Fixture, PASSWORD, terminal::Terminal};
  let fixture = Fixture::new().await;
  let session_path = fixture.client_dir.join("session");
  let before = std::fs::read(&session_path).unwrap();
  for password in ["wrong-password", PASSWORD] {
    let mut command = fixture.command();
    command.arg("login");
    let mut terminal = Terminal::new(command);
    terminal.reply("Email:", "  ADMIN@EXAMPLE.COM \r");
    terminal.reply("Password:", &format!("{password}\r"));
    let status = terminal.finish();
    assert!(!terminal.transcript.contains(password));
    if password == PASSWORD {
      assert!(status.success());
      let saved = session::load(&fixture.resolved()).unwrap().unwrap();
      assert_eq!(saved.email.as_deref(), Some(EMAIL));
      assert_ne!(saved.token, fixture.token);
    } else {
      assert_eq!(status.code(), Some(1));
      assert_eq!(std::fs::read(&session_path).unwrap(), before);
    }
  }
}

/// Start `dopbase login` and return the child once it has printed its user
/// code, along with that code and everything printed so far.
async fn start_browser_login(
  fixture: &super::support::Fixture,
  arguments: &[&str],
) -> (tokio::process::Child, String, String) {
  use tokio::io::AsyncReadExt;
  let mut command = fixture.command();
  command.args(arguments);
  let mut child = tokio::process::Command::from(command)
    .kill_on_drop(true)
    .stdin(std::process::Stdio::null())
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .spawn()
    .unwrap();
  let mut stderr = child.stderr.take().unwrap();
  let pattern = regex::Regex::new(r"code=([A-Z]{4}-[A-Z]{4})").unwrap();
  let mut printed = String::new();
  let code = tokio::time::timeout(std::time::Duration::from_secs(10), async {
    let mut buffer = [0_u8; 512];
    loop {
      let read = stderr.read(&mut buffer).await.unwrap();
      assert!(read > 0, "login exited early: {printed}");
      printed.push_str(&String::from_utf8_lossy(&buffer[..read]));
      if let Some(found) = pattern.captures(&printed) {
        return found[1].to_owned();
      }
    }
  })
  .await
  .expect("login never printed a code");
  // Keep draining so the child never blocks on a full pipe.
  tokio::spawn(async move {
    let mut sink = Vec::new();
    let _ = stderr.read_to_end(&mut sink).await;
  });
  (child, code, printed)
}

#[tokio::test(flavor = "multi_thread")]
async fn browser_login_saves_the_approved_session_without_printing_it() {
  use super::support::{EMAIL, Fixture, browser_credentials};
  let fixture = Fixture::new().await;
  session::remove(&fixture.resolved()).unwrap();
  let (child, code, printed) =
    start_browser_login(&fixture, &["--json", "login", "--browser"]).await;
  assert!(printed.contains(&fixture.url), "{printed}");
  assert!(
    printed.contains(&format!("/cli/approve?code={code}")),
    "{printed}"
  );

  let (cookie, csrf) = browser_credentials(&fixture).await;
  // The approval page names the program that asked.
  let details: serde_json::Value = reqwest::Client::new()
    .post(format!("{}/api/v1/auth/cli/inspect", fixture.url))
    .header("cookie", &cookie)
    .header("x-dopbase-csrf", &csrf)
    .json(&serde_json::json!({"userCode":code}))
    .send()
    .await
    .unwrap()
    .error_for_status()
    .unwrap()
    .json()
    .await
    .unwrap();
  let os = match std::env::consts::OS {
    "macos" => "macOS",
    "linux" => "Linux",
    "windows" => "Windows",
    other => other,
  };
  assert_eq!(
    details["data"]["client"],
    format!("dopbase {} on {os}", env!("CARGO_PKG_VERSION"))
  );
  reqwest::Client::new()
    .post(format!("{}/api/v1/auth/cli/approve", fixture.url))
    .header("cookie", &cookie)
    .header("x-dopbase-csrf", &csrf)
    .json(&serde_json::json!({"userCode":code}))
    .send()
    .await
    .unwrap()
    .error_for_status()
    .unwrap();

  let result = tokio::time::timeout(std::time::Duration::from_secs(20), child.wait_with_output())
    .await
    .expect("login did not finish")
    .unwrap();
  assert_eq!(success(&result)["authentication"], "encrypted_session");
  let saved = session::load(&fixture.resolved()).unwrap().unwrap();
  assert_eq!(saved.email.as_deref(), Some(EMAIL));
  assert!(saved.token.starts_with("dbc_"));
  assert!(!String::from_utf8_lossy(&result.stdout).contains(&saved.token));
  assert!(!printed.contains(&saved.token));
  let current: serde_json::Value = reqwest::Client::new()
    .get(format!("{}/api/v1/auth/session", fixture.url))
    .bearer_auth(&saved.token)
    .send()
    .await
    .unwrap()
    .error_for_status()
    .unwrap()
    .json()
    .await
    .unwrap();
  assert_eq!(current["data"]["sessionKind"], "cli");
}

#[tokio::test(flavor = "multi_thread")]
async fn browser_login_stops_when_the_request_is_denied() {
  use super::support::{Fixture, browser_credentials};
  let fixture = Fixture::new().await;
  let before = std::fs::read(fixture.client_dir.join("session")).unwrap();
  let (child, code, _) = start_browser_login(&fixture, &["login", "--browser"]).await;
  let (cookie, csrf) = browser_credentials(&fixture).await;
  reqwest::Client::new()
    .post(format!("{}/api/v1/auth/cli/deny", fixture.url))
    .header("cookie", &cookie)
    .header("x-dopbase-csrf", &csrf)
    .json(&serde_json::json!({"userCode":code}))
    .send()
    .await
    .unwrap()
    .error_for_status()
    .unwrap();
  let result = tokio::time::timeout(std::time::Duration::from_secs(20), child.wait_with_output())
    .await
    .expect("login did not finish")
    .unwrap();
  assert_eq!(result.status.code(), Some(1));
  assert_eq!(
    std::fs::read(fixture.client_dir.join("session")).unwrap(),
    before
  );
}

#[tokio::test]
async fn browser_login_flags_conflict_with_token_and_password() {
  let directory = TempDir::new().unwrap();
  for arguments in [
    ["login", "--browser", "--token"],
    ["login", "--browser-open", "--password"],
  ] {
    let mut cmd = command(directory.path());
    cmd.args(arguments);
    let result = output(cmd, None).await;
    assert_eq!(result.status.code(), Some(2), "{arguments:?}");
  }
}

#[tokio::test]
async fn login_and_logout_stop_while_dopbase_token_is_set() {
  let directory = TempDir::new().unwrap();
  let saved = "dbs_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
  let environment = "dbs_BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB";
  let mut cmd = command(directory.path());
  cmd.args(["login", "--token"]);
  success_exit(&output(cmd, Some(saved.as_bytes())).await);
  let session_path = directory.path().join("session");
  let key_path = directory.path().join("session-key");
  let session_before = std::fs::read(&session_path).unwrap();
  let key_before = std::fs::read(&key_path).unwrap();

  // Logout refuses, names the way out, and keeps the saved login.
  let mut cmd = command(directory.path());
  cmd.env("DOPBASE_TOKEN", environment).arg("logout");
  let result = output(cmd, None).await;
  assert_eq!(result.status.code(), Some(1));
  failure(
    &result,
    "DOPBASE_TOKEN is set, so Dopbase signs in with that token",
  );
  // The suggested command keeps the --data-dir this logout was given.
  let hint = format!(
    "env -u DOPBASE_TOKEN dopbase --data-dir {} logout",
    directory.path().display()
  );
  failure(&result, &hint);
  assert!(String::from_utf8_lossy(&result.stdout).is_empty());
  assert_eq!(std::fs::read(&session_path).unwrap(), session_before);
  assert_eq!(std::fs::read(&key_path).unwrap(), key_before);

  // JSON mode carries the same message.
  let mut cmd = command(directory.path());
  cmd
    .env("DOPBASE_TOKEN", environment)
    .args(["--json", "logout"]);
  let result = output(cmd, None).await;
  let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
  assert_eq!(error["success"], false);
  assert!(
    error["error"]["CLI_ERROR"]
      .as_str()
      .unwrap()
      .contains("Unset DOPBASE_TOKEN to stop using it")
  );

  // Login refuses before reading the piped token, in every mode.
  for arguments in [
    &["login", "--token"][..],
    &["login"],
    &["login", "--password"],
    &["login", "--browser"],
  ] {
    let mut cmd = command(directory.path());
    cmd.env("DOPBASE_TOKEN", environment).args(arguments);
    let result = output(
      cmd,
      Some(b"dbs_CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC"),
    )
    .await;
    assert_eq!(result.status.code(), Some(1), "{arguments:?}");
    failure(
      &result,
      "DOPBASE_TOKEN is set, so Dopbase would ignore a saved login",
    );
    assert!(
      !String::from_utf8_lossy(&result.stderr).contains("requires a terminal"),
      "{arguments:?}"
    );
    assert_eq!(std::fs::read(&session_path).unwrap(), session_before);
  }

  // An empty token keeps the error every other command gives.
  for arguments in [["logout"], ["login"]] {
    let mut cmd = command(directory.path());
    cmd.env("DOPBASE_TOKEN", "").args(arguments);
    failure(&output(cmd, None).await, "DOPBASE_TOKEN is set but empty");
  }
  assert_eq!(std::fs::read(&session_path).unwrap(), session_before);

  // --server is kept too, and quoted for the shell where needed.
  let mut cmd = command(directory.path());
  cmd
    .env("DOPBASE_TOKEN", environment)
    .args(["--server", "http://localhost:8840", "logout"]);
  failure(
    &output(cmd, None).await,
    &format!(
      "env -u DOPBASE_TOKEN dopbase --data-dir {} --server http://localhost:8840 logout",
      directory.path().display()
    ),
  );
  let spaced = TempDir::new().unwrap();
  let spaced_dir = spaced.path().join("my data");
  let mut cmd = command(&spaced_dir);
  cmd.args(["login", "--token"]);
  success_exit(&output(cmd, Some(saved.as_bytes())).await);
  let mut cmd = command(&spaced_dir);
  cmd.env("DOPBASE_TOKEN", environment).arg("logout");
  failure(
    &output(cmd, None).await,
    &format!("--data-dir '{}' logout", spaced_dir.display()),
  );

  // Running the suggested command, token still set in the shell, removes
  // the saved runner token and says how to revoke it.
  let mut shell = std::process::Command::new("sh");
  for name in app::constants::config::executable_environment_names() {
    shell.env_remove(name);
  }
  let binary = std::path::Path::new(env!("CARGO_BIN_EXE_dopbase"));
  shell
    .env(
      "PATH",
      format!(
        "{}:{}",
        binary.parent().unwrap().display(),
        std::env::var("PATH").unwrap_or_default()
      ),
    )
    .env("NO_COLOR", "1")
    .env("DOPBASE_TOKEN", environment)
    .args(["-c", &hint]);
  let result = output(shell, None).await;
  success_exit(&result);
  assert!(String::from_utf8_lossy(&result.stdout).contains("Logged out from"));
  assert!(
    String::from_utf8_lossy(&result.stderr).contains("Revoke it with dopbase token revoke"),
    "{}",
    String::from_utf8_lossy(&result.stderr)
  );
  assert!(!session_path.exists() && !key_path.exists());

  // With nothing saved there is no hint about removing a saved login.
  let mut cmd = command(directory.path());
  cmd.env("DOPBASE_TOKEN", environment).arg("logout");
  let result = output(cmd, None).await;
  failure(&result, "Unset DOPBASE_TOKEN to stop using it");
  assert!(!String::from_utf8_lossy(&result.stderr).contains("env -u"));
}

fn success_exit(result: &std::process::Output) {
  assert!(
    result.status.success(),
    "CLI failed: {}",
    String::from_utf8_lossy(&result.stderr)
  );
}

async fn session_status(
  url: &str,
  token: &str,
) -> reqwest::StatusCode {
  reqwest::Client::new()
    .get(format!("{url}/api/v1/auth/session"))
    .bearer_auth(token)
    .send()
    .await
    .unwrap()
    .status()
}

#[tokio::test(flavor = "multi_thread")]
async fn logout_ends_the_cli_session_on_the_server() {
  use super::support::Fixture;
  let fixture = Fixture::new().await;
  assert_eq!(
    session_status(&fixture.url, &fixture.token).await,
    reqwest::StatusCode::OK
  );
  let result = fixture.run(&["logout"]).await;
  success_exit(&result);
  assert!(
    String::from_utf8_lossy(&result.stdout).contains("and ended the session on the server"),
    "{}",
    String::from_utf8_lossy(&result.stdout)
  );
  assert_eq!(
    session_status(&fixture.url, &fixture.token).await,
    reqwest::StatusCode::UNAUTHORIZED
  );
  assert!(!fixture.client_dir.join("session").exists());
  assert!(!fixture.client_dir.join("session-key").exists());
  let audited: i64 =
    sqlx::query_scalar("SELECT COUNT(*) FROM audit_events WHERE action = 'logout.succeeded'")
      .fetch_one(fixture.state.db.pool())
      .await
      .unwrap();
  assert_eq!(audited, 1);

  // Nothing saved any more, so there is nothing to end.
  assert_eq!(
    fixture.json(&["logout"]).await["server_session"],
    "not_applicable"
  );
}

#[tokio::test(flavor = "multi_thread")]
async fn logout_reports_a_session_the_server_already_ended() {
  use super::support::Fixture;
  let fixture = Fixture::new().await;
  reqwest::Client::new()
    .post(format!("{}/api/v1/auth/logout", fixture.url))
    .bearer_auth(&fixture.token)
    .send()
    .await
    .unwrap()
    .error_for_status()
    .unwrap();
  let data = fixture.json(&["logout"]).await;
  assert_eq!(data["server_session"], "already_ended");
  assert_eq!(data["authentication"], "none");
  assert!(!fixture.client_dir.join("session").exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn logout_deletes_the_files_when_the_server_cannot_confirm() {
  // A port nothing listens on.
  let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
  let url = format!("http://{}", listener.local_addr().unwrap());
  drop(listener);
  let directory = TempDir::new().unwrap();
  let resolved = ResolvedServer {
    url: url.clone(),
    source: ServerSource::Argument,
    config_path: directory.path().join("config.toml"),
    config: ClientConfig::default(),
  };
  let cli_session = "dbc_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

  session::save(&resolved, cli_session, Some("admin@example.com")).unwrap();
  let mut cmd = command(directory.path());
  cmd.args(["--server", &url, "logout"]);
  let result = output(cmd, None).await;
  success_exit(&result);
  let stderr = String::from_utf8_lossy(&result.stderr);
  assert!(
    stderr.contains("Couldn't end the session on the server")
      && stderr.contains("It stays valid there until it expires"),
    "{stderr}"
  );
  assert!(String::from_utf8_lossy(&result.stdout).contains("Logged out from"));
  assert!(!session::is_saved(&resolved).unwrap());

  session::save(&resolved, cli_session, None).unwrap();
  let mut cmd = command(directory.path());
  cmd.args(["--json", "--server", &url, "logout"]);
  assert_eq!(
    success(&output(cmd, None).await)["server_session"],
    "not_reached"
  );

  // A damaged session can't be ended, but logout still clears it.
  std::fs::write(directory.path().join("session"), b"damaged").unwrap();
  let mut cmd = command(directory.path());
  cmd.args(["--server", &url, "logout"]);
  let result = output(cmd, None).await;
  success_exit(&result);
  assert!(String::from_utf8_lossy(&result.stderr).contains("the saved login could not be read"));
  assert!(!session::is_saved(&resolved).unwrap());

  // JSON tells a damaged login apart from a server that didn't answer.
  std::fs::write(directory.path().join("session"), b"damaged").unwrap();
  let mut cmd = command(directory.path());
  cmd.args(["--json", "--server", &url, "logout"]);
  assert_eq!(
    success(&output(cmd, None).await)["server_session"],
    "unreadable"
  );
  assert!(!session::is_saved(&resolved).unwrap());
}

#[tokio::test(flavor = "multi_thread")]
async fn logout_ends_a_saved_login_on_the_server_that_issued_it() {
  use super::support::Fixture;
  let fixture = Fixture::new().await;
  // Another server, which must never receive the saved token.
  let other = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
  other.set_nonblocking(true).unwrap();
  let other_url = format!("http://{}", other.local_addr().unwrap());

  let mut cmd = command(&fixture.client_dir);
  cmd.args(["--json", "--server", &other_url, "logout"]);
  let data = success(&output(cmd, None).await);
  assert_eq!(data["server_session"], "revoked");
  assert_eq!(data["server_url"], other_url);
  assert_eq!(data["session_server_url"], fixture.url);
  assert_eq!(
    session_status(&fixture.url, &fixture.token).await,
    reqwest::StatusCode::UNAUTHORIZED
  );
  assert!(!fixture.client_dir.join("session").exists());
  assert_eq!(
    other.accept().unwrap_err().kind(),
    std::io::ErrorKind::WouldBlock,
    "the other server received a request"
  );
}
