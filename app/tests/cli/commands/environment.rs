use app::cli::{
  local_config::{ClientConfig, ResolvedServer, ServerSource},
  session,
};
use axum::{
  Json, Router,
  extract::{Path, Query, State},
  http::StatusCode,
  response::{IntoResponse, Response},
  routing::{get, post},
};
use serde_json::{Value, json};
use std::{
  collections::BTreeMap,
  process::{Command, Output},
  sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
  },
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

const SECRET_MARKER: &str = "clone-secret-value-marker";

#[derive(Clone, Copy, PartialEq, Eq)]
enum CloneScenario {
  Success,
  ZeroSecrets,
  ExistingDestination,
  CreateFailure,
  ExportFailure,
  ImportFailure,
  CleanupFailure,
  FinalListFailure,
  RunnerCredential,
  ServiceAccountCredential,
  StaleSession,
}

#[derive(Clone)]
struct CloneState {
  scenario: CloneScenario,
  requests: Arc<Mutex<Vec<String>>>,
  import_body: Arc<Mutex<Option<Value>>>,
  list_calls: Arc<AtomicUsize>,
}

impl CloneState {
  fn new(scenario: CloneScenario) -> Self {
    Self {
      scenario,
      requests: Arc::new(Mutex::new(Vec::new())),
      import_body: Arc::new(Mutex::new(None)),
      list_calls: Arc::new(AtomicUsize::new(0)),
    }
  }

  fn record(
    &self,
    request: impl Into<String>,
  ) {
    self.requests.lock().unwrap().push(request.into());
  }

  fn recorded(&self) -> Vec<String> {
    self.requests.lock().unwrap().clone()
  }
}

fn clone_response(
  status: StatusCode,
  value: Value,
) -> Response {
  (status, Json(value)).into_response()
}

fn clone_ok(data: Value) -> Response {
  clone_response(StatusCode::OK, json!({"data":data}))
}

fn clone_failure(
  status: StatusCode,
  code: &str,
  message: &str,
) -> Response {
  clone_response(status, json!({"error":{code:message}}))
}

async fn clone_session(State(state): State<CloneState>) -> Response {
  state.record("session");
  match state.scenario {
    CloneScenario::RunnerCredential => clone_failure(
      StatusCode::FORBIDDEN,
      "AUTHORIZATION_DENIED",
      "This operation is not available to runner tokens.",
    ),
    CloneScenario::ServiceAccountCredential => clone_failure(
      StatusCode::FORBIDDEN,
      "AUTHORIZATION_DENIED",
      "This operation requires a human project account.",
    ),
    _ => clone_ok(json!({
      "email":"admin@example.com",
      "recentAuthentication": state.scenario != CloneScenario::StaleSession
    })),
  }
}

async fn clone_resolve(
  State(state): State<CloneState>,
  Query(query): Query<BTreeMap<String, String>>,
) -> Response {
  state.record(format!(
    "resolve:{}",
    query.get("reference").map(String::as_str).unwrap_or("")
  ));
  clone_ok(json!({
    "id":"env_100001",
    "projectId":"prj_01JTEST",
    "projectName":"payment-service",
    "name":"local",
    "createdAt":"2026-09-20T10:00:00Z",
    "updatedAt":"2026-09-20T10:00:00Z"
  }))
}

fn environment(
  id: &str,
  name: &str,
  updated_at: &str,
) -> Value {
  json!({
    "id":id,
    "projectId":"prj_01JTEST",
    "projectName":"payment-service",
    "name":name,
    "createdAt":updated_at,
    "updatedAt":updated_at
  })
}

async fn clone_list(
  State(state): State<CloneState>,
  Query(query): Query<BTreeMap<String, String>>,
) -> Response {
  let call = state.list_calls.fetch_add(1, Ordering::SeqCst);
  state.record(format!(
    "list:{}",
    query.get("project").map(String::as_str).unwrap_or("")
  ));
  if call > 0 && state.scenario == CloneScenario::FinalListFailure {
    return clone_failure(
      StatusCode::INTERNAL_SERVER_ERROR,
      "ENVIRONMENT_LIST_FAILED",
      "The environments could not be listed.",
    );
  }
  let mut environments = vec![
    environment("env_100001", "local", "2026-09-20T10:00:00Z"),
    environment("env_100002", "staging", "2026-09-20T11:00:00Z"),
  ];
  if call > 0 || state.scenario == CloneScenario::ExistingDestination {
    environments.push(environment(
      "env_100003",
      "production",
      "2026-09-21T12:00:00Z",
    ));
  }
  clone_ok(Value::Array(environments))
}

