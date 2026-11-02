use super::model::SecretMetadata;
use crate::services::crypto::EncryptedValue;
use sqlx::SqlitePool;
#[derive(sqlx::FromRow)]
pub struct SecretRow {
  pub key: String,
  pub version: i64,
  pub ciphertext: Vec<u8>,
  pub value_nonce: Vec<u8>,
  pub wrapped_key: Vec<u8>,
  pub key_nonce: Vec<u8>,
  pub created_at: String,
  pub updated_at: String,
}
impl SecretRow {
  pub fn encrypted(&self) -> EncryptedValue {
    EncryptedValue {
      ciphertext: self.ciphertext.clone(),
      value_nonce: self.value_nonce.clone(),
      wrapped_key: self.wrapped_key.clone(),
      key_nonce: self.key_nonce.clone(),
    }
  }
  pub fn metadata(&self) -> SecretMetadata {
    SecretMetadata {
      key: self.key.clone(),
      version: self.version,
      created_at: self.created_at.clone(),
      updated_at: self.updated_at.clone(),
    }
  }
}
pub async fn list(
  pool: &SqlitePool,
  environment_id: &str,
) -> Result<Vec<SecretMetadata>, sqlx::Error> {
  sqlx::query_as(
    "SELECT key,version,created_at,updated_at FROM secrets WHERE environment_id=? ORDER BY key",
  )
  .bind(environment_id)
  .fetch_all(pool)
  .await
}
pub async fn rows<'e>(
  executor: impl sqlx::SqliteExecutor<'e>,
  environment_id: &str,
) -> Result<Vec<SecretRow>, sqlx::Error> {
  sqlx::query_as("SELECT key,version,ciphertext,value_nonce,wrapped_key,key_nonce,created_at,updated_at FROM secrets WHERE environment_id=? ORDER BY key").bind(environment_id).fetch_all(executor).await
}
/// The version of each of `keys` the environment has. Keys it doesn't have
/// are left out.
pub async fn versions_of<'e>(
  executor: impl sqlx::SqliteExecutor<'e>,
  environment_id: &str,
  keys: &[&String],
) -> Result<Vec<(String, i64)>, sqlx::Error> {
  sqlx::query_as("SELECT key,version FROM secrets WHERE environment_id=? AND key IN (SELECT value FROM json_each(?))").bind(environment_id).bind(serde_json::json!(keys).to_string()).fetch_all(executor).await
}
/// Like [`rows`], for `keys` only.
pub async fn rows_of<'e>(
  executor: impl sqlx::SqliteExecutor<'e>,
  environment_id: &str,
  keys: &[&String],
) -> Result<Vec<SecretRow>, sqlx::Error> {
  sqlx::query_as("SELECT key,version,ciphertext,value_nonce,wrapped_key,key_nonce,created_at,updated_at FROM secrets WHERE environment_id=? AND key IN (SELECT value FROM json_each(?)) ORDER BY key").bind(environment_id).bind(serde_json::json!(keys).to_string()).fetch_all(executor).await
}
pub async fn find(
  pool: &SqlitePool,
  environment_id: &str,
  key: &str,
) -> Result<Option<SecretRow>, sqlx::Error> {
  sqlx::query_as("SELECT key,version,ciphertext,value_nonce,wrapped_key,key_nonce,created_at,updated_at FROM secrets WHERE environment_id=? AND key=?").bind(environment_id).bind(key).fetch_optional(pool).await
}
pub async fn layout(
  pool: &SqlitePool,
  environment_id: &str,
) -> Result<Option<String>, sqlx::Error> {
  let row: Option<(String,)> =
    sqlx::query_as("SELECT layout FROM environment_env_layout WHERE environment_id=?")
      .bind(environment_id)
      .fetch_optional(pool)
      .await?;
  Ok(row.map(|(layout,)| layout))
}
pub async fn upsert_layout(
  tx: &mut sqlx::SqliteConnection,
  environment_id: &str,
  layout: &str,
  now: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query("INSERT INTO environment_env_layout(environment_id,layout,updated_at)VALUES(?,?,?) ON CONFLICT(environment_id) DO UPDATE SET layout=excluded.layout,updated_at=excluded.updated_at")
        .bind(environment_id)
        .bind(layout)
        .bind(now)
        .execute(tx)
        .await?;
  Ok(())
}

