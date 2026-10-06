#![cfg(any(target_os = "linux", target_os = "macos"))]

use axum::{
  Json, Router,
  extract::State,
  http::{HeaderMap, StatusCode},
  response::{IntoResponse, Response},
  routing::{get, post},
};
use serde_json::{Value, json};
use std::{
  fs,
  io::{Read, Write},
  process::{Command, Stdio},
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
  thread,
  time::{Duration, Instant},
};
use tempfile::TempDir;

#[derive(Clone, Default)]
struct ExportState {
  recent: Arc<AtomicBool>,
  requested: Arc<AtomicBool>,
}

async fn reauthenticate(
  State(state): State<ExportState>,
  Json(body): Json<Value>,
) -> Response {
  if body["password"] != "fixture-password" {
    return (
      StatusCode::UNAUTHORIZED,
      Json(json!({"error":{"PASSWORD_INVALID":"Incorrect password."}})),
    )
      .into_response();
  }
  state.recent.store(true, Ordering::SeqCst);
  Json(json!({"data":null})).into_response()
}

async fn export(
  State(state): State<ExportState>,
  headers: HeaderMap,
) -> Response {
  state.requested.store(true, Ordering::SeqCst);
  if !state.recent.load(Ordering::SeqCst) {
    return StatusCode::FORBIDDEN.into_response();
  }
  if headers
    .get("content-length")
    .and_then(|value| value.to_str().ok())
    != Some("0")
  {
    return (StatusCode::LENGTH_REQUIRED, "<html>Length Required</html>").into_response();
  }
  Json(json!({"data":{"entries":[{"key":"API_KEY","value":"fixture-secret"}]}})).into_response()
}