async fn clone_secret_list(
  Path(id): Path<String>,
  State(state): State<CloneState>,
) -> Response {
  state.record(format!("secret-list:{id}"));
  if state.scenario == CloneScenario::ZeroSecrets {
    clone_ok(json!([]))
  } else {
    clone_ok(json!([
      {"key":"API_KEY","version":1},
      {"key":"DATABASE_URL","version":1}
    ]))
  }
}

async fn clone_create(
  Path(project): Path<String>,
  State(state): State<CloneState>,
  Json(body): Json<Value>,
) -> Response {
  state.record(format!("create:{project}:{}", body["name"]));
  if state.scenario == CloneScenario::CreateFailure {
    return clone_failure(
      StatusCode::CONFLICT,
      "ENVIRONMENT_ALREADY_EXISTS",
      "An environment with this name already exists in the project.",
    );
  }
  clone_response(
    StatusCode::CREATED,
    json!({"data":environment(
      "env_100003",
      "production",
      "2026-09-21T12:00:00Z"
    )}),
  )
}

async fn clone_export(
  Path(id): Path<String>,
  State(state): State<CloneState>,
) -> Response {
  state.record(format!("export:{id}"));
  if state.scenario == CloneScenario::ExportFailure {
    return clone_failure(
      StatusCode::INTERNAL_SERVER_ERROR,
      "SECRET_EXPORT_FAILED",
      "The source secrets could not be read.",
    );
  }
  let entries = if state.scenario == CloneScenario::ZeroSecrets {
    json!([])
  } else {
    json!([
      {"key":"API_KEY","value":SECRET_MARKER},
      {"key":"DATABASE_URL","value":"sqlite://private"}
    ])
  };
  clone_ok(json!({"entries":entries}))
}

async fn clone_import(
  Path(id): Path<String>,
  State(state): State<CloneState>,
  Json(body): Json<Value>,
) -> Response {
  state.record(format!("import:{id}"));
  *state.import_body.lock().unwrap() = Some(body);
  if matches!(
    state.scenario,
    CloneScenario::ImportFailure | CloneScenario::CleanupFailure
  ) {
    return clone_failure(
      StatusCode::INTERNAL_SERVER_ERROR,
      "SECRET_IMPORT_FAILED",
      "The source secrets could not be copied.",
    );
  }
  clone_ok(json!({
    "addedKeys":["API_KEY","DATABASE_URL"],
    "updatedKeys":[],
    "unchangedKeys":[],
    "deletedKeys":[],
    "dryRun":false,
    "revision":"revision-1"
  }))
}

async fn clone_delete(
  Path(id): Path<String>,
  State(state): State<CloneState>,
) -> Response {
  state.record(format!("delete:{id}"));
  if state.scenario == CloneScenario::CleanupFailure {
    return clone_failure(
      StatusCode::INTERNAL_SERVER_ERROR,
      "ENVIRONMENT_DELETE_FAILED",
      "The new environment could not be removed.",
    );
  }
  clone_ok(json!({"affected":{"environments":1,"secrets":0,"tokens":0}}))
}

async fn start_clone_server(
  scenario: CloneScenario
) -> (CloneState, String, tokio::task::JoinHandle<()>) {
  let state = CloneState::new(scenario);
  let router = Router::new()
    .route("/api/v1/auth/session", get(clone_session))
    .route("/api/v1/environments/resolve", get(clone_resolve))
    .route("/api/v1/environments", get(clone_list))
    .route(
      "/api/v1/projects/{project}/environments",
      post(clone_create),
    )
    .route("/api/v1/environments/{id}/secrets", get(clone_secret_list))
    .route(
      "/api/v1/environments/{id}/secrets/export",
      post(clone_export),
    )
    .route(
      "/api/v1/environments/{id}/secrets/import",
      post(clone_import),
    )
    .route(
      "/api/v1/environments/{id}",
      axum::routing::delete(clone_delete),
    )
    .with_state(state.clone());
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let address = listener.local_addr().unwrap();
  let task = tokio::spawn(async move {
    axum::serve(listener, router).await.unwrap();
  });
  (state, format!("http://{address}"), task)
}

