use crate::{
  cli::{
    client::{self, ApiClient},
    commands::cache::store::{self, CachedRuntime},
    local_config::ResolvedServer,
  },
  constants::{
    api,
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
  let fetched_at = cached.fetched_at.to_rfc3339_opts(SecondsFormat::Secs, true);
  let age = format_age(Utc::now().signed_duration_since(cached.fetched_at));
  Ok(RuntimeLoad {
    project: cached.project,
    environment: cached.environment,
    environment_id: cached.environment_id,
    entries: cached.entries,
    source: RuntimeSource::Cache {
      fetched_at,
      age,
      reason,
    },
  })
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
