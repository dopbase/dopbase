use app::cli::{
  client::ApiClient,
  local_config::{ClientConfig, ResolvedServer, ServerSource},
  runtime_cache::{self, RuntimeSource},
};
use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::{
  Arc,
  atomic::{AtomicU8, Ordering},
};
use std::{fs, path::PathBuf};
use tempfile::TempDir;

const TOKEN: &str = "dbt_cache_credential_marker";
const SECRET: &str = "cached-secret-marker";

fn make_server(
  directory: &TempDir,
  url: &str,
) -> ResolvedServer {
  ResolvedServer {
    url: url.into(),
    source: ServerSource::Argument,
    config_path: directory.path().join("config.toml"),
    config: ClientConfig {
      version: 1,
      server_url: None,
      default_environment: None,
    },
  }
}

fn cache_paths(server: &ResolvedServer) -> (PathBuf, PathBuf, PathBuf) {
  let root = server.config_path.parent().unwrap();
  let directory = root.join("run-cache");
  let name = format!("{:x}", Sha256::digest(server.url.as_bytes()));
  (
    root.join("run-cache-key"),
    directory.join(&name),
    directory.join(format!("{name}.lock")),
  )
}

async fn resolve_handler(State(mode): State<Arc<AtomicU8>>) -> axum::response::Response {
  match mode.load(Ordering::SeqCst) {
    7 => {
      return (
        StatusCode::SERVICE_UNAVAILABLE,
        "<html>proxy-error-marker</html>",
      )
        .into_response();
    }
    8 => return (StatusCode::UNAUTHORIZED, "proxy-error-marker").into_response(),
    9 => return (StatusCode::OK, "<html>proxy-error-marker</html>").into_response(),
    1 => (
      StatusCode::SERVICE_UNAVAILABLE,
      Json(json!({"error":{"SERVER_UNAVAILABLE":"try later"}})),
    ),
    2 => (
      StatusCode::UNAUTHORIZED,
      Json(json!({"error":{"AUTHENTICATION_REQUIRED":"sign in"}})),
    ),
    4 => (
      StatusCode::NOT_FOUND,
      Json(json!({"error":{"ENVIRONMENT_NOT_FOUND":"missing"}})),
    ),
    _ => (StatusCode::OK, Json(json!({"data":{"id":"env_01CACHE"}}))),
  }
  .into_response()
}

async fn runtime_handler(State(mode): State<Arc<AtomicU8>>) -> impl IntoResponse {
  if mode.load(Ordering::SeqCst) == 3 {
    return (StatusCode::OK, Json(json!({"data":{"invalid":true}})));
  }
  if mode.load(Ordering::SeqCst) == 5 {
    return (
      StatusCode::OK,
      Json(json!({
        "data": {
          "project": "payment-service",
          "environment": "development",
          "environmentId": "env_01CACHE",
          "entries": [{"key":"","value":"invalid"}]
        }
      })),
    );
  }
  if mode.load(Ordering::SeqCst) == 6 {
    let oversized = "x".repeat(4 * 1024 * 1024 + 1);
    return (StatusCode::OK, Json(json!({"data":{"padding":oversized}})));
  }
  (
    StatusCode::OK,
    Json(json!({
      "data": {
        "project": "payment-service",
        "environment": "development",
        "environmentId": "env_01CACHE",
        "entries": [{"key":"API_TOKEN","value":SECRET}]
      }
    })),
  )
}

async fn start_server() -> (Arc<AtomicU8>, String, tokio::task::JoinHandle<()>) {
  let mode = Arc::new(AtomicU8::new(0));
  let router = Router::new()
    .route("/api/v1/environments/resolve", get(resolve_handler))
    .route(
      "/api/v1/environments/{id}/secrets/runtime",
      get(runtime_handler),
    )
    .with_state(mode.clone());
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let address = listener.local_addr().unwrap();
  let task = tokio::spawn(async move {
    axum::serve(listener, router).await.unwrap();
  });
  (mode, format!("http://{address}"), task)
}