async fn run_clone(
  scenario: CloneScenario,
  json_output: bool,
  yes: bool,
) -> (Output, CloneState, TempDir) {
  let (state, url, server) = start_clone_server(scenario).await;
  let data_directory = TempDir::new().unwrap();
  let working_directory = TempDir::new().unwrap();
  save_session(&data_directory, &url);
  let mut command = Command::new(env!("CARGO_BIN_EXE_dopbase"));
  command
    .current_dir(working_directory.path())
    .args(["--server", &url, "--data-dir"])
    .arg(data_directory.path())
    .args(["env", "clone", "payment-service/local", "production"]);
  if json_output {
    command.arg("--json");
  }
  if yes {
    command.arg("--yes");
  }
  let output = command.output().unwrap();
  server.abort();
  (output, state, working_directory)
}

fn assert_redacted(output: &Output) {
  assert!(!String::from_utf8_lossy(&output.stdout).contains(SECRET_MARKER));
  assert!(!String::from_utf8_lossy(&output.stderr).contains(SECRET_MARKER));
}

#[tokio::test(flavor = "multi_thread")]
async fn env_clone_copies_secrets_and_returns_the_complete_environment_list_as_json() {
  let (output, state, working_directory) = run_clone(CloneScenario::Success, true, true).await;

  assert!(output.status.success(), "{output:?}");
  let environments: Value = serde_json::from_slice(&output.stdout).unwrap();
  assert_eq!(environments.as_array().unwrap().len(), 3);
  assert_eq!(environments[2]["name"], "production");
  assert_eq!(
    state.import_body.lock().unwrap().as_ref().unwrap(),
    &json!({
      "mode":"merge",
      "dryRun":false,
      "entries":[
        {"key":"API_KEY","value":SECRET_MARKER},
        {"key":"DATABASE_URL","value":"sqlite://private"}
      ]
    })
  );
  assert_eq!(
    state.recorded(),
    [
      "session",
      "resolve:payment-service/local",
      "list:prj_01JTEST",
      "secret-list:env_100001",
      "session",
      "create:prj_01JTEST:\"production\"",
      "export:env_100001",
      "import:env_100003",
      "list:prj_01JTEST",
    ]
  );
  assert_redacted(&output);
  assert_eq!(
    std::fs::read_dir(working_directory.path()).unwrap().count(),
    0
  );
}

#[tokio::test(flavor = "multi_thread")]
async fn env_clone_human_output_lists_every_environment_in_the_project() {
  let (output, _, _) = run_clone(CloneScenario::Success, false, true).await;

  assert!(output.status.success(), "{output:?}");
  let stdout = String::from_utf8_lossy(&output.stdout);
  assert!(stdout.contains("Cloned 2 secrets from payment-service/local"));
  assert!(stdout.contains("ENVIRONMENT"));
  assert!(stdout.contains("local"));
  assert!(stdout.contains("staging"));
  assert!(stdout.contains("production"));
  assert!(stdout.contains("3 environment(s)"));
  assert!(!stdout.contains("\"projectName\""));
  assert_redacted(&output);
}

#[tokio::test(flavor = "multi_thread")]
async fn env_clone_supports_an_empty_source() {
  let (output, state, _) = run_clone(CloneScenario::ZeroSecrets, false, true).await;

  assert!(output.status.success(), "{output:?}");
  assert!(String::from_utf8_lossy(&output.stdout).contains("Cloned 0 secrets"));
  assert_eq!(
    state.import_body.lock().unwrap().as_ref().unwrap()["entries"],
    json!([])
  );
}

