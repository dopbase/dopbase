use super::model::SecretMetadata;
use crate::services::crypto::EncryptedValue;
use chrono::{DateTime, Utc};
use sqlx::{Executor, Sqlite, SqlitePool};
/// Every column of a secret, including the previous value that stays valid
/// during a rotation grace period.
const COLUMNS: &str = "key,version,ciphertext,value_nonce,wrapped_key,key_nonce,created_at,updated_at,previous_version,previous_ciphertext,previous_value_nonce,previous_wrapped_key,previous_key_nonce,previous_expires_at";
/// The columns that never contain secret values, used for metadata reads.
const META_COLUMNS: &str = "key,version,created_at,updated_at,previous_version,previous_expires_at";
/// Clears the previous value without touching the current one. Ending a
/// rotation or an overwrite always removes the previous value.
const CLEAR_PREVIOUS: &str = "previous_version=NULL,previous_ciphertext=NULL,previous_value_nonce=NULL,previous_wrapped_key=NULL,previous_key_nonce=NULL,previous_expires_at=NULL";
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
  pub previous_version: Option<i64>,
  pub previous_ciphertext: Option<Vec<u8>>,
  pub previous_value_nonce: Option<Vec<u8>>,
  pub previous_wrapped_key: Option<Vec<u8>>,
  pub previous_key_nonce: Option<Vec<u8>>,
  pub previous_expires_at: Option<String>,
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
  /// The encrypted previous value. Each value keeps the version it was
  /// encrypted with, so both stay decryptable without re-encryption.
  pub fn previous_encrypted(&self) -> Option<EncryptedValue> {
    Some(EncryptedValue {
      ciphertext: self.previous_ciphertext.clone()?,
      value_nonce: self.previous_value_nonce.clone()?,
      wrapped_key: self.previous_wrapped_key.clone()?,
      key_nonce: self.previous_key_nonce.clone()?,
    })
  }
  pub fn previous(&self) -> Option<(i64, &str)> {
    Some((self.previous_version?, self.previous_expires_at.as_deref()?))
  }
  /// The previous value while its grace period is still running.
  pub fn live_previous(
    &self,
    now: &DateTime<Utc>,
  ) -> Option<(i64, String)> {
    let (version, expires_at) = self.previous()?;
    live(version, expires_at, now)
  }
  /// The previous value after its grace period ended. It is removed on the
  /// next write to the secret.
  pub fn expired_previous(
    &self,
    now: &DateTime<Utc>,
  ) -> Option<(i64, String)> {
    let (version, expires_at) = self.previous()?;
    expired(version, expires_at, now)
  }
  pub fn metadata(
    &self,
    now: &DateTime<Utc>,
  ) -> SecretMetadata {
    metadata(
      self.key.clone(),
      self.version,
      self.created_at.clone(),
      self.updated_at.clone(),
      self.live_previous(now),
    )
  }
}
/// Metadata read shape. Selecting only these columns keeps ciphertext out of
/// metadata responses.
#[derive(sqlx::FromRow)]
pub struct SecretMetaRow {
  pub key: String,
  pub version: i64,
  pub created_at: String,
  pub updated_at: String,
  pub previous_version: Option<i64>,
  pub previous_expires_at: Option<String>,
}
impl SecretMetaRow {
  pub fn previous(&self) -> Option<(i64, &str)> {
    Some((self.previous_version?, self.previous_expires_at.as_deref()?))
  }
  /// The previous value while its grace period is still running.
  pub fn live_previous(
    &self,
    now: &DateTime<Utc>,
  ) -> Option<(i64, String)> {
    let (version, expires_at) = self.previous()?;
    live(version, expires_at, now)
  }
  /// The previous value after its grace period ended. It is removed on the
  /// next write to the secret.
  pub fn expired_previous(
    &self,
    now: &DateTime<Utc>,
  ) -> Option<(i64, String)> {
    let (version, expires_at) = self.previous()?;
    expired(version, expires_at, now)
  }
  pub fn metadata(
    &self,
    now: &DateTime<Utc>,
  ) -> SecretMetadata {
    metadata(
      self.key.clone(),
      self.version,
      self.created_at.clone(),
      self.updated_at.clone(),
      self.live_previous(now),
    )
  }
}
fn deadline(expires_at: &str) -> Option<DateTime<Utc>> {
  DateTime::parse_from_rfc3339(expires_at)
    .ok()
    .map(|deadline| deadline.with_timezone(&Utc))
}
fn live(
  version: i64,
  expires_at: &str,
  now: &DateTime<Utc>,
) -> Option<(i64, String)> {
  deadline(expires_at)
    .is_none_or(|deadline| deadline > *now)
    .then(|| (version, expires_at.to_owned()))
}
fn expired(
  version: i64,
  expires_at: &str,
  now: &DateTime<Utc>,
) -> Option<(i64, String)> {
  deadline(expires_at)
    .is_some_and(|deadline| deadline <= *now)
    .then(|| (version, expires_at.to_owned()))
}
fn metadata(
  key: String,
  version: i64,
  created_at: String,
  updated_at: String,
  previous: Option<(i64, String)>,
) -> SecretMetadata {
  SecretMetadata {
    key,
    version,
    created_at,
    updated_at,
    previous_version: previous.as_ref().map(|(version, _)| *version),
    previous_expires_at: previous.map(|(_, expires_at)| expires_at),
  }
}
pub async fn list<'e, E>(
  executor: E,
  environment_id: &str,
) -> Result<Vec<SecretMetaRow>, sqlx::Error>
where
  E: Executor<'e, Database = Sqlite>,
{
  sqlx::query_as(&format!(
    "SELECT {META_COLUMNS} FROM secrets WHERE environment_id=? ORDER BY key"
  ))
  .bind(environment_id)
  .fetch_all(executor)
  .await
}
pub async fn rows<'e, E>(
  executor: E,
  environment_id: &str,
) -> Result<Vec<SecretRow>, sqlx::Error>
where
  E: Executor<'e, Database = Sqlite>,
{
  sqlx::query_as(&format!(
    "SELECT {COLUMNS} FROM secrets WHERE environment_id=? ORDER BY key"
  ))
  .bind(environment_id)
  .fetch_all(executor)
  .await
}
pub async fn find<'e, E>(
  executor: E,
  environment_id: &str,
  key: &str,
) -> Result<Option<SecretRow>, sqlx::Error>
where
  E: Executor<'e, Database = Sqlite>,
{
  sqlx::query_as(&format!(
    "SELECT {COLUMNS} FROM secrets WHERE environment_id=? AND key=?"
  ))
  .bind(environment_id)
  .bind(key)
  .fetch_optional(executor)
  .await
}
/// Removes a previous value whose grace period ended. The deadline is part of
/// the condition, so a concurrent rotation that started a new grace period is
/// never cleared by a reader that saw the old state.
pub async fn retire_previous(
  tx: &mut sqlx::SqliteConnection,
  environment_id: &str,
  key: &str,
  expires_at: &str,
) -> Result<bool, sqlx::Error> {
  let result = sqlx::query(&format!(
    "UPDATE secrets SET {CLEAR_PREVIOUS} WHERE environment_id=? AND key=? AND previous_expires_at=?"
  ))
  .bind(environment_id)
  .bind(key)
  .bind(expires_at)
  .execute(&mut *tx)
  .await?;
  Ok(result.rows_affected() == 1)
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
