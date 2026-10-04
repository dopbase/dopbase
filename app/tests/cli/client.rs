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

async fn api_response(
  method: Method,
  body: Option<serde_json::Value>,
  status: &str,
  response_body: &str,
  content_type: &str,
) -> (anyhow::Result<serde_json::Value>, String) {
  let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
  let address = listener.local_addr().unwrap();
  let status = status.to_owned();
  let response_body = response_body.to_owned();
  let content_type = content_type.to_owned();
  let server_task = tokio::spawn(async move {
    let (mut stream, _) = listener.accept().await.unwrap();
    let mut request = Vec::new();
    let mut buffer = [0_u8; 2048];
    let header_end = loop {
      let count = stream.read(&mut buffer).await.unwrap();
      assert!(count > 0, "request ended before its headers");
      request.extend_from_slice(&buffer[..count]);
      if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
        break end + 4;
      }
    };
    let headers = String::from_utf8_lossy(&request[..header_end]);
    let content_length = headers
      .lines()
      .find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name
          .eq_ignore_ascii_case("content-length")
          .then(|| value.trim().parse::<usize>().unwrap())
      })
      .unwrap_or(0);
    while request.len() < header_end + content_length {
      let count = stream.read(&mut buffer).await.unwrap();
      assert!(count > 0, "request ended before its body");
      request.extend_from_slice(&buffer[..count]);
    }
    stream
      .write_all(
        format!(
          "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response_body}",
          response_body.len()
        )
        .as_bytes(),
      )
      .await
      .unwrap();
    String::from_utf8(request).unwrap()
  });
  let directory = TempDir::new().unwrap();
  let server = ResolvedServer {
    url: format!("http://{address}"),
    source: ServerSource::Argument,
    config_path: directory.path().join("config.toml"),
    config: ClientConfig::default(),
  };
  let result = ApiClient::new(&server, Some("test-token".into()))
    .unwrap()
    .request(
      method,
      "/api/v1/environments/env_fixture/secrets/export",
      body,
    )
    .await;
  (result, server_task.await.unwrap())
}

async fn response_error(
  status: &str,
  body: &str,
) -> anyhow::Error {
  api_response(Method::GET, None, status, body, "application/json")
    .await
    .0
    .unwrap_err()
}

#[tokio::test]
async fn empty_export_post_has_an_explicit_zero_content_length() {
  let (result, request) = api_response(
    Method::POST,
    None,
    "200 OK",
    r#"{"data":{"entries":[{"key":"API_KEY","value":"fixture-value"}]}}"#,
    "application/json",
  )
  .await;
  assert!(request.starts_with("POST /api/v1/environments/env_fixture/secrets/export "));
  assert!(
    request
      .to_ascii_lowercase()
      .contains("\r\ncontent-length: 0\r\n")
  );
  assert!(request.ends_with("\r\n\r\n"));
  assert_eq!(result.unwrap()["entries"][0]["value"], "fixture-value");
}

#[tokio::test]
async fn json_requests_preserve_their_body_and_content_length() {
  for method in [Method::POST, Method::PUT, Method::PATCH] {
    let body = serde_json::json!({"password":"fixture-password","label":"日本語"});
    let expected_body = serde_json::to_string(&body).unwrap();
    let (result, request) = api_response(
      method,
      Some(body),
      "200 OK",
      r#"{"data":null}"#,
      "application/json",
    )
    .await;
    let (headers, sent_body) = request.split_once("\r\n\r\n").unwrap();
    let content_length = headers.lines().find_map(|line| {
      let (name, value) = line.split_once(':')?;
      name
        .eq_ignore_ascii_case("content-length")
        .then(|| value.trim())
    });
    assert_eq!(
      content_length,
      Some(expected_body.len().to_string().as_str())
    );
    assert_eq!(sent_body, expected_body);
    assert!(result.unwrap().is_null());
  }
}

#[tokio::test]
async fn bodyless_get_and_delete_requests_keep_their_existing_framing() {
  for method in [Method::GET, Method::DELETE] {
    let (result, request) = api_response(
      method,
      None,
      "200 OK",
      r#"{"data":null}"#,
      "application/json",
    )
    .await;
    assert!(!request.to_ascii_lowercase().contains("\r\ncontent-length:"));
    assert!(request.ends_with("\r\n\r\n"));
    assert!(result.unwrap().is_null());
  }
}

#[tokio::test]
async fn non_json_errors_preserve_status_without_exposing_response_bodies() {
  for (status, content_type, body) in [
    (
      "411 Length Required",
      "text/html",
      "<html>private-response-marker</html>",
    ),
    ("401 Unauthorized", "text/plain", "private-response-marker"),
    (
      "409 Conflict",
      "application/json",
      "private-response-marker",
    ),
    (
      "502 Bad Gateway",
      "text/html",
      "<html>private-response-marker</html>",
    ),
  ] {
    let (result, _) = api_response(Method::POST, None, status, body, content_type).await;
    let error = result.unwrap_err();
    assert_eq!(
      format!("{error:#}"),
      format!("server returned {status} (non-JSON response)")
    );
    assert_eq!(is_authentication_error(&error), status.starts_with("401"));
    assert_eq!(is_conflict_error(&error), status.starts_with("409"));
    assert!(!format!("{error:#}").contains("private-response-marker"));
  }
}

#[tokio::test]
async fn malformed_successful_responses_remain_errors() {
  let (result, _) = api_response(
    Method::GET,
    None,
    "200 OK",
    "<html>private-response-marker</html>",
    "text/html",
  )
  .await;
  let error = result.unwrap_err();
  let message = format!("{error:#}");
  assert!(message.contains("server returned an invalid JSON response"));
  assert!(!message.contains("private-response-marker"));
  assert!(!is_authentication_error(&error));
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
  for args in [
    vec!["secret", "get", "billing/production", "API_KEY", "--reveal"],
    vec!["export", "demo/local", "--stdout"],
  ] {
    let directory = TempDir::new().unwrap();
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_dopbase"))
      .args([
        "--server",
        "http://127.0.0.1:1",
        "--data-dir",
        directory.path().to_str().unwrap(),
      ])
      .args(args)
      .stdin(Stdio::null())
      .output()
      .unwrap();

    assert!(!output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
      stderr.contains("interactive password confirmation is required for plaintext secret access"),
      "{stderr}"
    );
  }
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

use app::cli::client::{credential_from_sources, validate_runner_token};
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
