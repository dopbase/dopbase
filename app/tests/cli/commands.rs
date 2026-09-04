use app::cli::{
  client::{Credential, CredentialSource},
  commands::{run_environment, server_switch_confirmed, status_document},
  local_config::{ClientConfig, DefaultEnvironment, ResolvedServer, ServerSource},
};
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