#[tokio::test(flavor = "multi_thread")]
async fn stdout_export_prints_only_secrets_after_password_confirmation() {
  run_interactive_export(b"fixture-password\n", 0).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn wrong_password_prevents_export_and_plaintext_output() {
  run_interactive_export(b"wrong-password\n", 1).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn cancelling_password_confirmation_prevents_export_and_plaintext_output() {
  run_interactive_export(b"\x03", 130).await;
}

async fn run_interactive_export(
  password_input: &[u8],
  expected_exit: i32,
) {
  let state = ExportState::default();
  let router = Router::new()
    .route(
      "/api/v1/auth/session",
      get(|| async { Json(json!({"data":{"email":"fixture@example.com"}})) }),
    )
    .route("/api/v1/auth/reauthenticate", post(reauthenticate))
    .route(
      "/api/v1/environments/resolve",
      get(|| async { Json(json!({"data":{"id":"env_fixture"}})) }),
    )
    .route(
      "/api/v1/environments/env_fixture/secrets/export",
      post(export),
    )
    .with_state(state.clone());
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let url = format!("http://{}", listener.local_addr().unwrap());
  let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
  let directory = TempDir::new().unwrap();
  let launcher = directory.path().join("export.sh");
  let output = directory.path().join("secrets.env");
  fs::write(
    &launcher,
    "stty rows 24 cols 80\nexec \"$DOPBASE_BIN\" --server \"$DOPBASE_SERVER\" --data-dir \"$DOPBASE_DATA_DIR\" export demo/local --stdout > \"$DOPBASE_OUTPUT\"\n",
  )
  .unwrap();
  let mut command = Command::new("script");
  #[cfg(target_os = "macos")]
  command.args(["-q", "-e", "/dev/null", "sh"]).arg(&launcher);
  #[cfg(target_os = "linux")]
  command.args([
    "-q",
    "-e",
    "-f",
    "-c",
    "sh \"$DOPBASE_TEST_SCRIPT\"",
    "/dev/null",
  ]);
  command
    .env("DOPBASE_BIN", env!("CARGO_BIN_EXE_dopbase"))
    .env("DOPBASE_SERVER", &url)
    .env("DOPBASE_DATA_DIR", directory.path())
    .env("DOPBASE_OUTPUT", &output)
    .env("DOPBASE_TEST_SCRIPT", &launcher)
    .env("DOPBASE_TOKEN", "dbc_export_fixture")
    .env("TERM", "xterm")
    .env_remove("DOPBASE_URL")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::null());
  let mut child = command.spawn().unwrap();
  let mut input = child.stdin.take().unwrap();
  let mut terminal = child.stdout.take().unwrap();
  let prompted = Arc::new(AtomicBool::new(false));
  let observed_prompt = prompted.clone();
  let reader = thread::spawn(move || {
    let mut captured = Vec::new();
    let mut buffer = [0_u8; 1024];
    while let Ok(count) = terminal.read(&mut buffer) {
      if count == 0 {
        break;
      }
      captured.extend_from_slice(&buffer[..count]);
      if captured.windows(9).any(|bytes| bytes == b"Password:") {
        observed_prompt.store(true, Ordering::SeqCst);
      }
    }
    captured
  });
  let deadline = Instant::now() + Duration::from_secs(5);
  let mut entered_password = false;
  let status = loop {
    if !entered_password && prompted.load(Ordering::SeqCst) {
      input.write_all(password_input).unwrap();
      entered_password = true;
    }
    if let Some(status) = child.try_wait().unwrap() {
      break status;
    }
    if Instant::now() >= deadline {
      let _ = child.kill();
      let _ = child.wait();
      server.abort();
      panic!("interactive export did not finish");
    }
    thread::sleep(Duration::from_millis(10));
  };
  drop(input);
  let terminal_output = reader.join().unwrap();
  server.abort();
  assert!(
    entered_password,
    "export did not request password confirmation"
  );
  assert_eq!(status.code(), Some(expected_exit));
  let success = expected_exit == 0;
  assert_eq!(state.recent.load(Ordering::SeqCst), success);
  assert_eq!(state.requested.load(Ordering::SeqCst), success);
  assert_eq!(
    fs::read_to_string(output).unwrap(),
    if success {
      "API_KEY=fixture-secret\n"
    } else {
      ""
    }
  );
  let terminal_output = String::from_utf8_lossy(&terminal_output);
  assert!(!terminal_output.contains("fixture-secret"));
  if expected_exit == 1 {
    assert!(terminal_output.contains("PASSWORD_INVALID"));
  }
  if expected_exit == 130 {
    assert!(terminal_output.contains("Password confirmation cancelled."));
  }
}

#[tokio::test(flavor = "multi_thread")]
async fn file_export_preserves_existing_files_unless_force_is_passed() {
  use super::support::{Fixture, PASSWORD, terminal::Terminal};
  let fixture = Fixture::new().await;
  let id = fixture.environment().await;
  fixture
    .request(
      reqwest::Method::PUT,
      &format!("/api/v1/environments/{id}/secrets/API_KEY"),
      Some(json!({"value":"file-export-private-marker"})),
    )
    .await;
  let path = fixture.directory.path().join("export.json");
  for (force, expected_success) in [(false, true), (false, false), (true, true)] {
    let mut command = fixture.command();
    command.args(["export", &id, "--output", path.to_str().unwrap()]);
    if force {
      command.arg("--force");
    }
    let mut terminal = Terminal::new(command);
    terminal.reply("Password:", &format!("{PASSWORD}\r"));
    assert_eq!(terminal.finish().success(), expected_success);
    assert!(!terminal.transcript.contains("file-export-private-marker"));
    if expected_success {
      assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(&path).unwrap()).unwrap(),
        json!({"API_KEY":"file-export-private-marker"})
      );
      #[cfg(unix)]
      {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
          fs::metadata(&path).unwrap().permissions().mode() & 0o777,
          0o600
        );
      }
      fs::write(&path, b"existing-file-marker").unwrap();
    } else {
      assert_eq!(fs::read(&path).unwrap(), b"existing-file-marker");
    }
  }
}

