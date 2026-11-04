use app::cli::{
  client::ApiClient,
  local_config::{ClientConfig, ResolvedServer, ServerSource},
  runtime_cache::{self, RuntimeSource},
};
use axum::{
  Json, Router,
  extract::{Query, State},
  http::StatusCode,
  response::IntoResponse,
  routing::get,
};
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

/// Mode 10 is an older server, which refuses a runner's reference to
/// another environment with its old message.
async fn resolve_handler(
  State(mode): State<Arc<AtomicU8>>,
  Query(query): Query<std::collections::HashMap<String, String>>,
) -> axum::response::Response {
  if mode.load(Ordering::SeqCst) == 10
    && query.get("reference").map(String::as_str) != Some("env_01CACHE")
  {
    return (
      StatusCode::FORBIDDEN,
      Json(
        json!({"error":{"TOKEN_SCOPE_INVALID":"The runner token cannot access this environment."}}),
      ),
    )
      .into_response();
  }
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

/// Mode 1 is an outage, mode 10 an older server without the route. Like
/// the server, it refuses a reference to another environment.
async fn runner_handler(
  State(mode): State<Arc<AtomicU8>>,
  Query(query): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
  if let Some(reference) = query.get("reference")
    && mode.load(Ordering::SeqCst) == 0
    && ![
      "env_01CACHE",
      "payment-service/development",
      "prj_01CACHE/development",
    ]
    .contains(&reference.as_str())
  {
    return (
      StatusCode::FORBIDDEN,
      Json(json!({"error":{"TOKEN_SCOPE_INVALID":format!(
        "The runner token cannot access {reference}."
      )}})),
    );
  }
  match mode.load(Ordering::SeqCst) {
    1 => (
      StatusCode::SERVICE_UNAVAILABLE,
      Json(json!({"error":{"SERVER_UNAVAILABLE":"try later"}})),
    ),
    10 => (
      StatusCode::NOT_FOUND,
      Json(json!({"error":{"REQUEST_INVALID":"The requested API route was not found."}})),
    ),
    _ => (
      StatusCode::OK,
      Json(json!({
        "data": {
          "projectId": "prj_01CACHE",
          "project": "payment-service",
          "environment": "development",
          "environmentId": "env_01CACHE",
          "revision": "rev-1",
          "entries": [{"key":"API_TOKEN","value":SECRET}]
        }
      })),
    ),
  }
}

async fn start_server() -> (Arc<AtomicU8>, String, tokio::task::JoinHandle<()>) {
  let mode = Arc::new(AtomicU8::new(0));
  let router = Router::new()
    .route("/api/v1/runtime/secrets", get(runner_handler))
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

#[tokio::test]
async fn a_runner_token_loads_its_own_environment_and_a_reference_only_checks_it() {
  let (_mode, url, task) = start_server().await;
  let directory = TempDir::new().unwrap();
  let server = make_server(&directory, &url);
  let api = ApiClient::new(&server, Some(TOKEN.into())).unwrap();

  let loaded = runtime_cache::load_runner(&server, &api, None)
    .await
    .unwrap();
  assert_eq!(loaded.source.as_str(), "live");
  assert_eq!(
    (loaded.project.as_str(), loaded.environment.as_str()),
    ("payment-service", "development")
  );
  assert_eq!(loaded.entries[0].value, SECRET);
  for reference in [
    "env_01CACHE",
    "payment-service/development",
    "prj_01CACHE/development",
  ] {
    runtime_cache::load_runner(&server, &api, Some(reference))
      .await
      .unwrap_or_else(|error| panic!("{reference}: {error}"));
  }
  let error = runtime_cache::load_runner(&server, &api, Some("payment-service/local"))
    .await
    .unwrap_err()
    .to_string();
  assert!(
    error.contains("The runner token cannot access payment-service/local."),
    "{error}"
  );
  assert!(!error.contains("development"), "{error}");
  task.abort();
}

#[tokio::test]
async fn a_runner_token_falls_back_to_its_cache_and_to_the_old_routes() {
  let (mode, url, task) = start_server().await;
  let directory = TempDir::new().unwrap();
  let server = make_server(&directory, &url);
  let api = ApiClient::new(&server, Some(TOKEN.into())).unwrap();
  runtime_cache::load_runner(&server, &api, None)
    .await
    .unwrap();

  // Offline: the cached entry is found without a reference and still
  // checks one.
  mode.store(1, Ordering::SeqCst);
  let cached = runtime_cache::load_runner(&server, &api, None)
    .await
    .unwrap();
  assert_eq!(cached.source.as_str(), "cache");
  assert_eq!(cached.entries[0].value, SECRET);
  let named = runtime_cache::load_runner(&server, &api, Some("prj_01CACHE/development"))
    .await
    .unwrap();
  assert_eq!(named.source.as_str(), "cache");
  let error = runtime_cache::load_runner(&server, &api, Some("payment-service/local"))
    .await
    .unwrap_err()
    .to_string();
  assert!(
    error.contains("The runner token cannot access payment-service/local."),
    "{error}"
  );
  assert!(!error.contains("development"), "{error}");

  // An older server: a reference takes the resolve + runtime routes.
  mode.store(10, Ordering::SeqCst);
  let legacy = runtime_cache::load_runner(&server, &api, Some("env_01CACHE"))
    .await
    .unwrap();
  assert_eq!(legacy.source.as_str(), "live");
  let error = runtime_cache::load_runner(&server, &api, None)
    .await
    .unwrap_err()
    .to_string();
  assert!(
    error.contains("Pass ENVIRONMENT_REF or upgrade the server"),
    "{error}"
  );
  // The older server's refusal reads like the new one's.
  let error = runtime_cache::load_runner(&server, &api, Some("payment-service/local"))
    .await
    .unwrap_err()
    .to_string();
  assert!(
    error.contains(
      "The runner token cannot access payment-service/local. Drop the environment reference"
    ),
    "{error}"
  );

  // The old-route fetch kept the runner alias, so an offline run without a
  // reference still finds the cache.
  mode.store(1, Ordering::SeqCst);
  let cached = runtime_cache::load_runner(&server, &api, None)
    .await
    .unwrap();
  assert_eq!(cached.source.as_str(), "cache");
  assert_eq!(cached.entries[0].value, SECRET);
  task.abort();
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

#[test]
fn a_runner_reference_is_optional_and_comes_from_the_argument_or_variable() {
  use app::cli::commands::runner_environment;
  assert_eq!(
    runner_environment(None, Err(VarError::NotPresent)).unwrap(),
    None
  );
  assert_eq!(
    runner_environment(Some("env_explicit".into()), Ok("env_variable".into())).unwrap(),
    Some("env_explicit".into())
  );
  assert_eq!(
    runner_environment(None, Ok("env_variable".into())).unwrap(),
    Some("env_variable".into())
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
  // The saved default belongs to people; a runner token runs its own
  // environment without a reference.
  fixture.json(&["env", "create", "fixture/staging"]).await;
  fixture.json(&["env", "default", "fixture/staging"]).await;
  let mut command = fixture.command();
  command
    .env("API_KEY", "incorrect-parent-value")
    .env("INHERITED_TEST_VALUE", "kept");
  command.env("DOPBASE_TOKEN", token).args(["run","--","sh","-c",
    r#"[ "$API_KEY" = runtime-private-marker ] && [ "$INHERITED_TEST_VALUE" = kept ] || exit 41; printf child-ok; exit 23"#]);
  let result = output(command, None).await;
  assert_eq!(result.status.code(), Some(23));
  assert_eq!(result.stdout, b"child-ok");
  let stderr = String::from_utf8_lossy(&result.stderr);
  assert!(!stderr.contains("runtime-private-marker"));
  // CI logs get the count and source, not the token's environment.
  assert!(stderr.contains("source=live"), "{stderr}");
  assert!(!stderr.contains("fixture/"), "{stderr}");
  assert!(!stderr.contains(&id), "{stderr}");
  // A signed-in person still sees where the values came from.
  let mut command = fixture.command();
  command.args(["run", &id, "--", "true"]);
  let result = output(command, None).await;
  assert!(result.status.success());
  let stderr = String::from_utf8_lossy(&result.stderr);
  assert!(
    stderr.contains(&format!("Dopbase: fixture/local ({id})")),
    "{stderr}"
  );
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

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn run_with_a_runner_token_refuses_another_environment_before_the_child_starts() {
  use super::support::{Fixture, failure, output};
  let fixture = Fixture::new().await;
  let id = fixture.environment().await;
  fixture.json(&["env", "create", "fixture/staging"]).await;
  let created = fixture
    .json(&["token", "create", &id, "--name", "runner"])
    .await;
  let token = created["plaintextToken"].as_str().unwrap();
  let marker = fixture.directory.path().join("child-was-started");
  for (argument, variable) in [
    (Some("fixture/staging"), None),
    (None, Some("fixture/staging")),
  ] {
    let mut command = fixture.command();
    command
      .env("DOPBASE_TOKEN", token)
      .env("DOPBASE_CHILD_MARKER", &marker)
      .arg("run");
    if let Some(variable) = variable {
      command.env("DOPBASE_ENV", variable);
    }
    command.args(argument).args([
      "--",
      "sh",
      "-c",
      r#"printf started > "$DOPBASE_CHILD_MARKER""#,
    ]);
    let result = output(command, None).await;
    failure(&result, "The runner token cannot access fixture/staging.");
    assert!(
      !String::from_utf8_lossy(&result.stderr).contains("fixture/local"),
      "the refusal named the token's environment"
    );
    assert!(
      !marker.exists(),
      "run started its child for another environment"
    );
  }

  let mut command = fixture.command();
  command
    .env("DOPBASE_TOKEN", token)
    .args(["run", "fixture/local", "--", "true"]);
  assert!(output(command, None).await.status.success());
}

use app::cli::commands::{SelectionOptions, select};
use app::models::SecretInput;

fn entries(keys: &[&str]) -> Vec<SecretInput> {
  keys
    .iter()
    .map(|key| SecretInput {
      key: (*key).into(),
      value: format!("{key}-value-marker"),
    })
    .collect()
}

fn injected(selection: &app::cli::commands::Selection) -> Vec<&str> {
  selection
    .inject
    .iter()
    .map(|entry| entry.key.as_str())
    .collect()
}

#[test]
fn select_keeps_every_ordinary_key_without_options() {
  let selection = select(
    entries(&["API_KEY", "DATABASE_URL"]),
    &SelectionOptions::default(),
    |_| true,
  )
  .unwrap();
  assert_eq!(injected(&selection), ["API_KEY", "DATABASE_URL"]);
  assert!(selection.skipped.is_empty() && selection.preserved.is_empty());
}

#[test]
fn select_only_filters_and_stops_on_missing_keys_unless_allowed() {
  let options = SelectionOptions {
    only: vec!["API_KEY".into(), "OPTIONAL_KEY".into(), "ZETA_KEY".into()],
    ..Default::default()
  };
  let error = select(entries(&["API_KEY", "DATABASE_URL"]), &options, |_| false)
    .unwrap_err()
    .to_string();
  assert!(error.contains("OPTIONAL_KEY, ZETA_KEY"), "{error}");
  assert!(error.contains("--allow-missing"), "{error}");

  let options = SelectionOptions {
    allow_missing: true,
    ..options
  };
  let selection = select(entries(&["API_KEY", "DATABASE_URL"]), &options, |_| false).unwrap();
  assert_eq!(injected(&selection), ["API_KEY"]);
  assert_eq!(selection.missing, ["OPTIONAL_KEY", "ZETA_KEY"]);
}

#[test]
fn select_skips_reserved_names_even_when_requested() {
  let stored = entries(&[
    "DOPBASE_TOKEN",
    "DOPBASE_URL",
    "DOPBASE_ENV",
    "DOPBASE_DATA_DIR",
    "DOPBASE_SECRETS_PATH",
    "PATH",
    "HOME",
    "PS2",
    "XDG_CONFIG_HOME",
    "LC_ALL",
    "DOPBASE_PORT",
    "SHELL",
    "API_KEY",
  ]);
  let selection = select(stored.clone(), &SelectionOptions::default(), |_| false).unwrap();
  assert_eq!(injected(&selection), ["DOPBASE_PORT", "SHELL", "API_KEY"]);
  assert_eq!(selection.skipped.len(), 10);
  assert!(
    selection
      .skipped
      .iter()
      .all(|name| !name.contains("value-marker"))
  );

  let options = SelectionOptions {
    only: vec!["DOPBASE_TOKEN".into(), "PATH".into()],
    ..Default::default()
  };
  let selection = select(stored, &options, |_| false).unwrap();
  assert!(selection.inject.is_empty());
  assert!(selection.missing.is_empty());
  assert_eq!(selection.skipped, ["DOPBASE_TOKEN", "PATH"]);
}

#[test]
fn select_preserves_only_named_keys_the_parent_defines() {
  let options = SelectionOptions {
    preserve_env: vec!["PORT".into(), "LOG_LEVEL".into(), "UNSTORED".into()],
    ..Default::default()
  };
  let selection = select(
    entries(&["PORT", "LOG_LEVEL", "API_KEY"]),
    &options,
    |key| key == "PORT" || key == "API_KEY",
  )
  .unwrap();
  assert_eq!(injected(&selection), ["LOG_LEVEL", "API_KEY"]);
  assert_eq!(selection.preserved, ["PORT"]);
}

#[test]
fn select_blocks_code_loading_names_unless_only_names_them() {
  let stored = entries(&["NODE_OPTIONS", "LD_PRELOAD", "HOSTNAME", "API_KEY"]);
  let selection = select(stored.clone(), &SelectionOptions::default(), |_| false).unwrap();
  assert_eq!(injected(&selection), ["HOSTNAME", "API_KEY"]);
  assert_eq!(selection.blocked, ["NODE_OPTIONS", "LD_PRELOAD"]);
  assert_eq!(selection.dangerous, ["HOSTNAME"]);

  let options = SelectionOptions {
    only: vec!["API_KEY".into(), "NODE_OPTIONS".into()],
    ..Default::default()
  };
  let selection = select(stored.clone(), &options, |_| false).unwrap();
  assert_eq!(injected(&selection), ["NODE_OPTIONS", "API_KEY"]);
  assert!(selection.blocked.is_empty());
  assert_eq!(selection.dangerous, ["NODE_OPTIONS"]);

  let options = SelectionOptions {
    only: vec!["API_KEY".into(), "LD_PRELOAD".into()],
    preserve_env: vec!["LD_PRELOAD".into()],
    ..Default::default()
  };
  let selection = select(stored, &options, |_| true).unwrap();
  assert_eq!(injected(&selection), ["API_KEY"]);
  assert_eq!(selection.preserved, ["LD_PRELOAD"]);
  assert!(selection.dangerous.is_empty() && selection.blocked.is_empty());
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn run_filters_secrets_before_the_child_starts() {
  use super::support::{Fixture, failure, output};
  let fixture = Fixture::new().await;
  let id = fixture.environment().await;
  for (key, value) in [
    ("API_KEY", "api-private-marker"),
    ("DATABASE_URL", "database-private-marker"),
    ("OTHER_KEY", "other-private-marker"),
    ("PORT", "stored-port-marker"),
    ("DOPBASE_URL", "stored-url-marker"),
    ("PATH", "stored-path-marker"),
    ("NODE_OPTIONS", "--stored-node-marker"),
  ] {
    fixture
      .request(
        reqwest::Method::PUT,
        &format!("/api/v1/environments/{id}/secrets/{key}"),
        Some(json!({ "value": value })),
      )
      .await;
  }
  let created = fixture
    .json(&["token", "create", &id, "--name", "runner"])
    .await;
  let token = created["plaintextToken"].as_str().unwrap();
  let assert_private = |stderr: &[u8]| {
    let stderr = String::from_utf8_lossy(stderr);
    assert!(!stderr.contains("private-marker"), "{stderr}");
    assert!(!stderr.contains("stored-"), "{stderr}");
  };

  // Repeated and comma-separated --only select exactly those keys.
  let mut command = fixture.command();
  command.env("DOPBASE_TOKEN", token).args([
    "run", &id, "--only", "API_KEY", "--only", "DATABASE_URL,NODE_OPTIONS", "--", "sh", "-c",
    r#"[ "$API_KEY" = api-private-marker ] && [ -n "$DATABASE_URL" ] && [ -z "${OTHER_KEY+x}" ] && [ -z "${PORT+x}" ] || exit 41; printf child-ok; exit 23"#,
  ]);
  let result = output(command, None).await;
  assert_eq!(result.status.code(), Some(23));
  assert_eq!(result.stdout, b"child-ok");
  let stderr = String::from_utf8_lossy(&result.stderr);
  assert!(stderr.contains("3 of 7 key(s)"), "{stderr}");
  assert!(
    stderr.contains("can change how programs load or run: NODE_OPTIONS"),
    "{stderr}"
  );
  assert_private(&result.stderr);

  // A missing --only key stops before the child starts.
  let marker = fixture.directory.path().join("child-was-started");
  let mut command = fixture.command();
  command
    .env("DOPBASE_TOKEN", token)
    .env("DOPBASE_CHILD_MARKER", &marker)
    .args([
      "run",
      &id,
      "--only",
      "API_KEY,OPTIONAL_KEY",
      "--",
      "sh",
      "-c",
      r#"printf started > "$DOPBASE_CHILD_MARKER""#,
    ]);
  let result = output(command, None).await;
  failure(
    &result,
    "--only key(s) not stored in this environment: OPTIONAL_KEY",
  );
  assert!(!marker.exists(), "run started its child with a missing key");

  // --allow-missing warns and runs.
  let mut command = fixture.command();
  command.env("DOPBASE_TOKEN", token).args([
    "run",
    &id,
    "--only",
    "API_KEY,OPTIONAL_KEY",
    "--allow-missing",
    "--",
    "sh",
    "-c",
    r#"[ -n "$API_KEY" ] && [ -z "${OPTIONAL_KEY+x}" ] || exit 41"#,
  ]);
  let result = output(command, None).await;
  assert_eq!(result.status.code(), Some(0));
  assert!(
    String::from_utf8_lossy(&result.stderr).contains("--only key(s) not stored: OPTIONAL_KEY")
  );

  // Preserved keys keep the parent value; reserved names never arrive.
  let mut command = fixture.command();
  command
    .env("DOPBASE_TOKEN", token)
    .env("PORT", "parent-port")
    .env_remove("NODE_OPTIONS")
    .args([
      "run", &id, "--preserve-env", "PORT", "--", "sh", "-c",
      r#"[ "$PORT" = parent-port ] && [ "$PATH" != stored-path-marker ] && [ -z "${DOPBASE_URL+x}" ] && [ -z "${DOPBASE_TOKEN+x}" ] && [ -z "${NODE_OPTIONS+x}" ] && [ -n "$OTHER_KEY" ] || exit 41"#,
    ]);
  let result = output(command, None).await;
  assert_eq!(
    result.status.code(),
    Some(0),
    "{}",
    String::from_utf8_lossy(&result.stderr)
  );
  let stderr = String::from_utf8_lossy(&result.stderr);
  assert!(
    stderr.contains("skipped reserved name(s): DOPBASE_URL, PATH"),
    "{stderr}"
  );
  assert!(stderr.contains("kept existing value for: PORT"), "{stderr}");
  assert!(
    stderr.contains("can load code into the child: NODE_OPTIONS"),
    "{stderr}"
  );
  assert_private(&result.stderr);

  // Signals that end the child still reach the caller.
  let mut command = fixture.command();
  command
    .env("DOPBASE_TOKEN", token)
    .args(["run", &id, "--", "sh", "-c", "kill -TERM $$"]);
  let result = output(command, None).await;
  use std::os::unix::process::ExitStatusExt;
  assert_eq!(result.status.signal(), Some(15));
}
