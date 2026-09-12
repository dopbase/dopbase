use crate::{
  cli::local_config::ResolvedServer, config::ensure_data_dir, models::SecretInput,
  utils::private_file,
};
use anyhow::{Context, Result, bail};
use chacha20poly1305::{
  KeyInit, XChaCha20Poly1305, XNonce,
  aead::{Aead, Payload},
};
use chrono::{DateTime, SecondsFormat, Utc};
use hkdf::Hkdf;
use serde::{Deserialize, Serialize, de::IgnoredAny};
use sha2::{Digest, Sha256};
use std::{
  collections::BTreeMap,
  fs::{self, OpenOptions},
  io::ErrorKind,
  path::{Path, PathBuf},
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
const MAX_CACHED_ENVIRONMENTS: usize = 16;
const MAX_CACHE_DOCUMENT_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct CachedRuntime {
  pub project: String,
  pub environment: String,
  pub environment_id: String,
  pub aliases: Vec<String>,
  pub fetched_at: DateTime<Utc>,
  pub entries: Vec<SecretInput>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CacheMetadata {
  pub project: String,
  pub environment: String,
  pub environment_id: String,
  pub fetched_at: String,
  pub age: String,
  pub age_seconds: i64,
  pub aliases: Vec<String>,
  #[serde(skip)]
  fetched_at_value: DateTime<Utc>,
}

impl CacheMetadata {
  pub fn is_older_than(
    &self,
    now: DateTime<Utc>,
    seconds: i64,
  ) -> bool {
    now
      .signed_duration_since(self.fetched_at_value)
      .num_seconds()
      > seconds
  }

  pub fn candidate(&self) -> CacheCandidate {
    CacheCandidate {
      environment_id: self.environment_id.clone(),
      fetched_at: self.fetched_at_value,
    }
  }
}

#[derive(Clone, Debug)]
pub struct CacheCandidate {
  environment_id: String,
  fetched_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct CacheSnapshot {
  pub server_url: String,
  pub entries: Vec<CacheMetadata>,
}

#[derive(Debug)]
pub struct CacheRemoval {
  pub removed: Vec<CacheMetadata>,
  pub remaining_count: usize,
}

#[derive(Debug, Deserialize, Serialize)]
struct StoredRuntime {
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
  environments: BTreeMap<String, StoredRuntime>,
}

#[derive(Debug, Deserialize)]
struct MetadataRuntime {
  project: String,
  environment: String,
  environment_id: String,
  aliases: Vec<String>,
  fetched_at: DateTime<Utc>,
  #[serde(rename = "entries")]
  _entries: IgnoredAny,
}

#[derive(Debug, Deserialize)]
struct MetadataDocument {
  version: u8,
  server_url: String,
  environments: BTreeMap<String, MetadataRuntime>,
}

pub fn save(
  server: &ResolvedServer,
  credential: &str,
  reference: &str,
  runtime: &CachedRuntime,
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
  lock.lock().context("failed to lock run cache")?;
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
  aliases.extend(runtime.aliases.iter().cloned());
  aliases.sort();
  aliases.dedup();
  document.environments.insert(
    runtime.environment_id.clone(),
    StoredRuntime {
      project: runtime.project.clone(),
      environment: runtime.environment.clone(),
      environment_id: runtime.environment_id.clone(),
      aliases,
      fetched_at: runtime.fetched_at,
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
  write_document(&paths.cache, &derived_key[..], &document)
}

pub fn load(
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
  validate_document(&document, server)?;
  let cached = document
    .environments
    .into_values()
    .filter(|cached| cached.aliases.iter().any(|alias| alias == reference))
    .max_by_key(|cached| cached.fetched_at)
    .with_context(|| format!("encrypted run cache has no entry matching {reference}"))?;
  Ok(CachedRuntime {
    project: cached.project,
    environment: cached.environment,
    environment_id: cached.environment_id,
    aliases: cached.aliases,
    fetched_at: cached.fetched_at,
    entries: cached.entries,
  })
}

pub fn inspect(
  server: &ResolvedServer,
  credential: Option<&str>,
  now: DateTime<Utc>,
) -> Result<CacheSnapshot> {
  let paths = paths(server)?;
  if !paths.cache.exists() {
    return Ok(CacheSnapshot {
      server_url: server.url.clone(),
      entries: Vec::new(),
    });
  }
  let credential = credential.context(
    "Dopbase authentication is required to unlock the encrypted runtime cache. Run `dopbase login` first or set DOPBASE_TOKEN.",
  )?;
  let local_key =
    read_key_read_only(&paths.key).map_err(|error| recovery_error(&paths.cache, error))?;
  let derived_key = derive_key(&local_key, credential, &server.url)?;
  let document = read_metadata_document(&paths.cache, &derived_key[..])
    .map_err(|error| recovery_error(&paths.cache, error))?;
  if document.version != VERSION || document.server_url != server.url {
    return Err(recovery_error(
      &paths.cache,
      anyhow::anyhow!("encrypted run cache has an invalid scope or unsupported version"),
    ));
  }
  let mut entries = document
    .environments
    .into_values()
    .map(|cached| metadata(cached, now))
    .collect::<Vec<_>>();
  sort_metadata(&mut entries);
  Ok(CacheSnapshot {
    server_url: server.url.clone(),
    entries,
  })
}

pub fn has_document(server: &ResolvedServer) -> Result<bool> {
  Ok(paths(server)?.cache.exists())
}

pub fn remove(
  server: &ResolvedServer,
  credential: &str,
  candidates: &[CacheCandidate],
  now: DateTime<Utc>,
) -> Result<CacheRemoval> {
  let paths = paths(server)?;
  if candidates.is_empty() || !paths.cache.exists() {
    return Ok(CacheRemoval {
      removed: Vec::new(),
      remaining_count: inspect(server, Some(credential), now)?.entries.len(),
    });
  }
  let lock = open_lock(&paths.lock)?;
  lock.lock().context("failed to lock run cache")?;
  if !paths.cache.exists() {
    return Ok(CacheRemoval {
      removed: Vec::new(),
      remaining_count: 0,
    });
  }
  let local_key = read_key(&paths.key).map_err(|error| recovery_error(&paths.cache, error))?;
  let derived_key = derive_key(&local_key, credential, &server.url)?;
  let mut document = read_document(&paths.cache, &derived_key[..])
    .map_err(|error| recovery_error(&paths.cache, error))?;
  validate_document(&document, server).map_err(|error| recovery_error(&paths.cache, error))?;

  let mut removed = Vec::new();
  for candidate in candidates {
    let unchanged = document
      .environments
      .get(&candidate.environment_id)
      .is_some_and(|cached| cached.fetched_at == candidate.fetched_at);
    if unchanged {
      let cached = document
        .environments
        .remove(&candidate.environment_id)
        .expect("cache candidate was checked immediately before removal");
      removed.push(metadata(cached.into(), now));
    }
  }
  sort_metadata(&mut removed);
  let remaining_count = document.environments.len();
  if removed.is_empty() {
    return Ok(CacheRemoval {
      removed,
      remaining_count,
    });
  }
  if document.environments.is_empty() {
    remove_cache_file(&paths.cache)?;
  } else {
    write_document(&paths.cache, &derived_key[..], &document)?;
  }
  Ok(CacheRemoval {
    removed,
    remaining_count,
  })
}

impl From<StoredRuntime> for MetadataRuntime {
  fn from(runtime: StoredRuntime) -> Self {
    Self {
      project: runtime.project,
      environment: runtime.environment,
      environment_id: runtime.environment_id,
      aliases: runtime.aliases,
      fetched_at: runtime.fetched_at,
      _entries: IgnoredAny,
    }
  }
}

fn metadata(
  runtime: MetadataRuntime,
  now: DateTime<Utc>,
) -> CacheMetadata {
  let age_seconds = now
    .signed_duration_since(runtime.fetched_at)
    .num_seconds()
    .max(0);
  CacheMetadata {
    project: runtime.project,
    environment: runtime.environment,
    environment_id: runtime.environment_id,
    fetched_at: runtime
      .fetched_at
      .to_rfc3339_opts(SecondsFormat::Secs, true),
    age: format_age(age_seconds),
    age_seconds,
    aliases: runtime.aliases,
    fetched_at_value: runtime.fetched_at,
  }
}

fn sort_metadata(entries: &mut [CacheMetadata]) {
  entries.sort_by(|left, right| {
    (&left.project, &left.environment, &left.environment_id).cmp(&(
      &right.project,
      &right.environment,
      &right.environment_id,
    ))
  });
}

fn validate_document(
  document: &CacheDocument,
  server: &ResolvedServer,
) -> Result<()> {
  if document.version != VERSION || document.server_url != server.url {
    bail!("encrypted run cache has an invalid scope or unsupported version");
  }
  Ok(())
}

fn recovery_error(
  path: &Path,
  error: anyhow::Error,
) -> anyhow::Error {
  anyhow::anyhow!(
    "encrypted runtime cache at {} could not be read: {}. The file was not changed. Retry with the credential that created it. If the file is damaged or no longer needed, move it out of the run-cache directory and run `dopbase run` while the server is available",
    path.display(),
    error
  )
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
  read_validated_key(path)
}

fn read_key_read_only(path: &Path) -> Result<Zeroizing<Vec<u8>>> {
  validate_regular_file(path, "run cache key")?;
  read_validated_key(path)
}

fn read_validated_key(path: &Path) -> Result<Zeroizing<Vec<u8>>> {
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

fn write_document(
  path: &Path,
  key: &[u8],
  document: &CacheDocument,
) -> Result<()> {
  let clear = Zeroizing::new(serde_json::to_vec(document)?);
  if clear.len() as u64 > MAX_CACHE_DOCUMENT_BYTES {
    bail!("encrypted run cache exceeded the maximum allowed size");
  }
  let envelope = encrypt(key, &clear)?;
  private_file::write(path, &envelope, true)
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
  let clear = decrypt(path, key, true)?;
  serde_json::from_slice(&clear).context("encrypted run cache payload is invalid")
}

fn read_metadata_document(
  path: &Path,
  key: &[u8],
) -> Result<MetadataDocument> {
  let clear = decrypt(path, key, false)?;
  serde_json::from_slice(&clear).context("encrypted run cache payload is invalid")
}

fn decrypt(
  path: &Path,
  key: &[u8],
  secure_permissions: bool,
) -> Result<Zeroizing<Vec<u8>>> {
  if secure_permissions {
    secure_file(path, "run cache")?;
  } else {
    validate_regular_file(path, "run cache")?;
  }
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
  cipher
    .decrypt(
      XNonce::from_slice(&stored[nonce_start..nonce_end]),
      Payload {
        msg: &stored[nonce_end..],
        aad: AAD,
      },
    )
    .map(Zeroizing::new)
    .map_err(|_| anyhow::anyhow!("encrypted run cache authentication failed"))
}

fn secure_file(
  path: &Path,
  label: &str,
) -> Result<()> {
  validate_regular_file(path, label)?;
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
      .with_context(|| format!("failed to secure {label} at {}", path.display()))?;
  }
  Ok(())
}

fn validate_regular_file(
  path: &Path,
  label: &str,
) -> Result<()> {
  let metadata = fs::symlink_metadata(path)
    .with_context(|| format!("failed to inspect {label} at {}", path.display()))?;
  if !metadata.file_type().is_file() {
    bail!("{label} at {} is not a regular file", path.display());
  }
  Ok(())
}

fn remove_cache_file(path: &Path) -> Result<()> {
  match fs::remove_file(path) {
    Ok(()) => {}
    Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
    Err(error) => {
      return Err(error)
        .with_context(|| format!("failed to remove encrypted run cache at {}", path.display()));
    }
  }
  #[cfg(unix)]
  if let Some(parent) = path.parent() {
    use std::fs::File;
    File::open(parent)?.sync_all()?;
  }
  Ok(())
}

fn format_age(seconds: i64) -> String {
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
