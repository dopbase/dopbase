use std::{
  fs::{self, OpenOptions},
  io::Write,
  path::Path,
  sync::{Arc, RwLock},
};

use anyhow::{Context, Result, bail};
use chacha20poly1305::{
  KeyInit, XChaCha20Poly1305, XNonce,
  aead::{Aead, Payload},
};
use chrono::Utc;
use sqlx::SqlitePool;
use zeroize::Zeroizing;

const VERIFICATION_TEXT: &[u8] = b"dopbase-master-key-v1";

#[derive(Clone)]
pub struct CryptoService {
  master_key: Arc<RwLock<Zeroizing<Vec<u8>>>>,
}

#[derive(Debug)]
pub struct EncryptedValue {
  pub ciphertext: Vec<u8>,
  pub value_nonce: Vec<u8>,
  pub wrapped_key: Vec<u8>,
  pub key_nonce: Vec<u8>,
}

impl CryptoService {
  pub async fn initialize(
    pool: &SqlitePool,
    path: &Path,
  ) -> Result<Self> {
    let record: Option<(Vec<u8>, Vec<u8>)> = sqlx::query_as(
      "SELECT verification_ciphertext, verification_nonce FROM instance_metadata WHERE id = 1",
    )
    .fetch_optional(pool)
    .await?;

    let key = if record.is_none() && !path.exists() {
      generate_key_file(path)?
    } else {
      read_key_file(path)?
    };
    let service = Self {
      master_key: Arc::new(RwLock::new(Zeroizing::new(key))),
    };

    if let Some((ciphertext, nonce)) = record {
      let clear = service
        .decrypt_master(&ciphertext, &nonce, b"instance-verification")
        .context("configured master key does not match this database")?;
      if clear != VERIFICATION_TEXT {
        bail!("configured master key does not match this database");
      }
    } else {
      let (ciphertext, nonce) =
        service.encrypt_master(VERIFICATION_TEXT, b"instance-verification")?;
      sqlx::query("INSERT INTO instance_metadata(id, verification_ciphertext, verification_nonce, created_at) VALUES(1, ?, ?, ?)")
                .bind(ciphertext).bind(nonce).bind(Utc::now().to_rfc3339()).execute(pool).await?;
    }
    Ok(service)
  }

  pub fn master_key_bytes(&self) -> Vec<u8> {
    self.master_key.read().unwrap().to_vec()
  }

  pub fn replace_master_key(
    &self,
    path: &Path,
    new_key: &[u8],
  ) -> Result<()> {
    if new_key.len() != 32 {
      bail!("master key must contain exactly 32 bytes");
    }
    write_key_file(path, new_key)?;
    let mut lock = self.master_key.write().unwrap();
    *lock = Zeroizing::new(new_key.to_vec());
    Ok(())
  }

  pub fn encrypt(
    &self,
    value: &[u8],
    environment_id: &str,
    key: &str,
    version: i64,
  ) -> Result<EncryptedValue> {
    let mut data_key = Zeroizing::new(vec![0_u8; 32]);
    getrandom::fill(&mut data_key)?;
    let aad = aad(environment_id, key, version);
    let value_nonce = random_nonce()?;
    let cipher = XChaCha20Poly1305::new_from_slice(&data_key).expect("32-byte key");
    let ciphertext = cipher
      .encrypt(
        XNonce::from_slice(&value_nonce),
        Payload {
          msg: value,
          aad: &aad,
        },
      )
      .map_err(|_| anyhow::anyhow!("secret encryption failed"))?;
    let (wrapped_key, key_nonce) = self.encrypt_master(&data_key, &aad)?;
    Ok(EncryptedValue {
      ciphertext,
      value_nonce,
      wrapped_key,
      key_nonce,
    })
  }

  pub fn decrypt(
    &self,
    encrypted: &EncryptedValue,
    environment_id: &str,
    key: &str,
    version: i64,
  ) -> Result<Zeroizing<Vec<u8>>> {
    let aad = aad(environment_id, key, version);
    let data_key =
      Zeroizing::new(self.decrypt_master(&encrypted.wrapped_key, &encrypted.key_nonce, &aad)?);
    let cipher = XChaCha20Poly1305::new_from_slice(&data_key)
      .map_err(|_| anyhow::anyhow!("invalid data key"))?;
    let clear = cipher
      .decrypt(
        XNonce::from_slice(&encrypted.value_nonce),
        Payload {
          msg: &encrypted.ciphertext,
          aad: &aad,
        },
      )
      .map_err(|_| anyhow::anyhow!("secret authentication failed"))?;
    Ok(Zeroizing::new(clear))
  }