/// What produced a stored version. Stored as text in
/// `secret_versions.change_type`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeType {
  Created,
  Updated,
  Imported,
  RolledBack,
}
impl ChangeType {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Created => "created",
      Self::Updated => "updated",
      Self::Imported => "imported",
      Self::RolledBack => "rolled_back",
    }
  }
}
pub struct VersionActor<'a> {
  pub actor_type: &'a str,
  pub actor_id: Option<&'a str>,
  pub actor_label: Option<&'a str>,
}
pub struct NewVersion<'a> {
  pub environment_id: &'a str,
  pub key: &'a str,
  pub version: i64,
  pub encrypted: EncryptedValue,
  /// When the secret itself was first created. Kept across versions.
  pub created_at: &'a str,
  pub now: &'a str,
  pub change: ChangeType,
  pub source_version: Option<i64>,
  pub actor: VersionActor<'a>,
  /// How many of the newest versions keep their ciphertext.
  pub keep: u32,
}
/// Makes `version` the current value of the secret and records it in the
/// history, then expires ciphertext beyond the retention count. This is the
/// only write path for secret values.
///
/// The upsert must stay `ON CONFLICT DO UPDATE`. `INSERT OR REPLACE` would
/// delete the row first and cascade away the whole history.
pub async fn write_version(
  conn: &mut sqlx::SqliteConnection,
  new: NewVersion<'_>,
) -> Result<(), sqlx::Error> {
  let EncryptedValue {
    ciphertext,
    value_nonce,
    wrapped_key,
    key_nonce,
  } = new.encrypted;
  sqlx::query("INSERT INTO secrets(environment_id,key,version,ciphertext,value_nonce,wrapped_key,key_nonce,created_at,updated_at)VALUES(?,?,?,?,?,?,?,?,?) ON CONFLICT(environment_id,key) DO UPDATE SET version=excluded.version,ciphertext=excluded.ciphertext,value_nonce=excluded.value_nonce,wrapped_key=excluded.wrapped_key,key_nonce=excluded.key_nonce,updated_at=excluded.updated_at")
    .bind(new.environment_id)
    .bind(new.key)
    .bind(new.version)
    .bind(&ciphertext)
    .bind(&value_nonce)
    .bind(&wrapped_key)
    .bind(&key_nonce)
    .bind(new.created_at)
    .bind(new.now)
    .execute(&mut *conn)
    .await?;
  sqlx::query("INSERT INTO secret_versions(environment_id,key,version,ciphertext,value_nonce,wrapped_key,key_nonce,change_type,source_version,actor_type,actor_id,actor_label,created_at)VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)")
    .bind(new.environment_id)
    .bind(new.key)
    .bind(new.version)
    .bind(ciphertext)
    .bind(value_nonce)
    .bind(wrapped_key)
    .bind(key_nonce)
    .bind(new.change.as_str())
    .bind(new.source_version)
    .bind(new.actor.actor_type)
    .bind(new.actor.actor_id)
    .bind(new.actor.actor_label)
    .bind(new.now)
    .execute(&mut *conn)
    .await?;
  sqlx::query("UPDATE secret_versions SET ciphertext=NULL,value_nonce=NULL,wrapped_key=NULL,key_nonce=NULL,expired_at=? WHERE environment_id=? AND key=? AND expired_at IS NULL AND version NOT IN (SELECT version FROM secret_versions WHERE environment_id=? AND key=? ORDER BY version DESC LIMIT ?)")
    .bind(new.now)
    .bind(new.environment_id)
    .bind(new.key)
    .bind(new.environment_id)
    .bind(new.key)
    .bind(i64::from(new.keep))
    .execute(&mut *conn)
    .await?;
  Ok(())
}
#[derive(sqlx::FromRow)]
pub struct VersionRow {
  pub version: i64,
  pub change_type: String,
  pub source_version: Option<i64>,
  pub actor_type: String,
  pub actor_label: Option<String>,
  pub created_at: String,
  pub expired_at: Option<String>,
}
/// Version metadata, newest first, strictly older than `before` when set.
pub async fn list_versions(
  pool: &SqlitePool,
  environment_id: &str,
  key: &str,
  before: Option<i64>,
  limit: u32,
) -> Result<Vec<VersionRow>, sqlx::Error> {
  sqlx::query_as("SELECT version,change_type,source_version,actor_type,actor_label,created_at,expired_at FROM secret_versions WHERE environment_id=? AND key=? AND (? IS NULL OR version<?) ORDER BY version DESC LIMIT ?")
    .bind(environment_id)
    .bind(key)
    .bind(before)
    .bind(before)
    .bind(i64::from(limit))
    .fetch_all(pool)
    .await
}
#[derive(sqlx::FromRow)]
pub struct StoredVersion {
  pub version: i64,
  pub ciphertext: Option<Vec<u8>>,
  pub value_nonce: Option<Vec<u8>>,
  pub wrapped_key: Option<Vec<u8>>,
  pub key_nonce: Option<Vec<u8>>,
}
impl StoredVersion {
  /// The encrypted value, or `None` once retention expired it.
  pub fn encrypted(&self) -> Option<EncryptedValue> {
    Some(EncryptedValue {
      ciphertext: self.ciphertext.clone()?,
      value_nonce: self.value_nonce.clone()?,
      wrapped_key: self.wrapped_key.clone()?,
      key_nonce: self.key_nonce.clone()?,
    })
  }
}
pub async fn find_version(
  conn: &mut sqlx::SqliteConnection,
  environment_id: &str,
  key: &str,
  version: i64,
) -> Result<Option<StoredVersion>, sqlx::Error> {
  sqlx::query_as("SELECT version,ciphertext,value_nonce,wrapped_key,key_nonce FROM secret_versions WHERE environment_id=? AND key=? AND version=?")
    .bind(environment_id)
    .bind(key)
    .bind(version)
    .fetch_optional(conn)
    .await
}
