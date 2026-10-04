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