  pub fn encrypt_master(
    &self,
    value: &[u8],
    aad: &[u8],
  ) -> Result<(Vec<u8>, Vec<u8>)> {
    let nonce = random_nonce()?;
    let key = self.master_key.read().unwrap();
    let cipher = XChaCha20Poly1305::new_from_slice(&key).expect("32-byte key");
    let ciphertext = cipher
      .encrypt(XNonce::from_slice(&nonce), Payload { msg: value, aad })
      .map_err(|_| anyhow::anyhow!("key wrapping failed"))?;
    Ok((ciphertext, nonce))
  }

  pub fn decrypt_master(
    &self,
    ciphertext: &[u8],
    nonce: &[u8],
    aad: &[u8],
  ) -> Result<Vec<u8>> {
    if nonce.len() != 24 {
      bail!("invalid nonce");
    }
    let key = self.master_key.read().unwrap();
    let cipher = XChaCha20Poly1305::new_from_slice(&key).expect("32-byte key");
    cipher
      .decrypt(
        XNonce::from_slice(nonce),
        Payload {
          msg: ciphertext,
          aad,
        },
      )
      .map_err(|_| anyhow::anyhow!("key authentication failed"))
  }

  pub fn encrypt_backup(
    &self,
    payload: &[u8],
  ) -> Result<Vec<u8>> {
    let nonce = random_nonce()?;
    let key = self.master_key.read().unwrap();
    let cipher = XChaCha20Poly1305::new_from_slice(&key).expect("32-byte key");
    let ciphertext = cipher
      .encrypt(
        XNonce::from_slice(&nonce),
        Payload {
          msg: payload,
          aad: b"dopbase-backup-v1",
        },
      )
      .map_err(|_| anyhow::anyhow!("backup encryption failed"))?;
    let mut out = Vec::with_capacity(12 + 24 + ciphertext.len());
    out.extend_from_slice(b"DOPBASE_BK1\0");
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(out)
  }

  pub fn decrypt_backup(
    &self,
    data: &[u8],
  ) -> Result<Vec<u8>> {
    let key = self.master_key.read().unwrap();
    Self::decrypt_backup_payload(data, &key)
  }

  pub fn decrypt_backup_with_key(
    data: &[u8],
    key: &[u8],
  ) -> Result<Vec<u8>> {
    Self::decrypt_backup_payload(data, key)
  }

  fn decrypt_backup_payload(
    data: &[u8],
    key: &[u8],
  ) -> Result<Vec<u8>> {
    const MAGIC: &[u8] = b"DOPBASE_BK1\0";
    if key.len() != 32 {
      bail!("master key must contain exactly 32 bytes");
    }
    if data.len() < MAGIC.len() + 24 + 16 {
      bail!("invalid backup file: file too short");
    }
    if &data[..MAGIC.len()] != MAGIC {
      bail!("invalid backup file: magic header mismatch");
    }
    let nonce = &data[MAGIC.len()..MAGIC.len() + 24];
    let ciphertext = &data[MAGIC.len() + 24..];
    let cipher = XChaCha20Poly1305::new_from_slice(key).expect("32-byte key");
    cipher
      .decrypt(
        XNonce::from_slice(nonce),
        Payload {
          msg: ciphertext,
          aad: b"dopbase-backup-v1",
        },
      )
      .map_err(|_| anyhow::anyhow!("backup decryption/authentication failed"))
  }

  pub fn from_key(key: Vec<u8>) -> Self {
    Self {
      master_key: Arc::new(RwLock::new(Zeroizing::new(key))),
    }
  }
}

