use app::cli::{
  local_config::{ClientConfig, ResolvedServer, ServerSource},
  session,
};
use axum::{
  Json, Router,
  extract::{Path, State},
  routing::{get, post},
};
use serde_json::{Value, json};
use std::{
  process::Command,
  sync::{Arc, Mutex},
};
use tempfile::TempDir;

const TOKEN: &str = "dbc_environment-command-test";

#[derive(Clone, Default)]
struct CapturedRequest {
  project: Arc<Mutex<Option<String>>>,
  body: Arc<Mutex<Option<Value>>>,
}

async fn session_handler() -> Json<Value> {
  Json(json!({"data":{"email":"admin@example.com"}}))
}

async fn create_handler(
  Path(project): Path<String>,
  State(request): State<CapturedRequest>,
  Json(body): Json<Value>,
) -> Json<Value> {
  *request.project.lock().unwrap() = Some(project);
  *request.body.lock().unwrap() = Some(body);
  Json(json!({
    "data": {
      "id": "env_482731",
      "projectName": "storefront",
      "name": "staging"
    }
  }))
}

async fn start_server() -> (CapturedRequest, String, tokio::task::JoinHandle<()>) {
  let request = CapturedRequest::default();
  let router = Router::new()
    .route("/api/v1/auth/session", get(session_handler))
    .route(
      "/api/v1/projects/{project}/environments",
      post(create_handler),
    )
    .with_state(request.clone());
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let address = listener.local_addr().unwrap();
  let task = tokio::spawn(async move {
    axum::serve(listener, router).await.unwrap();
  });
  (request, format!("http://{address}"), task)
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
async fn env_create_splits_a_qualified_target_for_the_request() {
  let (request, url, server) = start_server().await;
  let directory = TempDir::new().unwrap();
  save_session(&directory, &url);

  let output = Command::new(env!("CARGO_BIN_EXE_dopbase"))
    .args([
      "--server",
      &url,
      "--data-dir",
      directory.path().to_str().unwrap(),
      "--json",
      "env",
      "create",
      "prj_01JTEST/staging",
    ])
    .output()
    .unwrap();
  server.abort();

  assert!(output.status.success(), "{output:?}");
  assert_eq!(
    request.project.lock().unwrap().as_deref(),
    Some("prj_01JTEST")
  );
  assert_eq!(
    request.body.lock().unwrap().as_ref(),
    Some(&json!({"name":"staging"}))
  );
}
