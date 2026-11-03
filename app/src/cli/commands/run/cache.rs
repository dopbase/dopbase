use crate::{
  cli::{
    client::{self, ApiClient},
    commands::cache::store::{self, CachedRuntime},
    local_config::ResolvedServer,
  },
  constants::{
    api,
    errors::TOKEN_SCOPE_INVALID,
    limits::{MAX_SECRET_COLLECTION_BYTES, MAX_SECRETS_PER_ENVIRONMENT},
  },
  models::SecretInput,
};
use anyhow::{Context, Result, bail};
use chrono::{SecondsFormat, Utc};
use reqwest::Method;
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashSet, time::Duration};

const LIVE_TIMEOUT: Duration = Duration::from_secs(5);
/// The cache alias a runner token's environment is stored under, so a run
/// without a reference finds it offline. `@` never starts a reference.
pub const RUNNER_ALIAS: &str = "@runner";

#[derive(Debug)]
pub enum RuntimeSource {
  Live {
    cache_warning: Option<String>,
  },
  Cache {
    fetched_at: String,
    age: String,
    reason: String,
  },
}

impl RuntimeSource {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Live { .. } => "live",
      Self::Cache { .. } => "cache",
    }
  }
}

#[derive(Debug)]
pub struct RuntimeLoad {
  pub project: String,
  pub environment: String,
  pub environment_id: String,
  pub entries: Vec<SecretInput>,
  pub source: RuntimeSource,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeResponse {
  project: String,
  environment: String,
  environment_id: String,
  entries: Vec<SecretInput>,
}

pub async fn load(
  server: &ResolvedServer,
  api: &ApiClient,
  reference: &str,
) -> Result<RuntimeLoad> {
  let live = tokio::time::timeout(LIVE_TIMEOUT, fetch_live(api, reference)).await;
  match live {
    Ok(Ok(runtime)) => {
      let cached = CachedRuntime {
        project: runtime.project.clone(),
        environment: runtime.environment.clone(),
        environment_id: runtime.environment_id.clone(),
        aliases: Vec::new(),
        fetched_at: Utc::now(),
        entries: runtime.entries.clone(),
      };
      let cache_warning = store::save(server, api.credential_token()?, reference, &cached)
        .err()
        .map(|error| {
          format!("live secrets were loaded, but the encrypted cache was not updated: {error}")
        });
      Ok(RuntimeLoad {
        project: runtime.project,
        environment: runtime.environment,
        environment_id: runtime.environment_id,
        entries: runtime.entries,
        source: RuntimeSource::Live { cache_warning },
      })
    }
    Ok(Err(error)) if client::is_availability_error(&error) => {
      load_after_failure(server, api, reference, concise_reason(&error))
    }
    Err(_) => load_after_failure(
      server,
      api,
      reference,
      format!(
        "Dopbase at {} did not complete the runtime fetch within 5 seconds",
        server.url
      ),
    ),
    Ok(Err(error)) => Err(error),
  }
}

async fn fetch_live(
  api: &ApiClient,
  reference: &str,
) -> Result<RuntimeResponse> {
  let environment = api
    .request_runtime(Method::GET, &api::environments::resolve(reference), None)
    .await?;
  let id = environment
    .get("id")
    .and_then(Value::as_str)
    .context("environment response did not contain an id")?;
  if id.is_empty() {
    bail!("environment response contained an empty id");
  }
  let value = api
    .request_runtime(Method::GET, &api::secrets::runtime(id), None)
    .await?;
  let runtime: RuntimeResponse =
    serde_json::from_value(value).context("runtime response was invalid")?;
  if runtime.environment_id != id {
    bail!("runtime response returned a different environment id");
  }
  if runtime.project.is_empty() || runtime.environment.is_empty() {
    bail!("runtime response contained empty environment metadata");
  }
  validate_entries(&runtime.entries)?;
  Ok(runtime)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunnerResponse {
  project_id: String,
  project: String,
  environment: String,
  environment_id: String,
  entries: Vec<SecretInput>,
}

/// Loads the runner token's own environment. `expected` is the reference
/// the caller named, if any; it must be that environment.
pub async fn load_runner(
  server: &ResolvedServer,
  api: &ApiClient,
  expected: Option<&str>,
) -> Result<RuntimeLoad> {
  let live = tokio::time::timeout(LIVE_TIMEOUT, fetch_runner(api, expected)).await;
  // The server refuses a reference to another environment before it reads
  // any value.
  if let (Ok(Err(error)), Some(expected)) = (&live, expected)
    && client::coded_message(error, TOKEN_SCOPE_INVALID).is_some()
  {
    bail!(scope_refused(expected));
  }
  match live {
    Ok(Ok(runtime)) => {
      let aliases = vec![
        runtime.environment_id.clone(),
        format!("{}/{}", runtime.project, runtime.environment),
        format!("{}/{}", runtime.project_id, runtime.environment),
      ];
      let cached = CachedRuntime {
        project: runtime.project.clone(),
        environment: runtime.environment.clone(),
        environment_id: runtime.environment_id.clone(),
        aliases,
        fetched_at: Utc::now(),
        entries: runtime.entries.clone(),
      };
      let cache_warning = store::save(server, api.credential_token()?, RUNNER_ALIAS, &cached)
        .err()
        .map(|error| {
          format!("live secrets were loaded, but the encrypted cache was not updated: {error}")
        });
      Ok(RuntimeLoad {
        project: runtime.project,
        environment: runtime.environment,
        environment_id: runtime.environment_id,
        entries: runtime.entries,
        source: RuntimeSource::Live { cache_warning },
      })
    }
    Ok(Err(error)) if client::is_missing_route(&error) => match expected {
      Some(reference) => load(server, api, reference).await,
      None => bail!(
        "This Dopbase server can't pick the environment from a runner token. Pass ENVIRONMENT_REF or upgrade the server."
      ),
    },
    Ok(Err(error)) if client::is_availability_error(&error) => {
      load_runner_after_failure(server, api, expected, concise_reason(&error))
    }
    Err(_) => load_runner_after_failure(
      server,
      api,
      expected,
      format!(
        "Dopbase at {} did not complete the runtime fetch within 5 seconds",
        server.url
      ),
    ),
    Ok(Err(error)) => Err(error),
  }
}

async fn fetch_runner(
  api: &ApiClient,
  expected: Option<&str>,
) -> Result<RunnerResponse> {
  let value = api
    .request_runtime(Method::GET, &api::runtime::secrets(expected), None)
    .await?;
  let runtime: RunnerResponse =
    serde_json::from_value(value).context("runtime response was invalid")?;
  if runtime.environment_id.is_empty()
    || runtime.project_id.is_empty()
    || runtime.project.is_empty()
    || runtime.environment.is_empty()
  {
    bail!("runtime response contained empty environment metadata");
  }
  validate_entries(&runtime.entries)?;
  Ok(runtime)
}

/// Stops an offline run whose reference names another environment than
/// the cached one, before any child starts. Online, the server checks.
fn ensure_runner_environment(
  expected: Option<&str>,
  aliases: &[String],
) -> Result<()> {
  match expected {
    Some(expected) if !aliases.iter().any(|alias| alias == expected) => {
      bail!(scope_refused(expected))
    }
    _ => Ok(()),
  }
}

/// The refusal names only what the caller asked for, never the token's own
/// environment, since it is printed wherever the run's output goes.
fn scope_refused(expected: &str) -> String {
  format!(
    "The runner token cannot access {expected}. Drop the environment reference to run the token's own environment, or use a token made for {expected}."
  )
}

fn load_runner_after_failure(
  server: &ResolvedServer,
  api: &ApiClient,
  expected: Option<&str>,
  reason: String,
) -> Result<RuntimeLoad> {
  let token = api.credential_token()?;
  // An entry an older CLI saved has no runner alias, only the reference.
  let cached = store::load(server, token, RUNNER_ALIAS)
    .or_else(|error| match expected {
      Some(reference) => store::load(server, token, reference),
      None => Err(error),
    })
    .with_context(|| {
      format!(
        "{reason}. \nEnvironment variables were not injected, and the child was not started because no usable encrypted cache is available for this runner token"
      )
    })?;
  ensure_runner_environment(expected, &cached.aliases)?;
  validate_entries(&cached.entries)?;
  Ok(cached_load(cached, reason))
}

fn validate_entries(entries: &[SecretInput]) -> Result<()> {
  if entries.len() > MAX_SECRETS_PER_ENVIRONMENT {
    bail!("runtime response exceeded the maximum number of environment variables");
  }
  let total_bytes = entries.iter().try_fold(0_usize, |total, entry| {
    total
      .checked_add(entry.key.len())
      .and_then(|total| total.checked_add(entry.value.len()))
      .context("runtime response size overflowed")
  })?;
  if total_bytes > MAX_SECRET_COLLECTION_BYTES {
    bail!("runtime response exceeded the maximum secret collection size");
  }
  let mut keys = HashSet::with_capacity(entries.len());
  for entry in entries {
    if entry.key.is_empty() || entry.key.contains('=') || entry.key.contains('\0') {
      bail!("runtime response contained an invalid environment variable name");
    }
    if entry.value.contains('\0') {
      bail!("runtime response contained an invalid environment variable value");
    }
    if !keys.insert(&entry.key) {
      bail!("runtime response contained a duplicate environment variable name");
    }
  }
  Ok(())
}

fn load_after_failure(
  server: &ResolvedServer,
  api: &ApiClient,
  reference: &str,
  reason: String,
) -> Result<RuntimeLoad> {
  let cached = store::load(server, api.credential_token()?, reference).with_context(|| {
    format!(
      "{reason}. \nEnvironment variables were not injected, and the child was not started because no usable encrypted cache is available for {reference}"
    )
  })?;
  validate_entries(&cached.entries)?;
  Ok(cached_load(cached, reason))
}

fn cached_load(
  cached: CachedRuntime,
  reason: String,
) -> RuntimeLoad {
  let fetched_at = cached.fetched_at.to_rfc3339_opts(SecondsFormat::Secs, true);
  let age = format_age(Utc::now().signed_duration_since(cached.fetched_at));
  RuntimeLoad {
    project: cached.project,
    environment: cached.environment,
    environment_id: cached.environment_id,
    entries: cached.entries,
    source: RuntimeSource::Cache {
      fetched_at,
      age,
      reason,
    },
  }
}

fn concise_reason(error: &anyhow::Error) -> String {
  error
    .to_string()
    .lines()
    .next()
    .unwrap_or("Dopbase is unavailable")
    .to_owned()
}

fn format_age(age: chrono::Duration) -> String {
  let seconds = age.num_seconds().max(0);
  if seconds < 60 {
    format!("{seconds}s")
  } else if seconds < 60 * 60 {
    format!("{}m", seconds / 60)
  } else if seconds < 24 * 60 * 60 {
    format!("{}h", seconds / (60 * 60))
  } else {
    format!("{}d", seconds / (24 * 60 * 60))
  }
}
