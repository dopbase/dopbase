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
    email: Some("admin@dopbase.com".into()),
  };

  let value = status_document(&server, &credential, true, Some("Member"));
  assert_eq!(value["authentication"], "encrypted_session");
  assert_eq!(value["identity"], "admin");
  assert_eq!(value["role"], "Member");
  assert_eq!(value["email"], "admin@dopbase.com");
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

  let value = status_document(&server, &credential, true, None);
  assert_eq!(value["authentication"], "encrypted_session");
  assert_eq!(value["identity"], "runner");
  assert!(value["role"].is_null());
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
    ("dpp_personal-secret", "personal_token"),
    ("other-secret", "unknown"),
  ] {
    let credential = Credential {
      token: Some(token.into()),
      source: CredentialSource::Environment,
      email: None,
    };

    let value = status_document(&server, &credential, true, None);
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

  let value = status_document(&server, &credential, false, None);
  assert_eq!(value["identity"], "none");
  assert!(value["role"].is_null());
  assert!(value["email"].is_null());
  assert!(value["environment"].is_null());
  assert_eq!(value["server_status"], "offline");
  assert_eq!(value["status_source"], "cache");
}

#[tokio::test(flavor = "multi_thread")]
async fn status_reports_the_role_and_how_the_cli_signed_in() {
  use super::support::Fixture;
  let fixture = Fixture::new().await;
  fixture.environment().await;
  assert_eq!(fixture.json(&["client", "status"]).await["role"], "Root");

  fixture.sign_in_member().await;
  let member = fixture.json(&["client", "status"]).await;
  assert_eq!(member["identity"], "admin");
  assert_eq!(member["role"], "Member");
  let human = fixture.run(&["client", "status"]).await;
  assert!(human.status.success());
  let text = String::from_utf8_lossy(&human.stdout);
  assert!(text.contains("dopbase login (saved session)"), "{text}");
  assert!(text.contains("Role:") && text.contains("Member"), "{text}");
  assert!(!text.contains("Identity:"), "{text}");

  let token = fixture.agent_token().await;
  let mut agent = fixture.command();
  agent
    .env("DOPBASE_TOKEN", token["plaintextToken"].as_str().unwrap())
    .args(["client", "status"]);
  let agent = super::support::output(agent, None).await;
  let text = String::from_utf8_lossy(&agent.stdout);
  // Agent tokens share the personal prefix; the owner gives them away.
  assert!(text.contains("AI agent token (DOPBASE_TOKEN)"), "{text}");
  assert!(
    text.contains("AI agent (the token may reach less"),
    "{text}"
  );
  assert!(text.contains("metadata-agent"), "{text}");
  assert!(text.contains("Owner:"), "{text}");
  let mut agent = fixture.command();
  agent
    .env("DOPBASE_TOKEN", token["plaintextToken"].as_str().unwrap())
    .args(["--json", "client", "status"]);
  let agent = super::support::success(&super::support::output(agent, None).await);
  assert_eq!(agent["identity"], "ai_agent");
  assert!(agent["owner"].is_string(), "{agent}");
  fixture.stop().await;
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

/// Holds the instance lock for the client's data directory, the way a
/// foreground `dopbase server start` would.
#[cfg(unix)]
fn hold_foreground_lock(data_dir: &std::path::Path) -> app::server::InstanceLock {
  let config = app::config::ServerConfig::load(&app::config::ServerOverrides {
    data_dir: Some(data_dir.to_path_buf()),
    ..app::config::ServerOverrides::default()
  })
  .unwrap();
  app::server::InstanceLock::acquire(&config.database_url)
    .unwrap()
    .unwrap()
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn client_connect_ignores_a_foreground_server_it_is_not_leaving() {
  use super::support::{Fixture, command, terminal::Terminal};
  let fixture = Fixture::new().await;
  let destination = Fixture::new().await;
  let config_path = fixture.client_dir.join("config.toml");
  let mut config = app::cli::local_config::read(&config_path).unwrap();
  config.server_url = Some(fixture.url.clone());
  app::cli::local_config::write(&config_path, &config).unwrap();
  let before = std::fs::read(&config_path).unwrap();
  let _lock = hold_foreground_lock(&fixture.client_dir);

  let mut connect = command(&fixture.client_dir);
  connect.args(["client", "connect", &destination.url]);
  let mut terminal = Terminal::new(connect);
  terminal.reply("Continue?", "n\r");
  assert_eq!(terminal.finish().code(), Some(130));
  assert_eq!(std::fs::read(&config_path).unwrap(), before);
}

/// A loopback URL with nothing listening on it.
#[cfg(unix)]
fn unused_local_url() -> String {
  let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
  format!("http://{}", listener.local_addr().unwrap())
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn client_connect_local_works_while_the_local_instance_is_offline() {
  use super::support::{Fixture, command, terminal::Terminal};
  let fixture = Fixture::new().await;
  let config_path = fixture.client_dir.join("config.toml");
  let mut config = app::cli::local_config::read(&config_path).unwrap();
  config.server_url = Some(fixture.url.clone());
  app::cli::local_config::write(&config_path, &config).unwrap();
  let local_url = unused_local_url();

  let mut connect = command(&fixture.client_dir);
  connect
    .env("DOPBASE_PUBLIC_URL", &local_url)
    .args(["client", "connect", "local"]);
  let mut terminal = Terminal::new(connect);
  terminal.reply("Continue?", "y\r");
  assert!(terminal.finish().success(), "{}", terminal.transcript);
  assert!(
    terminal
      .transcript
      .contains(&format!("No server is running at {local_url} yet")),
    "{}",
    terminal.transcript
  );
  // `local` follows the instance's public URL, so a non-default one is saved.
  let config = app::cli::local_config::read(&config_path).unwrap();
  assert_eq!(config.server_url.as_deref(), Some(local_url.as_str()));
  assert!(!fixture.client_dir.join("session").exists());
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn client_connect_skips_the_live_check_only_for_this_machines_instance() {
  use super::support::{Fixture, command, failure, output, terminal::Terminal};
  let fixture = Fixture::new().await;
  let config_path = fixture.client_dir.join("config.toml");
  let mut config = app::cli::local_config::read(&config_path).unwrap();
  config.server_url = Some(fixture.url.clone());
  app::cli::local_config::write(&config_path, &config).unwrap();
  let before = std::fs::read(&config_path).unwrap();
  let local_url = unused_local_url();

  // No instance lives in this data directory yet, so the URL is just an
  // address that cannot be reached.
  let mut unreachable = command(&fixture.client_dir);
  unreachable
    .env("DOPBASE_PUBLIC_URL", &local_url)
    .args(["client", "connect", &local_url]);
  failure(
    &output(unreachable, None).await,
    &format!("Could not connect to Dopbase at {local_url}"),
  );
  assert_eq!(std::fs::read(&config_path).unwrap(), before);

  // Once the data directory holds the instance's database, its own URL is
  // accepted while the server is stopped.
  let server_config = app::config::ServerConfig::load(&app::config::ServerOverrides {
    data_dir: Some(fixture.client_dir.clone()),
    ..app::config::ServerOverrides::default()
  })
  .unwrap();
  let database = app::config::database_path(&server_config.database_url).unwrap();
  std::fs::write(&database, b"").unwrap();
  let mut own = command(&fixture.client_dir);
  own
    .env("DOPBASE_PUBLIC_URL", &local_url)
    .args(["client", "connect", &local_url]);
  let mut terminal = Terminal::new(own);
  terminal.reply("Continue?", "n\r");
  assert_eq!(terminal.finish().code(), Some(130));
  assert_eq!(std::fs::read(&config_path).unwrap(), before);
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn client_connect_local_refuses_a_non_dopbase_answer() {
  use super::support::{Fixture, command, failure, output, serve};
  let fixture = Fixture::new().await;
  let config_path = fixture.client_dir.join("config.toml");
  let mut config = app::cli::local_config::read(&config_path).unwrap();
  config.server_url = Some(fixture.url.clone());
  app::cli::local_config::write(&config_path, &config).unwrap();
  let before = std::fs::read(&config_path).unwrap();
  let (other_url, _task) = serve(axum::Router::new().route(
    "/api/v1/health",
    axum::routing::get(|| async {
      axum::Json(serde_json::json!({"success": true, "data": {"product": "something-else"}}))
    }),
  ))
  .await;

  let mut connect = command(&fixture.client_dir);
  connect
    .env("DOPBASE_PUBLIC_URL", &other_url)
    .args(["client", "connect", "local"]);
  failure(
    &output(connect, None).await,
    "the endpoint is not a compatible Dopbase v1 server",
  );
  assert_eq!(std::fs::read(&config_path).unwrap(), before);
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn client_connect_refuses_to_leave_the_local_foreground_server() {
  use super::support::{Fixture, command, failure, output};
  let fixture = Fixture::new().await;
  let destination = Fixture::new().await;
  let config_path = fixture.client_dir.join("config.toml");
  let mut config = app::cli::local_config::read(&config_path).unwrap();
  config.server_url = Some(fixture.url.clone());
  app::cli::local_config::write(&config_path, &config).unwrap();
  let before = std::fs::read(&config_path).unwrap();
  let _lock = hold_foreground_lock(&fixture.client_dir);

  // The local server's public URL is the current server, so this is a switch
  // away from a foreground server the CLI cannot stop.
  let mut connect = command(&fixture.client_dir);
  connect
    .env("DOPBASE_PUBLIC_URL", &fixture.url)
    .args(["client", "connect", &destination.url]);
  failure(
    &output(connect, None).await,
    &format!("{} is a foreground Dopbase server", fixture.url),
  );
  assert_eq!(std::fs::read(&config_path).unwrap(), before);
}

/// With nothing saved the CLI already talks to this machine's server at the
/// default URL. `local` then means "stay here", even when the public URL is
/// a different address for the same server.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn client_connect_local_stays_put_when_the_default_already_reaches_it() {
  use super::support::{Fixture, command, output};
  let fixture = Fixture::new().await;
  let config_path = fixture.client_dir.join("config.toml");
  let _ = std::fs::remove_file(&config_path);
  let session = fixture.client_dir.join("session");
  assert!(session.exists());

  let mut connect = command(&fixture.client_dir);
  connect
    .env("DOPBASE_PUBLIC_URL", "https://dopbase.internal")
    .args(["client", "connect", "local"]);
  let result = output(connect, None).await;
  let stdout = String::from_utf8_lossy(&result.stdout);
  assert!(
    result.status.success(),
    "{stdout}{}",
    String::from_utf8_lossy(&result.stderr)
  );
  assert!(
    stdout.contains("Already connected to http://localhost:8840"),
    "{stdout}"
  );
  assert!(session.exists(), "the CLI session was kept");
  let config = app::cli::local_config::read(&config_path).unwrap();
  assert_eq!(config.server_url, None);
}
