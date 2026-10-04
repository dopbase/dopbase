use app::cli::{
  client::{Credential, CredentialSource},
  commands::{insecure_transport_warning, server_switch_confirmed, status_document},
  local_config::{ClientConfig, DefaultEnvironment, ResolvedServer, ServerSource},
};
use tempfile::TempDir;
fn server(directory: &TempDir) -> ResolvedServer {
  ResolvedServer {
    url: "https://dopbase.example.com".into(),
    source: ServerSource::Config,
    config_path: directory.path().join("config.toml"),
    config: ClientConfig {
      version: 1,
      server_url: Some("https://dopbase.example.com".into()),
      default_environment: Some(DefaultEnvironment {
        server_url: "https://dopbase.example.com".into(),
        environment_id: "env_default".into(),
      }),
    },
  }
}

#[test]
fn insecure_transport_warning_names_the_server_and_the_risk() {
  assert_eq!(
    insecure_transport_warning("http://192.168.1.20:8840"),
    "Plain HTTP does not encrypt traffic to http://192.168.1.20:8840. Credentials and secrets could be exposed. Use HTTPS whenever possible."
  );
}

#[test]
fn server_switch_requires_an_explicit_yes_answer() {
  for accepted in ["y", "Y", "yes", "YES", " yes "] {
    assert!(server_switch_confirmed(accepted), "{accepted:?}");
  }
  for rejected in ["", "n", "no", "true", "1", "switch"] {
    assert!(!server_switch_confirmed(rejected), "{rejected:?}");
  }
}

#[test]
fn status_includes_cached_admin_email_and_default_environment() {
  let directory = TempDir::new().unwrap();
  let server = server(&directory);
  let credential = Credential {
    token: Some("dbc_secret-token".into()),
    source: CredentialSource::EncryptedSession,
    email: Some("admin@example.com".into()),
  };

  let value = status_document(&server, &credential, true);
  assert_eq!(value["authentication"], "encrypted_session");
  assert_eq!(value["identity"], "admin");
  assert_eq!(value["email"], "admin@example.com");
  assert_eq!(value["environment"], "env_default");
  assert_eq!(value["server_status"], "connected");
  assert_eq!(value["status_source"], "live");
  assert!(!value.to_string().contains("dbc_secret-token"));
}

#[test]
fn status_identifies_an_encrypted_runner_token() {
  let directory = TempDir::new().unwrap();
  let server = server(&directory);
  let token = "dbs_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
  let credential = Credential {
    token: Some(token.into()),
    source: CredentialSource::EncryptedSession,
    email: None,
  };

  let value = status_document(&server, &credential, true);
  assert_eq!(value["authentication"], "encrypted_session");
  assert_eq!(value["identity"], "runner");
  assert!(value["email"].is_null());
  assert!(!value.to_string().contains(token));
}

#[test]
fn status_identifies_environment_token_types_without_exposing_tokens() {
  let directory = TempDir::new().unwrap();
  let server = server(&directory);
  for (token, identity) in [
    ("dbc_admin-secret", "human"),
    ("dbs_runner-secret", "runner"),
    ("dpa_agent-secret", "ai_agent"),
    ("other-secret", "unknown"),
  ] {
    let credential = Credential {
      token: Some(token.into()),
      source: CredentialSource::Environment,
      email: None,
    };

    let value = status_document(&server, &credential, true);
    assert_eq!(value["identity"], identity);
    assert!(value["email"].is_null());
    assert!(!value.to_string().contains(token));
  }
}

#[test]
fn status_reports_no_identity_without_credentials() {
  let directory = TempDir::new().unwrap();
  let mut server = server(&directory);
  server.config.default_environment = None;
  let credential = Credential {
    token: None,
    source: CredentialSource::None,
    email: None,
  };

  let value = status_document(&server, &credential, false);
  assert_eq!(value["identity"], "none");
  assert!(value["email"].is_null());
  assert!(value["environment"].is_null());
  assert_eq!(value["server_status"], "offline");
  assert_eq!(value["status_source"], "cache");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn client_connection_switch_preserves_state_until_confirmed() {
  use super::support::{Fixture, command, failure, output, success, terminal::Terminal};
  let fixture = Fixture::new().await;
  let destination = Fixture::new().await;
  let id = fixture.environment().await;
  fixture.json(&["env", "default", &id]).await;
  let mut config = app::cli::local_config::read(&fixture.client_dir.join("config.toml")).unwrap();
  config.server_url = Some(fixture.url.clone());
  app::cli::local_config::write(&fixture.client_dir.join("config.toml"), &config).unwrap();
  let mut same = command(&fixture.client_dir);
  same.args(["--json", "client", "connect", &fixture.url]);
  assert_eq!(success(&output(same, None).await)["changed"], false);
  assert_eq!(
    fixture.json(&["client", "status"]).await,
    fixture.json(&["status"]).await
  );
  let config_path = fixture.client_dir.join("config.toml");
  let before = std::fs::read(&config_path).unwrap();
  let session_before = std::fs::read(fixture.client_dir.join("session")).unwrap();
  let mut overridden = command(&fixture.client_dir);
  overridden
    .env("DOPBASE_URL", &fixture.url)
    .args(["client", "connect", &destination.url]);
  failure(&output(overridden, None).await, "DOPBASE_URL is set");
  let mut refused = command(&fixture.client_dir);
  refused.args(["client", "connect", &destination.url]);
  let mut terminal = Terminal::new(refused);
  terminal.reply("Continue?", "n\r");
  assert_eq!(terminal.finish().code(), Some(130));
  assert_eq!(std::fs::read(&config_path).unwrap(), before);
  assert_eq!(
    std::fs::read(fixture.client_dir.join("session")).unwrap(),
    session_before
  );
  let mut accepted = command(&fixture.client_dir);
  accepted.args(["client", "connect", &destination.url]);
  let mut terminal = Terminal::new(accepted);
  terminal.reply("Continue?", "y\r");
  assert!(terminal.finish().success());
  let config = app::cli::local_config::read(&config_path).unwrap();
  assert_eq!(config.server_url.as_deref(), Some(destination.url.as_str()));
  assert!(config.default_environment.is_none());
  assert!(!fixture.client_dir.join("session").exists());
  assert!(!fixture.client_dir.join("session-key").exists());
}