pub async fn rekey_database(
  pool: &SqlitePool,
  old_master_key: &[u8],
  new_master_key: &[u8],
) -> Result<()> {
  if old_master_key == new_master_key {
    return Ok(());
  }
  if old_master_key.len() != 32 || new_master_key.len() != 32 {
    bail!("master keys must be exactly 32 bytes");
  }

  let old_crypto = CryptoService::from_key(old_master_key.to_vec());
  let new_crypto = CryptoService::from_key(new_master_key.to_vec());
  let mut tx = pool.begin().await?;

  // 1. Re-key instance verification metadata
  let meta: Option<(Vec<u8>, Vec<u8>)> = sqlx::query_as(
    "SELECT verification_ciphertext, verification_nonce FROM instance_metadata WHERE id = 1",
  )
  .fetch_optional(&mut *tx)
  .await?;

  if let Some((ciphertext, nonce)) = meta {
    let clear = old_crypto
      .decrypt_master(&ciphertext, &nonce, b"instance-verification")
      .context("provided master key cannot decrypt instance metadata")?;
    if clear != VERIFICATION_TEXT {
      bail!("provided master key verification text mismatch");
    }
    let (new_cipher, new_nonce) =
      new_crypto.encrypt_master(VERIFICATION_TEXT, b"instance-verification")?;
    sqlx::query(
      "UPDATE instance_metadata SET verification_ciphertext = ?, verification_nonce = ? WHERE id = 1",
    )
    .bind(new_cipher)
    .bind(new_nonce)
    .execute(&mut *tx)
    .await?;
  }

  // 2. Re-wrap data keys in secrets table
  type SecretKeyRow = (String, String, i64, Vec<u8>, Vec<u8>);
  let rows: Vec<SecretKeyRow> =
    sqlx::query_as("SELECT environment_id, key, version, wrapped_key, key_nonce FROM secrets")
      .fetch_all(&mut *tx)
      .await?;

  for (env_id, secret_key, version, wrapped_key, key_nonce) in rows {
    let secret_aad = aad(&env_id, &secret_key, version);
    let data_key = old_crypto
      .decrypt_master(&wrapped_key, &key_nonce, &secret_aad)
      .with_context(|| {
        format!("failed to unwrap secret {env_id}:{secret_key} with provided master key")
      })?;
    let (new_wrapped, new_nonce) = new_crypto.encrypt_master(&data_key, &secret_aad)?;
    sqlx::query(
      "UPDATE secrets SET wrapped_key = ?, key_nonce = ? WHERE environment_id = ? AND key = ?",
    )
    .bind(new_wrapped)
    .bind(new_nonce)
    .bind(&env_id)
    .bind(&secret_key)
    .execute(&mut *tx)
    .await?;
  }

  tx.commit().await?;
  Ok(())
}

pub fn parse_master_key(input: &[u8]) -> Result<Vec<u8>> {
  if input.len() == 32 {
    return Ok(input.to_vec());
  }
  let trimmed = std::str::from_utf8(input).map(|s| s.trim()).unwrap_or("");
  if trimmed.len() == 64
    && let Ok(bytes) = hex::decode(trimmed)
    && bytes.len() == 32
  {
    return Ok(bytes);
  }
  bail!("master key must contain 32 raw bytes or 64-character hex string");
}

pub fn write_key_file(
  path: &Path,
  bytes: &[u8],
) -> Result<()> {
  if bytes.len() != 32 {
    bail!("master key must contain exactly 32 bytes");
  }
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent)?;
  }
  let temp_path = path.with_extension("tmp");
  let mut options = OpenOptions::new();
  options.write(true).create(true).truncate(true);
  #[cfg(unix)]
  {
    use std::os::unix::fs::OpenOptionsExt;
    options.mode(0o600);
  }
  let mut file = options
    .open(&temp_path)
    .with_context(|| format!("failed to create temp master key {}", temp_path.display()))?;
  file.write_all(bytes)?;
  file.sync_all()?;
  fs::rename(&temp_path, path)
    .with_context(|| format!("failed to atomically replace master key {}", path.display()))?;
  Ok(())
}

fn aad(
  environment_id: &str,
  key: &str,
  version: i64,
) -> Vec<u8> {
  format!("dopbase:v1:{environment_id}:{key}:{version}").into_bytes()
}

fn random_nonce() -> Result<Vec<u8>> {
  let mut nonce = vec![0_u8; 24];
  getrandom::fill(&mut nonce)?;
  Ok(nonce)
}

fn read_key_file(path: &Path) -> Result<Vec<u8>> {
  let bytes =
    fs::read(path).with_context(|| format!("failed to read master key {}", path.display()))?;
  if bytes.len() != 32 {
    bail!("master key must contain exactly 32 bytes");
  }
  Ok(bytes)
}

fn generate_key_file(path: &Path) -> Result<Vec<u8>> {
  let mut bytes = vec![0_u8; 32];
  getrandom::fill(&mut bytes)?;
  write_key_file(path, &bytes)?;
  Ok(bytes)
}
