use app::cli::{
  client::{Credential, CredentialSource, credential_from_sources, validate_runner_token},
  commands::{
    complete_factory_reset, factory_reset_archive_path, factory_reset_confirmation_matches,
    factory_reset_quarantine_path, insecure_transport_warning, run_environment,
    server_switch_confirmed, status_document, validate_factory_reset_target,
  },
  local_config::{ClientConfig, DefaultEnvironment, ResolvedServer, ServerSource},
};

mod cache;

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

#[test]
fn factory_reset_creates_a_zip_and_removes_the_data_directory() {
  use std::io::Read;

  let directory = TempDir::new().unwrap();
  let data_dir = directory.path().join("instance");
  let nested = data_dir.join("backups");
  std::fs::create_dir_all(&nested).unwrap();
  std::fs::write(data_dir.join("dopbase.db"), b"database").unwrap();
  std::fs::write(nested.join("existing.dop"), b"backup").unwrap();

  let archive = complete_factory_reset(&data_dir, false).unwrap().unwrap();
  assert!(!data_dir.exists());
  assert!(archive.exists());
  assert_eq!(
    archive,
    factory_reset_archive_path(
      &archive.with_file_name(
        archive
          .file_name()
          .unwrap()
          .to_string_lossy()
          .trim_end_matches(".zip")
      )
    )
  );

  let file = std::fs::File::open(&archive).unwrap();
  let mut zip = zip::ZipArchive::new(file).unwrap();
  let root = "instance";
  let mut database = String::new();
  zip
    .by_name(&format!("{root}/dopbase.db"))
    .unwrap()
    .read_to_string(&mut database)
    .unwrap();
  assert_eq!(database, "database");
  assert!(zip.by_name(&format!("{root}/backups/existing.dop")).is_ok());

  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(
      std::fs::metadata(&archive).unwrap().permissions().mode() & 0o777,
      0o600
    );
  }
}

#[test]
fn factory_reset_no_backup_removes_data_without_an_archive() {
  let directory = TempDir::new().unwrap();
  let data_dir = directory.path().join("instance");
  std::fs::create_dir_all(&data_dir).unwrap();
  std::fs::write(data_dir.join("dopbase.db"), b"database").unwrap();

  assert!(complete_factory_reset(&data_dir, true).unwrap().is_none());
  assert!(!data_dir.exists());
  assert!(
    std::fs::read_dir(directory.path())
      .unwrap()
      .next()
      .is_none()
  );
}

#[cfg(unix)]
#[test]
fn factory_reset_restores_data_when_the_zip_cannot_include_a_symlink() {
  use std::os::unix::fs::symlink;

  let directory = TempDir::new().unwrap();
  let data_dir = directory.path().join("instance");
  std::fs::create_dir_all(&data_dir).unwrap();
  std::fs::write(data_dir.join("dopbase.db"), b"database").unwrap();
  symlink(directory.path(), data_dir.join("outside")).unwrap();

  let error = complete_factory_reset(&data_dir, false)
    .unwrap_err()
    .to_string();
  assert!(error.contains("ZIP backup could not be created"), "{error}");
  assert!(data_dir.exists());
  assert!(data_dir.join("outside").is_symlink());
  assert_eq!(
    std::fs::read_dir(directory.path())
      .unwrap()
      .filter_map(Result::ok)
      .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "zip"))
      .count(),
    0
  );
}

#[test]
fn insecure_transport_warning_names_the_server_and_the_risk() {
  assert_eq!(
    insecure_transport_warning("http://192.168.1.20:8840"),
    "Plain HTTP does not encrypt traffic to http://192.168.1.20:8840. Credentials and secrets could be exposed. Use HTTPS whenever possible."
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
    message.contains("dopbase env default <ENVIRONMENT_REF>"),
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
fn explicit_runner_token_has_priority_over_environment_and_saved_credentials() {
  let explicit = "dbs_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
  let credential = credential_from_sources(
    Some(explicit.into()),
    Ok("environment-token".into()),
    || panic!("saved credentials must not be loaded for an explicit token"),
  )
  .unwrap();

  assert_eq!(credential.token.as_deref(), Some(explicit));
  assert!(matches!(credential.source, CredentialSource::Argument));
}

#[test]
fn environment_token_has_priority_over_the_saved_credential() {
  let environment = "dbs_BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB";
  let credential = credential_from_sources(None, Ok(environment.into()), || {
    panic!("saved credentials must not be loaded when DOPBASE_TOKEN is set")
  })
  .unwrap();

  assert_eq!(credential.token.as_deref(), Some(environment));
  assert!(matches!(credential.source, CredentialSource::Environment));
}

#[test]
fn runner_token_validation_rejects_empty_wrong_prefix_and_wrong_length() {
  assert!(validate_runner_token("dbs_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA").is_ok());
  for token in [
    "",
    "dbc_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "dbs_too-short",
    "dbs_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA!",
  ] {
    assert!(validate_runner_token(token).is_err(), "accepted {token:?}");
  }
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
mod environment;
mod import;
mod run;
mod update;
