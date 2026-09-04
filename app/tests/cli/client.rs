use app::cli::{
  client::{ApiClient, CliCancelled, normalize_login_email, password_confirmed_human_client},
  local_config::{ClientConfig, ResolvedServer, ServerSource},
};
use reqwest::Method;
use std::io::IsTerminal;
use tempfile::TempDir;

#[test]
fn login_email_is_trimmed_lowercased_and_validated() {
  assert_eq!(
    normalize_login_email("  Admin@Example.COM  ").unwrap(),
    "admin@example.com"
  );
  assert_eq!(
    normalize_login_email("not-an-email")
      .unwrap_err()
      .to_string(),
    "Enter a valid email address."
  );
}

#[test]
fn login_cancelled_has_stable_user_message() {
  assert_eq!(CliCancelled::Login.to_string(), "Login cancelled.");
  assert_eq!(
    CliCancelled::PasswordConfirmation.to_string(),
    "Password confirmation cancelled."
  );
  assert_eq!(
    CliCancelled::ServerSwitch.to_string(),
    "Server switch cancelled."
  );
}

#[tokio::test]
async fn plaintext_access_rejects_non_interactive_execution_before_connecting() {
  if std::io::stdin().is_terminal() {
    return;
  }
  let directory = TempDir::new().unwrap();
  let server = ResolvedServer {
    url: "http://127.0.0.1:1".into(),
    source: ServerSource::Argument,
    config_path: directory.path().join("config.toml"),
    config: ClientConfig {
      version: 1,
      server_url: None,
      default_environment: None,
    },
  };

  let message = password_confirmed_human_client(&server)
    .await
    .err()
    .unwrap()
    .to_string();
  assert_eq!(
    message,
    "interactive password confirmation is required for plaintext secret access"
  );
}

#[tokio::test]
async fn connection_failure_is_concise_and_actionable() {
  let directory = TempDir::new().unwrap();
  let server = ResolvedServer {
    url: "http://127.0.0.1:1".into(),
    source: ServerSource::Argument,
    config_path: directory.path().join("config.toml"),
    config: ClientConfig {
      version: 1,
      server_url: None,
      default_environment: None,
    },
  };
  let client = ApiClient::new(&server, None).unwrap();

  let error = client
    .request(Method::GET, "/api/v1/environments", None)
    .await
    .err()
    .unwrap();
  let message = format!("{error:#}");

  assert_eq!(
    message,
    "Could not connect to Dopbase at http://127.0.0.1:1.\n\
Check that the server is running and verify the active endpoint with `dopbase status`. For local development, start it with `dopbase serve`."
  );
  assert!(!message.contains("GET /api/v1/environments"));
  assert!(!message.contains("Request:"));
  assert!(!message.contains("error sending request"));
  assert!(!message.contains("tcp connect error"));
  assert!(!message.contains("os error"));
}
