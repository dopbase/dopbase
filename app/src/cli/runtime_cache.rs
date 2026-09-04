use super::{
  client::{self, ApiClient},
  local_config::ResolvedServer,
};
use crate::constants::limits::{MAX_SECRET_COLLECTION_BYTES, MAX_SECRETS_PER_ENVIRONMENT};
use crate::{config::ensure_data_dir, models::SecretInput, utils::private_file};
use anyhow::{Context, Result, bail};
use chacha20poly1305::{
  KeyInit, XChaCha20Poly1305, XNonce,
  aead::{Aead, Payload},
};
use chrono::{DateTime, SecondsFormat, Utc};
use fs2::FileExt;
use hkdf::Hkdf;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
  collections::{BTreeMap, HashSet},
  fs::{self, OpenOptions},
  path::{Path, PathBuf},
  time::Duration,
};
use zeroize::Zeroizing;

const CACHE_DIRECTORY: &str = "run-cache";
const CACHE_KEY_FILENAME: &str = "run-cache-key";
const MAGIC: &[u8; 8] = b"DOPRUN\0\0";
const VERSION: u8 = 1;
const NONCE_LENGTH: usize = 24;
const KEY_LENGTH: usize = 32;
const HEADER_LENGTH: usize = MAGIC.len() + 1 + NONCE_LENGTH;
const AAD: &[u8] = b"dopbase-run-cache-v1";
const KDF_INFO: &[u8] = b"dopbase-run-cache-key-v1";
const LIVE_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_CACHED_ENVIRONMENTS: usize = 16;
const MAX_CACHE_DOCUMENT_BYTES: u64 = 32 * 1024 * 1024;

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

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CachedRuntime {
  project: String,
  environment: String,
  environment_id: String,
  aliases: Vec<String>,
  fetched_at: DateTime<Utc>,
  entries: Vec<SecretInput>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CacheDocument {
  version: u8,
  server_url: String,
  environments: BTreeMap<String, CachedRuntime>,
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
      let cache_warning = save(server, api.credential_token()?, reference, &runtime)
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
    .request_runtime(
      Method::GET,
      &format!(
        "/api/v1/environments/resolve?reference={}",
        client::encode_query(reference)
      ),
      None,
    )
    .await?;
  let id = environment
    .get("id")
    .and_then(Value::as_str)
    .context("environment response did not contain an id")?;
  if id.is_empty() {
    bail!("environment response contained an empty id");
  }
  let value = api
    .request_runtime(
      Method::GET,
      &format!("/api/v1/environments/{id}/secrets/runtime"),
      None,
    )
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
  let cached = load_cached(server, api.credential_token()?, reference).with_context(|| {
    format!(
      "{reason}; environment variables were not injected and the child was not started because no usable encrypted cache is available for {reference}"
    )
  })?;
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

fn save(
  server: &ResolvedServer,
  credential: &str,
  reference: &str,
  runtime: &RuntimeResponse,
) -> Result<()> {
  let paths = paths(server)?;
  secure_directory(
    paths
      .key
      .parent()
      .context("run cache key path has no parent directory")?,
  )?;
  secure_directory(&paths.directory)?;
  let lock = open_lock(&paths.lock)?;
  lock.lock_exclusive().context("failed to lock run cache")?;
  let local_key = load_or_create_key(&paths.key)?;
  let derived_key = derive_key(&local_key, credential, &server.url)?;
  let mut document = match read_document(&paths.cache, &derived_key[..]) {
    Ok(document) if document.server_url == server.url && document.version == VERSION => document,
    Ok(_) | Err(_) => CacheDocument {
      version: VERSION,
      server_url: server.url.clone(),
      environments: BTreeMap::new(),
    },
  };
  let canonical = format!("{}/{}", runtime.project, runtime.environment);
  let mut aliases = vec![
    runtime.environment_id.clone(),
    canonical,
    reference.to_owned(),
  ];
  aliases.sort();
  aliases.dedup();
  document.environments.insert(
    runtime.environment_id.clone(),
    CachedRuntime {
      project: runtime.project.clone(),
      environment: runtime.environment.clone(),
      environment_id: runtime.environment_id.clone(),
      aliases,
      fetched_at: Utc::now(),
      entries: runtime.entries.clone(),
    },
  );
  while document.environments.len() > MAX_CACHED_ENVIRONMENTS {
    let oldest = document
      .environments
      .iter()
      .min_by_key(|(_, cached)| cached.fetched_at)
      .map(|(id, _)| id.clone())
      .context("run cache contained no environment records")?;
    document.environments.remove(&oldest);
  }
  let clear = Zeroizing::new(serde_json::to_vec(&document)?);
  if clear.len() as u64 > MAX_CACHE_DOCUMENT_BYTES {
    bail!("encrypted run cache exceeded the maximum allowed size");
  }
  let envelope = encrypt(&derived_key[..], &clear)?;
  private_file::write(&paths.cache, &envelope, true)
}

fn load_cached(
  server: &ResolvedServer,
  credential: &str,
  reference: &str,
) -> Result<CachedRuntime> {
  let paths = paths(server)?;
  let lock = open_lock(&paths.lock)?;
  lock.lock_shared().context("failed to lock run cache")?;
  if !paths.cache.exists() {
    bail!("encrypted run cache has not been created");
  }
  let local_key = read_key(&paths.key).context("encrypted run cache key is unavailable")?;
  let derived_key = derive_key(&local_key, credential, &server.url)?;
  let document = read_document(&paths.cache, &derived_key[..])
    .context("encrypted run cache could not be unlocked by the current credential or is damaged")?;
  if document.version != VERSION || document.server_url != server.url {
    bail!("encrypted run cache has an invalid scope or unsupported version");
  }
  let cached = document
    .environments
    .into_values()
    .filter(|cached| cached.aliases.iter().any(|alias| alias == reference))
    .max_by_key(|cached| cached.fetched_at)
    .with_context(|| format!("encrypted run cache has no entry matching {reference}"))?;
  validate_entries(&cached.entries)?;
  Ok(cached)
}

struct CachePaths {
  directory: PathBuf,
  key: PathBuf,
  cache: PathBuf,
  lock: PathBuf,
}

fn paths(server: &ResolvedServer) -> Result<CachePaths> {
  let data_directory = server
    .config_path
    .parent()
    .context("client configuration path has no parent directory")?;
  let directory = data_directory.join(CACHE_DIRECTORY);
  let name = format!("{:x}", Sha256::digest(server.url.as_bytes()));
  Ok(CachePaths {
    key: data_directory.join(CACHE_KEY_FILENAME),
    cache: directory.join(&name),
    lock: directory.join(format!("{name}.lock")),
    directory,
  })
}

fn secure_directory(path: &Path) -> Result<()> {
  ensure_data_dir(path)?;
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
      .with_context(|| format!("failed to secure run cache directory {}", path.display()))?;
  }
  Ok(())
}

fn open_lock(path: &Path) -> Result<fs::File> {
  if let Some(parent) = path.parent() {
    secure_directory(parent)?;
  }
  let mut options = OpenOptions::new();
  options.read(true).write(true).create(true);
  #[cfg(unix)]
  {
    use std::os::unix::fs::OpenOptionsExt;
    options.mode(0o600);
  }
  let file = options
    .open(path)
    .with_context(|| format!("failed to open run cache lock at {}", path.display()))?;
  secure_file(path, "run cache lock")?;
  Ok(file)
}

fn load_or_create_key(path: &Path) -> Result<Zeroizing<Vec<u8>>> {
  if path.exists() {
    return read_key(path);
  }
  let mut generated = Zeroizing::new(vec![0_u8; KEY_LENGTH]);
  getrandom::fill(&mut generated)?;
  if let Err(error) = private_file::write(path, &generated, false) {
    if path.exists() {
      return read_key(path);
    }
    return Err(error);
  }
  Ok(generated)
}

fn read_key(path: &Path) -> Result<Zeroizing<Vec<u8>>> {
  secure_file(path, "run cache key")?;
  let key =
    Zeroizing::new(fs::read(path).with_context(|| format!("failed to read {}", path.display()))?);
  if key.len() != KEY_LENGTH {
    bail!("run cache key must contain exactly {KEY_LENGTH} bytes");
  }
  Ok(key)
}

fn derive_key(
  local_key: &[u8],
  credential: &str,
  server_url: &str,
) -> Result<Zeroizing<[u8; KEY_LENGTH]>> {
  let hkdf = Hkdf::<Sha256>::new(Some(local_key), credential.as_bytes());
  let mut info = Vec::with_capacity(KDF_INFO.len() + server_url.len());
  info.extend_from_slice(KDF_INFO);
  info.extend_from_slice(server_url.as_bytes());
  let mut key = Zeroizing::new([0_u8; KEY_LENGTH]);
  hkdf
    .expand(&info, key.as_mut())
    .map_err(|_| anyhow::anyhow!("failed to derive run cache key"))?;
  Ok(key)
}

fn encrypt(
  key: &[u8],
  clear: &[u8],
) -> Result<Vec<u8>> {
  let mut nonce = [0_u8; NONCE_LENGTH];
  getrandom::fill(&mut nonce)?;
  let cipher = XChaCha20Poly1305::new_from_slice(key).expect("validated 32-byte run cache key");
  let ciphertext = cipher
    .encrypt(
      XNonce::from_slice(&nonce),
      Payload {
        msg: clear,
        aad: AAD,
      },
    )
    .map_err(|_| anyhow::anyhow!("failed to encrypt run cache"))?;
  let mut envelope = Vec::with_capacity(HEADER_LENGTH + ciphertext.len());
  envelope.extend_from_slice(MAGIC);
  envelope.push(VERSION);
  envelope.extend_from_slice(&nonce);
  envelope.extend_from_slice(&ciphertext);
  Ok(envelope)
}

fn read_document(
  path: &Path,
  key: &[u8],
) -> Result<CacheDocument> {
  secure_file(path, "run cache")?;
  let size = fs::metadata(path)
    .with_context(|| {
      format!(
        "failed to inspect encrypted run cache at {}",
        path.display()
      )
    })?
    .len();
  if size > MAX_CACHE_DOCUMENT_BYTES {
    bail!("encrypted run cache exceeded the maximum allowed size");
  }
  let stored = fs::read(path)
    .with_context(|| format!("failed to read encrypted run cache at {}", path.display()))?;
  if stored.len() <= HEADER_LENGTH || &stored[..MAGIC.len()] != MAGIC {
    bail!("encrypted run cache has an invalid format");
  }
  if stored[MAGIC.len()] != VERSION {
    bail!("encrypted run cache uses an unsupported version");
  }
  let nonce_start = MAGIC.len() + 1;
  let nonce_end = nonce_start + NONCE_LENGTH;
  let cipher = XChaCha20Poly1305::new_from_slice(key).expect("validated 32-byte run cache key");
  let clear = cipher
    .decrypt(
      XNonce::from_slice(&stored[nonce_start..nonce_end]),
      Payload {
        msg: &stored[nonce_end..],
        aad: AAD,
      },
    )
    .map(Zeroizing::new)
    .map_err(|_| anyhow::anyhow!("encrypted run cache authentication failed"))?;
  serde_json::from_slice(&clear).context("encrypted run cache payload is invalid")
}

fn secure_file(
  path: &Path,
  label: &str,
) -> Result<()> {
  let metadata = fs::symlink_metadata(path)
    .with_context(|| format!("failed to inspect {label} at {}", path.display()))?;
  if !metadata.file_type().is_file() {
    bail!("{label} at {} is not a regular file", path.display());
  }
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
      .with_context(|| format!("failed to secure {label} at {}", path.display()))?;
  }
  Ok(())
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
