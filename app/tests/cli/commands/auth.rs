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
    assert_eq!(success(&output(cmd, None).await)["authentication"], "none");
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