async fn runner_fixture() -> (super::support::Fixture, String, Value) {
  let fixture = super::support::Fixture::new().await;
  let id = fixture.environment().await;
  fixture.request(reqwest::Method::POST, &format!("/api/v1/environments/{id}/secrets/import"),
    Some(json!({"mode":"merge","entries":[{"key":"API_KEY","value":"runner-private-marker"}],"envLayout":"# deployment\nAPI_KEY=\n"}))).await;
  let created = fixture
    .json(&["token", "create", &id, "--name", "pipeline"])
    .await;
  (fixture, id, created)
}

#[tokio::test(flavor = "multi_thread")]
async fn runner_export_uses_injected_and_saved_credentials_and_audits_the_snapshot() {
  use super::support::{failure, output, success};
  let (fixture, id, created) = runner_fixture().await;
  let token = created["plaintextToken"].as_str().unwrap();
  let path = fixture.directory.path().join("runner.json");
  let mut command = fixture.command();
  command
    .env("DOPBASE_TOKEN", token)
    .args(["--json", "export", "fixture/local", "--output"])
    .arg(&path);
  let result = output(command, None).await;
  assert_eq!(success(&result)["secret_count"], 1);
  assert_eq!(
    serde_json::from_slice::<Value>(&fs::read(&path).unwrap()).unwrap(),
    json!({"API_KEY":"runner-private-marker"})
  );
  use std::os::unix::fs::PermissionsExt;
  assert_eq!(
    fs::metadata(&path).unwrap().permissions().mode() & 0o777,
    0o600
  );
  assert!(!String::from_utf8_lossy(&result.stdout).contains(token));
  assert!(!String::from_utf8_lossy(&result.stderr).contains("runner-private-marker"));

  let mut command = fixture.command();
  command.args(["login", "--token"]);
  assert!(
    output(command, Some(token.as_bytes()))
      .await
      .status
      .success()
  );
  let mut command = fixture.command();
  command.args(["export", &id, "--stdout"]);
  let result = output(command, None).await;
  assert!(result.status.success());
  assert_eq!(result.stdout, b"API_KEY=runner-private-marker\n");
  assert!(!String::from_utf8_lossy(&result.stderr).contains("Password"));

  fs::write(&path, b"existing-file-marker").unwrap();
  let mut command = fixture.command();
  command.args(["export", &id, "--output"]).arg(&path);
  failure(&output(command, None).await, "already exists");
  assert_eq!(fs::read(&path).unwrap(), b"existing-file-marker");
  let mut command = fixture.command();
  command
    .args(["export", &id, "--output"])
    .arg(&path)
    .args(["--force", "--format", "dotenv"]);
  assert!(output(command, None).await.status.success());
  assert_eq!(fs::read(&path).unwrap(), b"API_KEY=runner-private-marker\n");

  let api = reqwest::Client::new();
  let endpoint = format!("{}/api/v1/environments/{id}/secrets/export", fixture.url);
  let runner: Value = api
    .post(&endpoint)
    .bearer_auth(token)
    .send()
    .await
    .unwrap()
    .error_for_status()
    .unwrap()
    .json()
    .await
    .unwrap();
  let human = fixture
    .request(
      reqwest::Method::POST,
      &format!("/api/v1/environments/{id}/secrets/export"),
      None,
    )
    .await;
  assert_eq!(runner["data"], human);
  assert_eq!(runner["data"]["envLayout"], "# deployment\nAPI_KEY=\n");
  assert!(runner["data"]["revision"].is_string());
  let audits = fixture
    .request(
      reqwest::Method::GET,
      "/api/v1/audit-events?action=secret.exported",
      None,
    )
    .await;
  let runner_audits: Vec<_> = audits["items"]
    .as_array()
    .unwrap()
    .iter()
    .filter(|entry| entry["actorType"] == "runner")
    .collect();
  assert_eq!(runner_audits.len(), 5);
  for entry in runner_audits {
    assert_eq!(entry["actorId"], created["token"]["id"]);
    assert_eq!(entry["environmentId"], id);
    assert!(entry["projectId"].is_string());
    assert_eq!(entry["metadata"], json!({"count":1}));
  }
  assert!(!audits.to_string().contains(token));
  assert!(!audits.to_string().contains("runner-private-marker"));
}