#[tokio::test(flavor = "multi_thread")]
async fn env_clone_stops_before_creation_for_conflict_cancellation_and_stale_authentication() {
  for (scenario, yes, expected) in [
    (CloneScenario::ExistingDestination, true, "already exists"),
    (CloneScenario::Success, false, "confirmation is required"),
    (
      CloneScenario::StaleSession,
      true,
      "recent human authentication is required",
    ),
  ] {
    let (output, state, _) = run_clone(scenario, false, yes).await;
    assert!(!output.status.success(), "{output:?}");
    assert!(
      String::from_utf8_lossy(&output.stderr).contains(expected),
      "{output:?}"
    );
    let requests = state.recorded();
    assert!(
      !requests
        .iter()
        .any(|request| request.starts_with("create:"))
    );
    assert!(
      !requests
        .iter()
        .any(|request| request.starts_with("export:"))
    );
  }
}

#[tokio::test(flavor = "multi_thread")]
async fn env_clone_rejects_runner_and_service_account_credentials() {
  for scenario in [
    CloneScenario::RunnerCredential,
    CloneScenario::ServiceAccountCredential,
  ] {
    let (output, state, _) = run_clone(scenario, false, true).await;
    assert!(!output.status.success(), "{output:?}");
    assert_eq!(state.recorded(), ["session"]);
  }
}

#[tokio::test(flavor = "multi_thread")]
async fn env_clone_does_not_fetch_plaintext_when_destination_creation_fails() {
  let (output, state, _) = run_clone(CloneScenario::CreateFailure, false, true).await;

  assert!(!output.status.success(), "{output:?}");
  let requests = state.recorded();
  assert!(
    requests
      .iter()
      .any(|request| request.starts_with("create:"))
  );
  assert!(
    !requests
      .iter()
      .any(|request| request.starts_with("export:"))
  );
  assert!(
    !requests
      .iter()
      .any(|request| request.starts_with("delete:"))
  );
}

#[tokio::test(flavor = "multi_thread")]
async fn env_clone_removes_the_destination_when_export_fails() {
  let (output, state, _) = run_clone(CloneScenario::ExportFailure, false, true).await;

  assert!(!output.status.success(), "{output:?}");
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(stderr.contains("SECRET_EXPORT_FAILED"), "{stderr}");
  assert!(stderr.contains("Cleanup: removed"), "{stderr}");
  assert!(state.recorded().contains(&"delete:env_100003".into()));
  assert_redacted(&output);
}

#[tokio::test(flavor = "multi_thread")]
async fn env_clone_reports_import_and_cleanup_results() {
  let (output, state, _) = run_clone(CloneScenario::ImportFailure, false, true).await;
  assert!(!output.status.success(), "{output:?}");
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(stderr.contains("SECRET_IMPORT_FAILED"), "{stderr}");
  assert!(stderr.contains("Cleanup: removed"), "{stderr}");
  assert!(state.recorded().contains(&"delete:env_100003".into()));
  assert_redacted(&output);

  let (output, state, _) = run_clone(CloneScenario::CleanupFailure, false, true).await;
  assert!(!output.status.success(), "{output:?}");
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(stderr.contains("SECRET_IMPORT_FAILED"), "{stderr}");
  assert!(stderr.contains("Cleanup failed"), "{stderr}");
  assert!(stderr.contains("ENVIRONMENT_DELETE_FAILED"), "{stderr}");
  assert!(state.recorded().contains(&"delete:env_100003".into()));
  assert_redacted(&output);
}

#[tokio::test(flavor = "multi_thread")]
async fn env_clone_keeps_a_completed_clone_when_the_final_list_fails() {
  let (output, state, _) = run_clone(CloneScenario::FinalListFailure, false, true).await;

  assert!(!output.status.success(), "{output:?}");
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(
    stderr.contains("Cloned payment-service/production"),
    "{stderr}"
  );
  assert!(
    stderr.contains("dopbase env list payment-service"),
    "{stderr}"
  );
  assert!(state.recorded().contains(&"import:env_100003".into()));
  assert!(!state.recorded().contains(&"delete:env_100003".into()));
  assert_redacted(&output);
}
