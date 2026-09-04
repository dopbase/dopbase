use super::local_config::ResolvedServer;
use crate::{config::ensure_data_dir, utils::private_file};
use anyhow::{Context, Result, bail};
use chacha20poly1305::{
  KeyInit, XChaCha20Poly1305, XNonce,
  aead::{Aead, Payload},
};
use serde::{Deserialize, Serialize};
use std::{
  fs,
  io::ErrorKind,
  path::{Path, PathBuf},
};
use zeroize::Zeroizing;

const SESSION_FILENAME: &str = "session";
const SESSION_KEY_FILENAME: &str = "session-key";
const MAGIC: &[u8; 8] = b"DOPSESS\0";
const VERSION: u8 = 1;
const NONCE_LENGTH: usize = 24;
const KEY_LENGTH: usize = 32;
const HEADER_LENGTH: usize = MAGIC.len() + 1 + NONCE_LENGTH;
const AAD: &[u8] = b"dopbase-cli-session-v1";

#[derive(Deserialize, Serialize)]
struct SessionPayload {
  server_url: String,
  token: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  email: Option<String>,
}

pub struct StoredSession {
  pub token: String,
  pub email: Option<String>,
}

pub fn load(server: &ResolvedServer) -> Result<Option<StoredSession>> {
  let (session_path, key_path) = paths(server)?;
  if !session_path.exists() {
    return Ok(None);
  }
  let stored = fs::read(&session_path)
    .with_context(|| format!("failed to read saved session at {}", session_path.display()))?;
  if stored.len() <= HEADER_LENGTH || &stored[..MAGIC.len()] != MAGIC {
    bail!("saved Dopbase session has an invalid format; run dopbase logout and dopbase login");
  }
  if stored[MAGIC.len()] != VERSION {
    bail!(
      "saved Dopbase session uses an unsupported version; run dopbase logout and dopbase login"
    );
  }
  let key = read_key(&key_path).with_context(|| {
    format!(
      "saved Dopbase session key is unavailable at {}; run dopbase logout and dopbase login",
      key_path.display()
    )
  })?;
  let nonce_start = MAGIC.len() + 1;
  let nonce_end = nonce_start + NONCE_LENGTH;
  let cipher = XChaCha20Poly1305::new_from_slice(&key).expect("validated 32-byte session key");
  let clear = cipher
    .decrypt(
      XNonce::from_slice(&stored[nonce_start..nonce_end]),
      Payload {
        msg: &stored[nonce_end..],
        aad: AAD,
      },
    )
    .map(Zeroizing::new)
    .map_err(|_| {
      anyhow::anyhow!(
        "could not decrypt saved Dopbase session; run dopbase logout and dopbase login"
      )
    })?;
  let payload: SessionPayload = serde_json::from_slice(&clear).map_err(|_| {
    anyhow::anyhow!("saved Dopbase session is invalid; run dopbase logout and dopbase login")
  })?;
  if payload.token.is_empty() {
    bail!("saved Dopbase session is invalid; run dopbase logout and dopbase login");
  }
  Ok((payload.server_url == server.url).then_some(StoredSession {
    token: payload.token,
    email: payload.email,
  }))
}

pub fn save(
  server: &ResolvedServer,
  token: &str,
  email: Option<&str>,
) -> Result<()> {
  if token.is_empty() {
    bail!("Dopbase refused to save an empty session token");
  }
  let (session_path, key_path) = paths(server)?;
  let parent = session_path
    .parent()
    .context("saved session path has no parent directory")?;
  secure_directory(parent)?;
  let key = load_or_create_key(&key_path, session_path.exists())?;
  let clear = Zeroizing::new(serde_json::to_vec(&SessionPayload {
    server_url: server.url.clone(),
    token: token.into(),
    email: email.map(str::to_owned),
  })?);
  let mut nonce = [0_u8; NONCE_LENGTH];
  getrandom::fill(&mut nonce)?;
  let cipher = XChaCha20Poly1305::new_from_slice(&key).expect("validated 32-byte session key");
  let ciphertext = cipher
    .encrypt(
      XNonce::from_slice(&nonce),
      Payload {
        msg: &clear,
        aad: AAD,
      },
    )
    .map_err(|_| anyhow::anyhow!("failed to encrypt Dopbase session"))?;
  let mut envelope = Vec::with_capacity(HEADER_LENGTH + ciphertext.len());
  envelope.extend_from_slice(MAGIC);
  envelope.push(VERSION);
  envelope.extend_from_slice(&nonce);
  envelope.extend_from_slice(&ciphertext);
  private_file::write(&session_path, &envelope, true)?;
  Ok(())
}

pub fn remove(server: &ResolvedServer) -> Result<bool> {
  let (session_path, key_path) = paths(server)?;
  let session_removed = remove_if_present(&session_path)?;
  let key_removed = remove_if_present(&key_path)?;
  Ok(session_removed || key_removed)
}

fn paths(server: &ResolvedServer) -> Result<(PathBuf, PathBuf)> {
  let directory = server
    .config_path
    .parent()
    .context("client configuration path has no parent directory")?;
  Ok((
    directory.join(SESSION_FILENAME),
    directory.join(SESSION_KEY_FILENAME),
  ))
}

fn secure_directory(path: &Path) -> Result<()> {
  ensure_data_dir(path)?;
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
      .with_context(|| format!("failed to secure data directory {}", path.display()))?;
  }
  Ok(())
}

fn load_or_create_key(
  path: &Path,
  session_exists: bool,
) -> Result<Zeroizing<Vec<u8>>> {
  if path.exists() {
    return read_key(path);
  }
  if session_exists {
    bail!("saved Dopbase session key is missing; run dopbase logout and dopbase login");
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
  let key = Zeroizing::new(fs::read(path)?);
  if key.len() != KEY_LENGTH {
    bail!("session key must contain exactly {KEY_LENGTH} bytes");
  }
  Ok(key)
}

fn remove_if_present(path: &Path) -> Result<bool> {
  match fs::remove_file(path) {
    Ok(()) => Ok(true),
    Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
    Err(error) => Err(error)
      .with_context(|| format!("failed to remove saved session file at {}", path.display())),
  }
}
