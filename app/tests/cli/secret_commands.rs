use app::cli::{
  local_config::{ClientConfig, ResolvedServer, ServerSource},
  session,
};
use axum::{Json, Router, extract::State, routing::get};
use serde_json::{Value, json};
use std::{
  io::Write,
  process::{Command, Stdio},
  sync::{Arc, Mutex},
};
use tempfile::TempDir;

const TOKEN: &str = "dbc_secret-format-test";
const SECRET_MARKER: &str = "secret-value-marker";

#[derive(Clone, Default)]
struct CapturedRequests {
  init: Arc<Mutex<Option<Value>>>,
  import: Arc<Mutex<Option<Value>>>,
}

async fn session_handler() -> Json<Value> {
  Json(json!({"data":{"email":"admin@example.com"}}))
}

async fn resolve_handler() -> Json<Value> {
  Json(json!({"data":{"id":"env_01TEST"}}))
}

async fn init_handler(
  State(requests): State<CapturedRequests>,
  Json(body): Json<Value>,
) -> Json<Value> {
  *requests.init.lock().unwrap() = Some(body);
  Json(json!({
    "data": {
      "project": {"id":"proj_01TEST"},
      "environmentId": "env_01TEST",
      "secretCount": 2
    }
  }))
}

async fn import_handler(
  State(requests): State<CapturedRequests>,
  Json(body): Json<Value>,
) -> Json<Value> {
  *requests.import.lock().unwrap() = Some(body);
  Json(json!({
    "data": {
      "addedKeys": ["API_KEY", "DATABASE_URL"],
      "updatedKeys": [],
      "unchangedKeys": [],
      "deletedKeys": [],
      "dryRun": true,
      "revision": "revision-1"
    }
  }))
}

async fn start_server() -> (CapturedRequests, String, tokio::task::JoinHandle<()>) {
  let requests = CapturedRequests::default();
  let router = Router::new()
    .route("/api/v1/auth/session", get(session_handler))
    .route("/api/v1/environments/resolve", get(resolve_handler))
    .route("/api/v1/projects/init", axum::routing::post(init_handler))
    .route(
      "/api/v1/environments/{id}/secrets/import",
      axum::routing::post(import_handler),
    )
    .with_state(requests.clone());
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let address = listener.local_addr().unwrap();
  let task = tokio::spawn(async move {
    axum::serve(listener, router).await.unwrap();
  });
  (requests, format!("http://{address}"), task)
}

fn save_session(
  directory: &TempDir,
  url: &str,
) {
  let server = ResolvedServer {
    url: url.into(),
    source: ServerSource::Argument,
    config_path: directory.path().join("config.toml"),
    config: ClientConfig::default(),
  };
  session::save(&server, TOKEN, Some("admin@example.com")).unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn init_reads_yaml_from_stdin_and_sends_string_entries() {
  let (requests, url, server) = start_server().await;
  let directory = TempDir::new().unwrap();
  save_session(&directory, &url);
  let mut child = Command::new(env!("CARGO_BIN_EXE_dopbase"))
    .args([
      "--server",
      &url,
      "--data-dir",
      directory.path().to_str().unwrap(),
      "--json",
      "init",
      "storefront",
      "development",
      "--from",
      "-",
      "--format",
      "yaml",
    ])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .unwrap();
  child
    .stdin
    .take()
    .unwrap()
    .write_all(format!("API_KEY: {SECRET_MARKER}\nEMPTY: \"\"\n").as_bytes())
    .unwrap();
  let output = child.wait_with_output().unwrap();
  server.abort();

  assert!(output.status.success(), "{output:?}");
  let body = requests.init.lock().unwrap().clone().unwrap();
  assert_eq!(body["projectName"], "storefront");
  assert_eq!(body["environmentName"], "development");
  assert_eq!(
    body["entries"][0],
    json!({"key":"API_KEY","value":SECRET_MARKER})
  );
  assert_eq!(body["entries"][1], json!({"key":"EMPTY","value":""}));
  assert!(!String::from_utf8_lossy(&output.stdout).contains(SECRET_MARKER));
  assert!(!String::from_utf8_lossy(&output.stderr).contains(SECRET_MARKER));
}

#[tokio::test(flavor = "multi_thread")]
async fn import_infers_json_from_the_filename_and_preserves_dry_run() {
  let (requests, url, server) = start_server().await;
  let directory = TempDir::new().unwrap();
  save_session(&directory, &url);
  let source = directory.path().join("secrets.json");
  std::fs::write(
    &source,
    format!(r#"{{"DATABASE_URL":"{SECRET_MARKER}","API_KEY":"token"}}"#),
  )
  .unwrap();
  let output = Command::new(env!("CARGO_BIN_EXE_dopbase"))
    .args([
      "--server",
      &url,
      "--data-dir",
      directory.path().to_str().unwrap(),
      "--json",
      "import",
      "storefront/development",
      source.to_str().unwrap(),
      "--dry-run",
    ])
    .output()
    .unwrap();
  server.abort();

  assert!(output.status.success(), "{output:?}");
  let body = requests.import.lock().unwrap().clone().unwrap();
  assert_eq!(body["mode"], "merge");
  assert_eq!(body["dryRun"], true);
  assert_eq!(body["entries"][0], json!({"key":"API_KEY","value":"token"}));
  assert_eq!(
    body["entries"][1],
    json!({"key":"DATABASE_URL","value":SECRET_MARKER})
  );
  assert!(!String::from_utf8_lossy(&output.stdout).contains(SECRET_MARKER));
  assert!(!String::from_utf8_lossy(&output.stderr).contains(SECRET_MARKER));
}
