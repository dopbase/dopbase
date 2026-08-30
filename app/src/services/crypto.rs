use std::{
  fs::{self, OpenOptions},
  io::Write,
  path::Path,
  sync::Arc,
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
  master_key: Arc<Zeroizing<Vec<u8>>>,
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
      master_key: Arc::new(Zeroizing::new(key)),
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

  fn encrypt_master(
    &self,
    value: &[u8],
    aad: &[u8],
  ) -> Result<(Vec<u8>, Vec<u8>)> {
    let nonce = random_nonce()?;
    let cipher = XChaCha20Poly1305::new_from_slice(&self.master_key).expect("32-byte key");
    let ciphertext = cipher
      .encrypt(XNonce::from_slice(&nonce), Payload { msg: value, aad })
      .map_err(|_| anyhow::anyhow!("key wrapping failed"))?;
    Ok((ciphertext, nonce))
  }

  fn decrypt_master(
    &self,
    ciphertext: &[u8],
    nonce: &[u8],
    aad: &[u8],
  ) -> Result<Vec<u8>> {
    if nonce.len() != 24 {
      bail!("invalid nonce");
    }
    let cipher = XChaCha20Poly1305::new_from_slice(&self.master_key).expect("32-byte key");
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
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent)?;
  }
  let mut bytes = vec![0_u8; 32];
  getrandom::fill(&mut bytes)?;
  let mut options = OpenOptions::new();
  options.write(true).create_new(true);
  #[cfg(unix)]
  {
    use std::os::unix::fs::OpenOptionsExt;
    options.mode(0o600);
  }
  let mut file = options
    .open(path)
    .with_context(|| format!("failed to create master key {}", path.display()))?;
  file.write_all(&bytes)?;
  file.sync_all()?;
  Ok(bytes)
}
