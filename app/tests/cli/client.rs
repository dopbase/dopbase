use app::cli::{
  client::{
    ApiClient, CliCancelled, Credential, CredentialSource, authenticated_client,
    is_authentication_error, normalize_login_email, recently_authenticated_client,
  },
  local_config::{ClientConfig, ResolvedServer, ServerSource},
};
use reqwest::Method;
use std::io::IsTerminal;
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

#[tokio::test]
async fn api_client_preserves_unauthorized_responses_for_run() {
  let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
  let address = listener.local_addr().unwrap();
  let server_task = tokio::spawn(async move {
    let (mut stream, _) = listener.accept().await.unwrap();
    let mut request = [0_u8; 2048];
    let _ = stream.read(&mut request).await.unwrap();
    let body = r#"{"error":{"AUTHENTICATION_INVALID":"The provided credential is invalid."}}"#;
    stream
      .write_all(
        format!(
          "HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
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

  let error = ApiClient::new(&server, Some("invalid-token".into()))
    .unwrap()
    .request(Method::GET, "/api/v1/environments/resolve", None)
    .await
    .unwrap_err();
  assert!(is_authentication_error(&error));
  assert!(error.to_string().contains("AUTHENTICATION_INVALID"));
  server_task.await.unwrap();
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

  let message = recently_authenticated_client(&server)
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
Check that the server is running and verify the active endpoint with `dopbase client status`."
  );
  assert!(!message.contains("GET /api/v1/environments"));
  assert!(!message.contains("Request:"));
  assert!(!message.contains("error sending request"));
  assert!(!message.contains("tcp connect error"));
  assert!(!message.contains("os error"));
}

#[test]
fn api_client_accepts_a_validated_remote_http_server() {
  let directory = TempDir::new().unwrap();
  let server = ResolvedServer {
    url: "http://dopbase.example.com".into(),
    source: ServerSource::Argument,
    config_path: directory.path().join("config.toml"),
    config: ClientConfig::default(),
  };

  let client = ApiClient::new(&server, Some("secret-token".into())).unwrap();
  assert_eq!(client.base_url, "http://dopbase.example.com");
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
