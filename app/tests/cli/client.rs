use app::cli::{
  client::{
    ApiClient, Credential, CredentialSource, authenticated_client, is_authentication_error,
    is_conflict_error, normalize_login_email,
  },
  local_config::{ClientConfig, ResolvedServer, ServerSource},
};
use reqwest::Method;
use std::process::{Command as ProcessCommand, Stdio};
use tempfile::TempDir;
use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::TcpListener,
  time::{Duration, timeout},
};

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
fn run_client_requires_existing_authentication_without_prompting() {
  let directory = TempDir::new().unwrap();
  let server = ResolvedServer {
    url: "http://localhost:8840".into(),
    source: ServerSource::Default,
    config_path: directory.path().join("config.toml"),
    config: ClientConfig::default(),
  };
  let error = authenticated_client(
    &server,
    Credential {
      token: None,
      source: CredentialSource::None,
      email: None,
    },
  )
  .err()
  .unwrap()
  .to_string();
  assert_eq!(
    error,
    "Dopbase authentication is required. Run `dopbase login` first or set DOPBASE_TOKEN."
  );
}

async fn response_error(
  status: &str,
  body: &str,
) -> anyhow::Error {
  let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
  let address = listener.local_addr().unwrap();
  let status = status.to_owned();
  let body = body.to_owned();
  let server_task = tokio::spawn(async move {
    let (mut stream, _) = listener.accept().await.unwrap();
    let mut request = [0_u8; 2048];
    let _ = stream.read(&mut request).await.unwrap();
    stream
      .write_all(
        format!(
          "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
          body.len()
        )
        .as_bytes(),
      )
      .await
      .unwrap();
  });
  let directory = TempDir::new().unwrap();
  let server = ResolvedServer {
    url: format!("http://{address}"),
    source: ServerSource::Argument,
    config_path: directory.path().join("config.toml"),
    config: ClientConfig::default(),
  };

  let error = ApiClient::new(&server, Some("test-token".into()))
    .unwrap()
    .request(Method::GET, "/api/v1/environments/resolve", None)
    .await
    .unwrap_err();
  server_task.await.unwrap();
  error
}

#[tokio::test]
async fn api_client_classifies_response_statuses() {
  let error = response_error(
    "401 Unauthorized",
    r#"{"error":{"AUTHENTICATION_INVALID":"The provided credential is invalid."}}"#,
  )
  .await;
  assert!(is_authentication_error(&error));
  assert!(error.to_string().contains("AUTHENTICATION_INVALID"));

  let error = response_error(
    "409 Conflict",
    r#"{"error":{"PROJECT_ALREADY_EXISTS":"A project with this name already exists."}}"#,
  )
  .await;
  assert!(is_conflict_error(&error));
  assert!(error.to_string().contains("PROJECT_ALREADY_EXISTS"));
}

#[test]
fn plaintext_access_rejects_non_interactive_execution_before_connecting() {
  let directory = TempDir::new().unwrap();
  let output = ProcessCommand::new(env!("CARGO_BIN_EXE_dopbase"))
    .args([
      "--server",
      "http://127.0.0.1:1",
      "--data-dir",
      directory.path().to_str().unwrap(),
      "secret",
      "get",
      "billing/production",
      "API_KEY",
      "--reveal",
    ])
    .stdin(Stdio::null())
    .output()
    .unwrap();

  assert!(!output.status.success(), "{output:?}");
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(
    stderr.contains("interactive password confirmation is required for plaintext secret access"),
    "{stderr}"
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
Check that the server is running and verify the active endpoint with `dopbase client status`."
  );
  assert!(!message.contains("GET /api/v1/environments"));
  assert!(!message.contains("Request:"));
  assert!(!message.contains("error sending request"));
  assert!(!message.contains("tcp connect error"));
  assert!(!message.contains("os error"));
}

#[tokio::test]
async fn api_client_does_not_follow_redirects() {
  let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
  let address = listener.local_addr().unwrap();
  let server_task = tokio::spawn(async move {
    let (mut stream, _) = listener.accept().await.unwrap();
    let mut request = [0_u8; 2048];
    let _ = stream.read(&mut request).await.unwrap();
    stream
      .write_all(
        b"HTTP/1.1 302 Found\r\nLocation: /redirected\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
      )
      .await
      .unwrap();
    stream.shutdown().await.unwrap();
    timeout(Duration::from_millis(250), listener.accept())
      .await
      .is_err()
  });
  let directory = TempDir::new().unwrap();
  let server = ResolvedServer {
    url: format!("http://{address}"),
    source: ServerSource::Argument,
    config_path: directory.path().join("config.toml"),
    config: ClientConfig::default(),
  };

  let error = ApiClient::new(&server, Some("secret-token".into()))
    .unwrap()
    .request(Method::GET, "/start", None)
    .await
    .unwrap_err()
    .to_string();

  assert!(error.contains("server returned 302 Found"), "{error}");
  assert!(server_task.await.unwrap(), "client followed the redirect");
}
