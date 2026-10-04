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