#[tokio::test(flavor = "multi_thread")]
async fn runner_export_rejects_other_environments_and_invalid_credentials_without_fallback() {
  use super::support::{failure, output};
  let (fixture, id, created) = runner_fixture().await;
  let token = created["plaintextToken"].as_str().unwrap();
  let other = fixture.json(&["env", "create", "fixture/other"]).await["id"]
    .as_str()
    .unwrap()
    .to_owned();
  let path = fixture.directory.path().join("protected.env");
  fs::write(&path, b"existing-file-marker").unwrap();
  let mut command = fixture.command();
  command
    .env("DOPBASE_TOKEN", token)
    .args(["export", &other, "--output"])
    .arg(&path)
    .arg("--force");
  failure(&output(command, None).await, "TOKEN_SCOPE_INVALID");
  for target in [&other, "env_absent"] {
    let response = reqwest::Client::new()
      .post(format!(
        "{}/api/v1/environments/{target}/secrets/export",
        fixture.url
      ))
      .bearer_auth(token)
      .send()
      .await
      .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    let body: Value = response.json().await.unwrap();
    assert!(body["error"]["TOKEN_SCOPE_INVALID"].is_string());
  }
  for (credential, error) in [
    (
      "dbs_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
      "AUTHENTICATION_INVALID",
    ),
    ("dbs_too-short", "runner token"),
    ("", "DOPBASE_TOKEN is set but empty"),
    (fixture.token.as_str(), "interactive password confirmation"),
  ] {
    let mut command = fixture.command();
    command
      .env("DOPBASE_TOKEN", credential)
      .args(["export", &id, "--output"])
      .arg(&path)
      .arg("--force");
    let result = output(command, None).await;
    failure(&result, error);
    assert!(!String::from_utf8_lossy(&result.stderr).contains("runner-private-marker"));
    assert_eq!(fs::read(&path).unwrap(), b"existing-file-marker");
  }
  let token_id = created["token"]["id"].as_str().unwrap();
  for column in ["expires_at", "revoked_at"] {
    sqlx::query(&format!("UPDATE runner_tokens SET {column}=? WHERE id=?"))
      .bind((chrono::Utc::now() - chrono::Duration::seconds(1)).to_rfc3339())
      .bind(token_id)
      .execute(fixture.state.db.pool())
      .await
      .unwrap();
    let mut command = fixture.command();
    command
      .env("DOPBASE_TOKEN", token)
      .args(["export", &id, "--stdout"]);
    let result = output(command, None).await;
    failure(&result, "AUTHENTICATION_INVALID");
    assert!(result.stdout.is_empty());
    let response = reqwest::Client::new()
      .post(format!(
        "{}/api/v1/environments/{id}/secrets/export",
        fixture.url
      ))
      .bearer_auth(token)
      .send()
      .await
      .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
    sqlx::query(&format!(
      "UPDATE runner_tokens SET {column}=NULL WHERE id=?"
    ))
    .bind(token_id)
    .execute(fixture.state.db.pool())
    .await
    .unwrap();
  }
  let count: i64 =
    sqlx::query_scalar("SELECT COUNT(*) FROM audit_events WHERE action='secret.exported'")
      .fetch_one(fixture.state.db.pool())
      .await
      .unwrap();
  assert_eq!(count, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_export_is_denied_by_the_cli_and_server() {
  use super::support::{failure, output};
  let (fixture, id, _) = runner_fixture().await;
  let created = fixture.agent_token().await;
  let token = created["plaintextToken"].as_str().unwrap();
  for saved in [false, true] {
    let mut command = fixture.command();
    if saved {
      app::cli::session::save(&fixture.resolved(), token, None).unwrap();
    } else {
      command.env("DOPBASE_TOKEN", token);
    }
    command.args(["export", &id, "--stdout"]);
    let result = output(command, None).await;
    failure(
      &result,
      "AI agents may access secret metadata but never secret values",
    );
    assert!(result.stdout.is_empty());
  }
  let response = reqwest::Client::new()
    .post(format!(
      "{}/api/v1/environments/{id}/secrets/export",
      fixture.url
    ))
    .bearer_auth(token)
    .send()
    .await
    .unwrap();
  assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
  assert!(
    !response
      .text()
      .await
      .unwrap()
      .contains("runner-private-marker")
  );
}

#[tokio::test(flavor = "multi_thread")]
async fn runner_export_preserves_files_on_render_failure_and_never_falls_back_to_cache() {
  use super::support::{failure, output};
  use app::cli::{
    client::ApiClient,
    runtime_cache::{self, RuntimeSource},
  };
  let (fixture, id, created) = runner_fixture().await;
  let token = created["plaintextToken"].as_str().unwrap();
  let path = fixture.directory.path().join("protected.env");
  fs::write(&path, b"existing-file-marker").unwrap();
  fixture
    .request(
      reqwest::Method::PUT,
      &format!("/api/v1/environments/{id}/secrets/API_KEY"),
      Some(json!({"value":"private-line-one\nprivate-line-two"})),
    )
    .await;
  let mut command = fixture.command();
  command
    .env("DOPBASE_TOKEN", token)
    .args(["export", &id, "--output"])
    .arg(&path)
    .args(["--force", "--format", "docker"]);
  let result = output(command, None).await;
  failure(&result, "contains a line break");
  assert!(result.stdout.is_empty());
  assert!(!String::from_utf8_lossy(&result.stderr).contains("private-line-one"));
  assert_eq!(fs::read(&path).unwrap(), b"existing-file-marker");
  let server = fixture.resolved();
  let api = ApiClient::new(&server, Some(token.into())).unwrap();
  runtime_cache::load(&server, &api, &id).await.unwrap();
  fixture.stop().await;
  assert!(matches!(
    runtime_cache::load(&server, &api, &id)
      .await
      .unwrap()
      .source,
    RuntimeSource::Cache { .. }
  ));
  let mut command = fixture.command();
  command
    .env("DOPBASE_TOKEN", token)
    .args(["export", &id, "--output"])
    .arg(&path)
    .arg("--force");
  let result = output(command, None).await;
  failure(&result, "Could not connect");
  assert!(result.stdout.is_empty());
  assert_eq!(fs::read(&path).unwrap(), b"existing-file-marker");
}

#[tokio::test(flavor = "multi_thread")]
async fn browser_export_still_requires_csrf_and_recent_password_authentication() {
  let (fixture, id, _) = runner_fixture().await;
  let api = reqwest::Client::new();
  let (cookie, valid_csrf) = super::support::browser_credentials(&fixture).await;
  let endpoint = format!("{}/api/v1/environments/{id}/secrets/export", fixture.url);
  for csrf in [None, Some("invalid"), Some(valid_csrf.as_str())] {
    let mut request = api.post(&endpoint).header("cookie", &cookie);
    if let Some(csrf) = csrf {
      request = request.header("x-dopbase-csrf", csrf);
    }
    let response = request.send().await.unwrap();
    assert_eq!(
      response.status(),
      if csrf == Some(valid_csrf.as_str()) {
        reqwest::StatusCode::OK
      } else {
        reqwest::StatusCode::FORBIDDEN
      }
    );
  }
  sqlx::query("UPDATE sessions SET recent_auth_at=? WHERE kind='browser'")
    .bind((chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339())
    .execute(fixture.state.db.pool())
    .await
    .unwrap();
  let response = api
    .post(&endpoint)
    .header("cookie", cookie)
    .header("x-dopbase-csrf", valid_csrf.as_str())
    .send()
    .await
    .unwrap();
  assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
  let body: Value = response.json().await.unwrap();
  assert!(body["error"]["RECENT_AUTHENTICATION_REQUIRED"].is_string());
}