#[tokio::test]
async fn live_fetch_refreshes_encrypted_cache_and_falls_back_only_on_availability_errors() {
  let (mode, url, task) = start_server().await;
  let directory = TempDir::new().unwrap();
  let server = make_server(&directory, &url);
  let api = ApiClient::new(&server, Some(TOKEN.into())).unwrap();

  let live = runtime_cache::load(&server, &api, "payment-service/development")
    .await
    .unwrap();
  assert!(matches!(live.source, RuntimeSource::Live { .. }));

  let (key_path, cache_path, lock_path) = cache_paths(&server);
  let stored = fs::read(&cache_path).unwrap();
  for marker in [
    SECRET,
    "API_TOKEN",
    "payment-service",
    "development",
    &url,
    TOKEN,
  ] {
    assert!(
      !stored
        .windows(marker.len())
        .any(|window| window == marker.as_bytes())
    );
  }

  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(
      fs::metadata(directory.path()).unwrap().permissions().mode() & 0o777,
      0o700
    );
    assert_eq!(
      fs::metadata(cache_path.parent().unwrap())
        .unwrap()
        .permissions()
        .mode()
        & 0o777,
      0o700
    );
    for path in [&key_path, &cache_path, &lock_path] {
      assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600
      );
    }
    for path in [&key_path, &cache_path, &lock_path] {
      fs::set_permissions(path, std::fs::Permissions::from_mode(0o644)).unwrap();
    }
  }

  mode.store(1, Ordering::SeqCst);
  let cached = runtime_cache::load(&server, &api, "payment-service/development")
    .await
    .unwrap();
  assert!(matches!(cached.source, RuntimeSource::Cache { .. }));
  assert_eq!(cached.entries[0].value, SECRET);

  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    for path in [&key_path, &cache_path, &lock_path] {
      assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600
      );
    }
  }

  mode.store(7, Ordering::SeqCst);
  let cached_from_proxy_error = runtime_cache::load(&server, &api, "payment-service/development")
    .await
    .unwrap();
  assert!(matches!(
    cached_from_proxy_error.source,
    RuntimeSource::Cache { .. }
  ));

  for (response_mode, expected_error) in [
    (8, "server returned 401 Unauthorized (non-JSON response)"),
    (9, "server returned an invalid JSON response"),
  ] {
    mode.store(response_mode, Ordering::SeqCst);
    let error = runtime_cache::load(&server, &api, "payment-service/development")
      .await
      .unwrap_err();
    let message = format!("{error:#}");
    assert!(message.contains(expected_error), "{message}");
    assert!(!message.contains("proxy-error-marker"));
  }

  mode.store(2, Ordering::SeqCst);
  let unauthorized = runtime_cache::load(&server, &api, "payment-service/development")
    .await
    .unwrap_err()
    .to_string();
  assert!(unauthorized.contains("AUTHENTICATION_REQUIRED"));

  mode.store(4, Ordering::SeqCst);
  let not_found = runtime_cache::load(&server, &api, "payment-service/development")
    .await
    .unwrap_err()
    .to_string();
  assert!(not_found.contains("ENVIRONMENT_NOT_FOUND"));

  mode.store(3, Ordering::SeqCst);
  let invalid = runtime_cache::load(&server, &api, "payment-service/development")
    .await
    .unwrap_err()
    .to_string();
  assert!(invalid.contains("runtime response was invalid"));

  mode.store(5, Ordering::SeqCst);
  let invalid_entry = runtime_cache::load(&server, &api, "payment-service/development")
    .await
    .unwrap_err()
    .to_string();
  assert!(invalid_entry.contains("invalid environment variable name"));

  mode.store(6, Ordering::SeqCst);
  let oversized = runtime_cache::load(&server, &api, "payment-service/development")
    .await
    .unwrap_err()
    .to_string();
  assert!(oversized.contains("maximum allowed size"));

  mode.store(0, Ordering::SeqCst);
  let original_key = fs::read(&key_path).unwrap();
  fs::write(&key_path, b"invalid").unwrap();
  let live_with_warning = runtime_cache::load(&server, &api, "payment-service/development")
    .await
    .unwrap();
  assert!(matches!(
    live_with_warning.source,
    RuntimeSource::Live {
      cache_warning: Some(_)
    }
  ));

  fs::write(&key_path, original_key).unwrap();
  task.abort();
  let offline = runtime_cache::load(&server, &api, "payment-service/development")
    .await
    .unwrap();
  assert!(matches!(offline.source, RuntimeSource::Cache { .. }));

  let mut tampered = fs::read(&cache_path).unwrap();
  let last = tampered.len() - 1;
  tampered[last] ^= 1;
  fs::write(&cache_path, tampered).unwrap();
  assert!(
    runtime_cache::load(&server, &api, "payment-service/development")
      .await
      .is_err()
  );

  let wrong_api = ApiClient::new(&server, Some("different-token".into())).unwrap();
  assert!(
    runtime_cache::load(&server, &wrong_api, "payment-service/development")
      .await
      .is_err()
  );
}

