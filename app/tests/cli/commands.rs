use app::cli::{
  client::{Credential, CredentialSource},
  commands::{
    factory_reset_confirmation_matches, factory_reset_quarantine_path, run_environment,
    server_switch_confirmed, status_document, validate_factory_reset_target,
  },
  local_config::{ClientConfig, DefaultEnvironment, ResolvedServer, ServerSource},
};

#[test]
fn factory_reset_requires_the_exact_confirmation_phrase() {
  assert!(factory_reset_confirmation_matches(
    "please-wipe-out-system\n"
  ));
  assert!(factory_reset_confirmation_matches(
    "please-wipe-out-system\r\n"
  ));
  for rejected in [
    "",
    "PLEASE-WIPE-OUT-SYSTEM",
    "please wipe out system",
    "please-wipe-out-system ",
    " please-wipe-out-system",
  ] {
    assert!(
      !factory_reset_confirmation_matches(rejected),
      "{rejected:?}"
    );
  }
}

#[test]
fn factory_reset_requires_a_local_database_inside_the_data_directory() {
  let directory = TempDir::new().unwrap();
  let data_dir = directory.path().join("instance");
  let database = data_dir.join("dopbase.db");

  let missing = validate_factory_reset_target(&data_dir, &database)
    .unwrap_err()
    .to_string();
  assert!(missing.contains("only available on the Dopbase server host"));

  std::fs::create_dir(&data_dir).unwrap();
  std::fs::write(&database, b"database").unwrap();
  assert_eq!(
    validate_factory_reset_target(&data_dir, &database).unwrap(),
    data_dir.canonicalize().unwrap()
  );

  let outside_database = directory.path().join("outside.db");
  std::fs::write(&outside_database, b"database").unwrap();
  let outside = validate_factory_reset_target(&data_dir, &outside_database)
    .unwrap_err()
    .to_string();
  assert!(outside.contains("does not contain its database"));
}

#[test]
fn factory_reset_quarantine_is_a_timestamped_sibling() {
  let directory = TempDir::new().unwrap();
  let data_dir = directory.path().join("instance");
  let quarantine = factory_reset_quarantine_path(&data_dir).unwrap();
  assert_eq!(quarantine.parent(), data_dir.parent());
  assert!(
    quarantine
      .file_name()
      .unwrap()
      .to_string_lossy()
      .starts_with("instance.factory-reset-")
  );
}
use std::env::VarError;
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
fn run_environment_uses_explicit_then_variable_then_saved_default() {
  assert_eq!(
    run_environment(
      Some("env_explicit".into()),
      Ok("env_variable".into()),
      Some("env_default")
    )
    .unwrap()
    .reference,
    "env_explicit"
  );
  assert_eq!(
    run_environment(None, Ok("env_variable".into()), Some("env_default"))
      .unwrap()
      .reference,
    "env_variable"
  );
  assert_eq!(
    run_environment(None, Err(VarError::NotPresent), Some("env_default"))
      .unwrap()
      .reference,
    "env_default"
  );
}

#[test]
fn run_environment_rejects_empty_variable_and_explains_how_to_set_a_default() {
  assert_eq!(
    run_environment(None, Ok(String::new()), Some("env_default"))
      .err()
      .unwrap()
      .to_string(),
    "DOPBASE_ENV is set but empty"
  );
  let message = run_environment(None, Err(VarError::NotPresent), None)
    .err()
    .unwrap()
    .to_string();
  assert!(
    message.contains("No default environment is set."),
    "{message}"
  );
  assert!(
    message.contains("dopbase env default <project/environment>"),
    "{message}"
  );
}

#[test]
fn run_environment_rejects_a_non_unicode_variable() {
  let invalid = std::ffi::OsString::from("invalid");
  assert_eq!(
    run_environment(
      None,
      Err(VarError::NotUnicode(invalid)),
      Some("env_default")
    )
    .err()
    .unwrap()
    .to_string(),
    "DOPBASE_ENV contains invalid Unicode"
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
    token: Some("secret-token".into()),
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
  assert!(!value.to_string().contains("secret-token"));
}

#[test]
fn status_identifies_runner_tokens_without_an_email() {
  let directory = TempDir::new().unwrap();
  let server = server(&directory);
  let credential = Credential {
    token: Some("runner-token".into()),
    source: CredentialSource::Environment,
    email: None,
  };

  let value = status_document(&server, &credential, true);
  assert_eq!(value["identity"], "runner");
  assert!(value["email"].is_null());
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

#[tokio::test]
async fn backup_rejects_when_server_is_offline() {
  use clap::Parser;

  let cli =
    app::cli::args::Cli::try_parse_from(["dopbase", "--server", "http://127.0.0.1:1", "backup"])
      .unwrap();

  let err = app::cli::commands::execute(cli).await.unwrap_err();
  let msg = err.to_string();
  assert!(
    msg.contains("Cannot perform backup: Dopbase server at http://127.0.0.1:1 is not connected or offline (live status required)"),
    "unexpected message: {msg}"
  );
}

#[tokio::test]
async fn restore_rejects_when_server_is_offline() {
  use clap::Parser;

  let dir = TempDir::new().unwrap();
  let backup_file = dir.path().join("test_backup.dop");
  std::fs::write(&backup_file, b"dummy content").unwrap();

  let cli = app::cli::args::Cli::try_parse_from([
    "dopbase",
    "--server",
    "http://127.0.0.1:1",
    "restore",
    backup_file.to_str().unwrap(),
    "--yes",
  ])
  .unwrap();

  let err = app::cli::commands::execute(cli).await.unwrap_err();
  let msg = err.to_string();
  assert!(
    msg.contains("Cannot perform restore: Dopbase server at http://127.0.0.1:1 is not connected or offline (live status required)"),
    "unexpected message: {msg}"
  );
}

#[tokio::test]
async fn factory_reset_rejects_remote_and_json_modes_before_touching_local_state() {
  use clap::Parser;

  let remote = app::cli::args::Cli::try_parse_from([
    "dopbase",
    "--server",
    "https://dopbase.example.com",
    "admin",
    "factory-reset",
  ])
  .unwrap();
  assert_eq!(
    app::cli::commands::execute(remote)
      .await
      .unwrap_err()
      .to_string(),
    "--server cannot be used with local `dopbase admin` commands"
  );

  let json =
    app::cli::args::Cli::try_parse_from(["dopbase", "--json", "admin", "factory-reset"]).unwrap();
  assert_eq!(
    app::cli::commands::execute(json)
      .await
      .unwrap_err()
      .to_string(),
    "--json cannot be used with `dopbase admin factory-reset`"
  );
}