#[tokio::test]
async fn unavailable_server_without_cache_does_not_provide_runtime_values() {
  let (mode, url, task) = start_server().await;
  let directory = TempDir::new().unwrap();
  let server = make_server(&directory, &url);
  let api = ApiClient::new(&server, Some(TOKEN.into())).unwrap();
  mode.store(1, Ordering::SeqCst);
  task.abort();

  let error = runtime_cache::load(&server, &api, "payment-service/development")
    .await
    .unwrap_err()
    .to_string();
  assert!(error.contains("Environment variables were not injected"));
  assert!(error.contains("child was not started"));
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn interactive_child_reads_from_the_terminal_and_returns_its_exit_status() {
  use super::support::{command, terminal::Terminal};
  let (_mode, url, task) = start_server().await;
  let directory = TempDir::new().unwrap();
  let mut command = command(directory.path());
  command
    .env("DOPBASE_TOKEN", TOKEN)
    .env("EXPECTED_SECRET", SECRET);
  command.args(["--server", &url, "run", "env_01CACHE", "--", "sh", "-c",
    r#"printf child-ready; IFS= read -r line; if [ "$line" = ping ] && [ "$API_TOKEN" = "$EXPECTED_SECRET" ]; then exit 23; else exit 42; fi"#]);
  let mut terminal = Terminal::new(command);
  terminal.reply("child-ready", "ping\n");
  assert_eq!(terminal.finish().code(), Some(23));
  assert!(!terminal.transcript.contains(SECRET));
  task.abort();
}

use app::cli::commands::run_environment;
use std::env::VarError;
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
    message.contains("dopbase env default <ENVIRONMENT_REF>"),
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

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn run_injects_secrets_preserves_child_status_and_never_launches_with_a_revoked_token() {
  use super::support::{Fixture, failure, output};
  let fixture = Fixture::new().await;
  let id = fixture.environment().await;
  fixture
    .request(
      reqwest::Method::PUT,
      &format!("/api/v1/environments/{id}/secrets/API_KEY"),
      Some(json!({"value":"runtime-private-marker"})),
    )
    .await;
  let created = fixture
    .json(&["token", "create", &id, "--name", "runner"])
    .await;
  let token = created["plaintextToken"].as_str().unwrap();
  fixture.json(&["env", "default", &id]).await;
  let mut command = fixture.command();
  command
    .env("API_KEY", "incorrect-parent-value")
    .env("INHERITED_TEST_VALUE", "kept");
  command.env("DOPBASE_TOKEN", token).args(["run","--","sh","-c",
    r#"[ "$API_KEY" = runtime-private-marker ] && [ "$INHERITED_TEST_VALUE" = kept ] || exit 41; printf child-ok; exit 23"#]);
  let result = output(command, None).await;
  assert_eq!(result.status.code(), Some(23));
  assert_eq!(result.stdout, b"child-ok");
  assert!(!String::from_utf8_lossy(&result.stderr).contains("runtime-private-marker"));
  fixture
    .json(&["token", "revoke", created["token"]["id"].as_str().unwrap()])
    .await;
  let marker = fixture.directory.path().join("child-was-started");
  let mut command = fixture.command();
  command
    .env("DOPBASE_TOKEN", token)
    .env("DOPBASE_CHILD_MARKER", &marker)
    .args([
      "run",
      &id,
      "--",
      "sh",
      "-c",
      r#"printf started > "$DOPBASE_CHILD_MARKER""#,
    ]);
  let result = output(command, None).await;
  failure(&result, "Dopbase authentication failed");
  assert!(
    !marker.exists(),
    "run started its child after authentication failed"
  );
}
