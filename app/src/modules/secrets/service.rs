use super::{model::*, repository};
use crate::modules::common;
use crate::{
  constants::{
    errors::{
      IMPORT_DUPLICATE_KEY, SECRET_COUNT_LIMIT_MESSAGE, SECRET_LIMIT_EXCEEDED,
      SECRET_TOTAL_SIZE_LIMIT_MESSAGE, TOKEN_SCOPE_INVALID,
    },
    limits::{MAX_ENV_LAYOUT_BYTES, MAX_SECRET_COLLECTION_BYTES, MAX_SECRETS_PER_ENVIRONMENT},
  },
  extractors::{require_metadata_access, require_project_manager, require_recent_browser_auth},
  http::HttpError,
  models::{AuthIdentity, SecretInput},
  services::{crypto::EncryptedValue, rotation},
  state::AppState,
};
use chrono::{DateTime, Duration, Utc};
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::OnceLock;
/// Who performed the operation, in the shape the audit log stores.
type Actor<'a> = (&'a str, Option<&'a str>, Option<&'a str>);
/// A previous value whose grace period ended and that still occupies the
/// second slot of a secret.
struct Retired {
  key: String,
  version: i64,
  expires_at: String,
}
/// Rows that can hold a previous value, whether or not they carry ciphertext.
trait PreviousValue {
  fn key(&self) -> &str;
  fn expired_previous(
    &self,
    now: &DateTime<Utc>,
  ) -> Option<(i64, String)>;
}
impl PreviousValue for repository::SecretRow {
  fn key(&self) -> &str {
    &self.key
  }
  fn expired_previous(
    &self,
    now: &DateTime<Utc>,
  ) -> Option<(i64, String)> {
    repository::SecretRow::expired_previous(self, now)
  }
}
impl PreviousValue for repository::SecretMetaRow {
  fn key(&self) -> &str {
    &self.key
  }
  fn expired_previous(
    &self,
    now: &DateTime<Utc>,
  ) -> Option<(i64, String)> {
    repository::SecretMetaRow::expired_previous(self, now)
  }
}
fn retired<R: PreviousValue>(
  rows: &[R],
  now: &DateTime<Utc>,
) -> Vec<Retired> {
  rows
    .iter()
    .filter_map(|row| {
      let (version, expires_at) = row.expired_previous(now)?;
      Some(Retired {
        key: row.key().to_owned(),
        version,
        expires_at,
      })
    })
    .collect()
}
/// Removes previous values whose grace period ended and records one
/// `secret.retired` event for each. Reads enforce retirement; Dopbase has no
/// background sweeper. The deadline is part of the condition, so a rotation
/// that started a new grace period is never retired by a reader that saw the
/// previous state.
async fn retire(
  conn: &mut sqlx::SqliteConnection,
  actor: Actor<'_>,
  project_id: &str,
  environment_id: &str,
  retired: &[Retired],
) -> Result<(), HttpError> {
  for entry in retired {
    // Each retirement commits together with its audit record. The update runs
    // first, so the transaction holds the write lock the audit chain needs.
    let mut tx = sqlx::Connection::begin(conn).await?;
    if !repository::retire_previous(&mut tx, environment_id, &entry.key, &entry.expires_at).await? {
      continue;
    }
    common::audit(
      &mut tx,
      actor.0,
      actor.1,
      actor.2,
      "secret.retired",
      Some(project_id),
      Some(environment_id),
      Some("secret"),
      Some(&entry.key),
      serde_json::json!({"key":entry.key,"version":entry.version,"expiresAt":entry.expires_at}),
    )
    .await?;
    tx.commit().await?;
  }
  Ok(())
}
/// Reads one secret and retires a previous value whose grace period ended.
async fn load(
  conn: &mut sqlx::SqliteConnection,
  actor: Actor<'_>,
  project_id: &str,
  environment_id: &str,
  key: &str,
) -> Result<Option<repository::SecretRow>, HttpError> {
  let row = repository::find(&mut *conn, environment_id, key).await?;
  let Some(row) = row else {
    return Ok(None);
  };
  let retired = retired(std::slice::from_ref(&row), &Utc::now());
  retire(conn, actor, project_id, environment_id, &retired).await?;
  Ok(Some(row))
}
/// Reads every secret of an environment and retires previous values whose
/// grace period ended.
async fn load_all(
  conn: &mut sqlx::SqliteConnection,
  actor: Actor<'_>,
  project_id: &str,
  environment_id: &str,
) -> Result<Vec<repository::SecretRow>, HttpError> {
  let rows = repository::rows(&mut *conn, environment_id).await?;
  let retired = retired(&rows, &Utc::now());
  retire(conn, actor, project_id, environment_id, &retired).await?;
  Ok(rows)
}
pub fn validate_entry(entry: &SecretInput) -> Result<(), HttpError> {
  static KEY: OnceLock<Regex> = OnceLock::new();
  let valid = KEY.get_or_init(|| Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*$").unwrap());
  let mut errors = BTreeMap::new();
  if entry.key.len() > 128 || !valid.is_match(&entry.key) {
    errors.insert(
      "SECRET_KEY_INVALID".into(),
      "Secret keys must use letters, digits, and underscores and may not begin with a digit."
        .into(),
    );
  }
  if entry.value.len() > 64 * 1024 {
    errors.insert(
      "SECRET_VALUE_TOO_LARGE".into(),
      "A secret value may contain at most 64 KiB.".into(),
    );
  }
  if errors.is_empty() {
    Ok(())
  } else {
    Err(HttpError::validation(errors))
  }
}
fn validate_grace(requested: Option<i64>) -> Result<Duration, HttpError> {
  let seconds = requested.unwrap_or_else(|| rotation::default_grace().num_seconds());
  if !(0..=rotation::max_grace_seconds()).contains(&seconds) {
    return Err(HttpError::validation(BTreeMap::from([(
      "SECRET_GRACE_INVALID".into(),
      rotation::GRACE_ERROR.into(),
    )])));
  }
  Ok(Duration::seconds(seconds))
}
async fn environment(
  state: &AppState,
  id: &str,
) -> Result<crate::modules::environments::model::EnvironmentResponse, HttpError> {
  crate::modules::environments::service::show(state, id).await
}
pub async fn list(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
) -> Result<Vec<SecretMetadata>, HttpError> {
  require_metadata_access(identity)?;
  let env = environment(state, id).await?;
  let mut conn = state.db.pool().acquire().await?;
  let rows: Vec<repository::SecretMetaRow> = repository::list(&mut *conn, id).await?;
  let now = Utc::now();
  let retired = retired(&rows, &now);
  retire(&mut conn, actor(identity), &env.project_id, id, &retired).await?;
  Ok(rows.iter().map(|row| row.metadata(&now)).collect())
}
pub async fn get(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
  key: &str,
) -> Result<SecretMetadata, HttpError> {
  require_metadata_access(identity)?;
  let env = environment(state, id).await?;
  let mut conn = state.db.pool().acquire().await?;
  load(&mut conn, actor(identity), &env.project_id, id, key)
    .await?
    .map(|row| row.metadata(&Utc::now()))
    .ok_or_else(|| HttpError::not_found("SECRET_NOT_FOUND", "The requested secret was not found."))
}
pub async fn set(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
  key: String,
  request: SetSecretRequest,
) -> Result<SecretMetadata, HttpError> {
  let input = SecretInput {
    key: key.clone(),
    value: request.value,
  };
  validate_entry(&input)?;
  let (admin_id, email) = require_project_manager(identity)?;
  let env = environment(state, id).await?;
  let mut tx = state.db.pool().begin_with("BEGIN IMMEDIATE").await?;
  let existing = repository::find(&mut *tx, id, &key).await?;
  let version = existing.as_ref().map_or(1, |row| row.version + 1);
  let created_at = existing
    .as_ref()
    .map_or_else(|| Utc::now().to_rfc3339(), |row| row.created_at.clone());
  let encrypted = state
    .crypto
    .encrypt(input.value.as_bytes(), id, &key, version)
    .map_err(|_| HttpError::internal())?;
  let now = Utc::now().to_rfc3339();
  let metadata = SecretMetadata {
    key: key.clone(),
    version,
    created_at: created_at.clone(),
    updated_at: now.clone(),
    previous_version: None,
    previous_expires_at: None,
  };
  let action = if existing.is_some() {
    "secret.updated"
  } else {
    "secret.created"
  };
  // Overwriting a secret ends any rotation grace period: the stored value is
  // the only one that stays valid.
  sqlx::query("INSERT INTO secrets(environment_id,key,version,ciphertext,value_nonce,wrapped_key,key_nonce,created_at,updated_at)VALUES(?,?,?,?,?,?,?,?,?) ON CONFLICT(environment_id,key) DO UPDATE SET version=excluded.version,ciphertext=excluded.ciphertext,value_nonce=excluded.value_nonce,wrapped_key=excluded.wrapped_key,key_nonce=excluded.key_nonce,updated_at=excluded.updated_at,previous_version=NULL,previous_ciphertext=NULL,previous_value_nonce=NULL,previous_wrapped_key=NULL,previous_key_nonce=NULL,previous_expires_at=NULL").bind(id).bind(&key).bind(version).bind(encrypted.ciphertext).bind(encrypted.value_nonce).bind(encrypted.wrapped_key).bind(encrypted.key_nonce).bind(&created_at).bind(&now).execute(&mut *tx).await?;
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(admin_id),
    Some(email),
    action,
    Some(&env.project_id),
    Some(id),
    Some("secret"),
    Some(&key),
    serde_json::json!({"key":key,"version":version}),
  )
  .await?;
  tx.commit().await?;
  Ok(metadata)
}
/// Stores a new value in the free slot of the secret. The value that was
/// current stays readable until the grace period ends, so consumers that
/// still hold the old credential keep working while they pick up the new one.
pub async fn rotate(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
  key: String,
  request: RotateSecretRequest,
) -> Result<SecretMetadata, HttpError> {
  let input = SecretInput {
    key: key.clone(),
    value: request.value,
  };
  validate_entry(&input)?;
  let grace = validate_grace(request.grace_seconds)?;
  let (admin_id, email) = require_project_manager(identity)?;
  let env = environment(state, id).await?;
  let mut tx = state.db.pool().begin_with("BEGIN IMMEDIATE").await?;
  let existing = load(&mut tx, actor(identity), &env.project_id, id, &key)
    .await?
    .ok_or_else(|| {
      HttpError::not_found("SECRET_NOT_FOUND", "The requested secret was not found.")
    })?;
  let version = existing.version + 1;
  let encrypted = state
    .crypto
    .encrypt(input.value.as_bytes(), id, &key, version)
    .map_err(|_| HttpError::internal())?;
  let now = Utc::now();
  let expires_at = (!grace.is_zero()).then(|| (now + grace).to_rfc3339());
  let previous = expires_at.as_ref().map(|_| existing.encrypted());
  let previous_version = previous.as_ref().map(|_| existing.version);
  sqlx::query("UPDATE secrets SET version=?,ciphertext=?,value_nonce=?,wrapped_key=?,key_nonce=?,updated_at=?,previous_version=?,previous_ciphertext=?,previous_value_nonce=?,previous_wrapped_key=?,previous_key_nonce=?,previous_expires_at=? WHERE environment_id=? AND key=?")
    .bind(version).bind(&encrypted.ciphertext).bind(&encrypted.value_nonce).bind(&encrypted.wrapped_key).bind(&encrypted.key_nonce).bind(now.to_rfc3339())
    .bind(previous_version).bind(previous.as_ref().map(|value| &value.ciphertext)).bind(previous.as_ref().map(|value| &value.value_nonce)).bind(previous.as_ref().map(|value| &value.wrapped_key)).bind(previous.as_ref().map(|value| &value.key_nonce)).bind(&expires_at)
    .bind(id).bind(&key).execute(&mut *tx).await?;
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(admin_id),
    Some(email),
    "secret.rotated",
    Some(&env.project_id),
    Some(id),
    Some("secret"),
    Some(&key),
    serde_json::json!({"key":key,"version":version,"previousVersion":previous_version,"previousExpiresAt":expires_at}),
  )
  .await?;
  tx.commit().await?;
  Ok(SecretMetadata {
    key,
    version,
    created_at: existing.created_at,
    updated_at: now.to_rfc3339(),
    previous_version,
    previous_expires_at: expires_at,
  })
}
/// Makes the previous value current again. The two values exchange places and
/// keep the version each one was encrypted with, so no ciphertext changes.
/// The demoted value stays available until the same retire deadline.
pub async fn promote(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
  key: &str,
) -> Result<SecretMetadata, HttpError> {
  let (admin_id, email) = require_project_manager(identity)?;
  let env = environment(state, id).await?;
  let mut tx = state.db.pool().begin_with("BEGIN IMMEDIATE").await?;
  let existing = load(&mut tx, actor(identity), &env.project_id, id, key)
    .await?
    .ok_or_else(|| {
      HttpError::not_found("SECRET_NOT_FOUND", "The requested secret was not found.")
    })?;
  let now = Utc::now();
  let (previous_version, expires_at) = match existing.live_previous(&now) {
    Some(previous) => previous,
    None => {
      // Retiring an expired value must survive this failed promotion, so the
      // transaction holds only the retirement when it is committed here.
      tx.commit().await?;
      return Err(HttpError::not_found(
        "SECRET_PREVIOUS_NOT_FOUND",
        "The secret has no previous value to promote.",
      ));
    }
  };
  let demoted = existing.encrypted();
  let demoted_version = existing.version;
  sqlx::query("UPDATE secrets SET version=previous_version,ciphertext=previous_ciphertext,value_nonce=previous_value_nonce,wrapped_key=previous_wrapped_key,key_nonce=previous_key_nonce,previous_version=?,previous_ciphertext=?,previous_value_nonce=?,previous_wrapped_key=?,previous_key_nonce=?,updated_at=? WHERE environment_id=? AND key=?")
    .bind(demoted_version).bind(&demoted.ciphertext).bind(&demoted.value_nonce).bind(&demoted.wrapped_key).bind(&demoted.key_nonce).bind(now.to_rfc3339())
    .bind(id).bind(key).execute(&mut *tx).await?;
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(admin_id),
    Some(email),
    "secret.promoted",
    Some(&env.project_id),
    Some(id),
    Some("secret"),
    Some(key),
    serde_json::json!({"key":key,"version":previous_version,"demotedVersion":demoted_version,"expiresAt":expires_at}),
  )
  .await?;
  tx.commit().await?;
  Ok(SecretMetadata {
    key: key.into(),
    version: previous_version,
    created_at: existing.created_at,
    updated_at: now.to_rfc3339(),
    previous_version: Some(demoted_version),
    previous_expires_at: Some(expires_at),
  })
}
pub async fn delete(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
  key: &str,
) -> Result<(), HttpError> {
  let (admin_id, email) = require_project_manager(identity)?;
  let env = environment(state, id).await?;
  let mut tx = state.db.pool().begin_with("BEGIN IMMEDIATE").await?;
  let deleted = sqlx::query("DELETE FROM secrets WHERE environment_id=? AND key=?")
    .bind(id)
    .bind(key)
    .execute(&mut *tx)
    .await?;
  if deleted.rows_affected() != 1 {
    return Err(HttpError::not_found(
      "SECRET_NOT_FOUND",
      "The requested secret was not found.",
    ));
  }
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(admin_id),
    Some(email),
    "secret.deleted",
    Some(&env.project_id),
    Some(id),
    Some("secret"),
    Some(key),
    serde_json::json!({"key":key}),
  )
  .await?;
  tx.commit().await?;
  Ok(())
}
fn decrypt_value(
  state: &AppState,
  id: &str,
  key: &str,
  version: i64,
  encrypted: &EncryptedValue,
) -> Result<String, HttpError> {
  let clear = state
    .crypto
    .decrypt(encrypted, id, key, version)
    .map_err(|error| {
      tracing::error!(%error,"secret decryption failed");
      HttpError::internal()
    })?;
  String::from_utf8(clear.to_vec()).map_err(|_| HttpError::internal())
}
fn decrypt_row(
  state: &AppState,
  id: &str,
  row: &repository::SecretRow,
) -> Result<String, HttpError> {
  decrypt_value(state, id, &row.key, row.version, &row.encrypted())
}
pub async fn reveal(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
  key: &str,
  include_previous: bool,
) -> Result<RevealedSecret, HttpError> {
  require_recent_browser_auth(identity)?;
  let (admin_id, email) = require_project_manager(identity)?;
  let env = environment(state, id).await?;
  let mut conn = state.db.pool().acquire().await?;
  let row = load(&mut conn, actor(identity), &env.project_id, id, key)
    .await?
    .ok_or_else(|| {
      HttpError::not_found("SECRET_NOT_FOUND", "The requested secret was not found.")
    })?;
  let value = decrypt_row(state, id, &row)?;
  // Verification tooling can read both values while the grace period runs, so
  // a rotation can be checked before the old credential is retired.
  let previous = match (
    include_previous,
    row.live_previous(&Utc::now()),
    row.previous_encrypted(),
  ) {
    (true, Some((version, expires_at)), Some(encrypted)) => Some(PreviousRevealedValue {
      value: decrypt_value(state, id, key, version, &encrypted)?,
      version,
      expires_at,
    }),
    _ => None,
  };
  common::audit_standalone(
    state.db.pool(),
    identity.audit_actor_type(),
    Some(admin_id),
    Some(email),
    "secret.revealed",
    Some(&env.project_id),
    Some(id),
    Some("secret"),
    Some(key),
    serde_json::json!({"key":key,"previous":include_previous}),
  )
  .await?;
  Ok(RevealedSecret {
    key: key.into(),
    value,
    version: row.version,
    previous,
  })
}
fn validate_import(request: &ImportSecretsRequest) -> Result<(), HttpError> {
  if request.entries.len() > MAX_SECRETS_PER_ENVIRONMENT {
    return Err(HttpError::new(
      axum::http::StatusCode::UNPROCESSABLE_ENTITY,
      SECRET_LIMIT_EXCEEDED,
      SECRET_COUNT_LIMIT_MESSAGE,
    ));
  }
  if request
    .entries
    .iter()
    .map(|entry| entry.value.len())
    .sum::<usize>()
    > MAX_SECRET_COLLECTION_BYTES
  {
    return Err(HttpError::new(
      axum::http::StatusCode::UNPROCESSABLE_ENTITY,
      SECRET_LIMIT_EXCEEDED,
      SECRET_TOTAL_SIZE_LIMIT_MESSAGE,
    ));
  }
  let mut keys = HashSet::new();
  for entry in &request.entries {
    validate_entry(entry)?;
    if !keys.insert(&entry.key) {
      return Err(HttpError::new(
        axum::http::StatusCode::UNPROCESSABLE_ENTITY,
        IMPORT_DUPLICATE_KEY,
        "The import contains a duplicate secret key.",
      ));
    }
  }
  if let Some(layout) = &request.env_layout
    && layout.len() > MAX_ENV_LAYOUT_BYTES
  {
    return Err(HttpError::new(
      axum::http::StatusCode::UNPROCESSABLE_ENTITY,
      SECRET_LIMIT_EXCEEDED,
      "The .env layout may contain at most 64 KiB.",
    ));
  }
  if let Some(layout) = &request.env_layout {
    crate::utils::env_layout::validate(layout).map_err(|_| {
      HttpError::validation(BTreeMap::from([(
        "ENV_LAYOUT_INVALID".into(),
        "The .env layout must contain only comments, blank lines, and empty KEY= slots.".into(),
      )]))
    })?;
  }
  Ok(())
}
pub async fn import(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
  request: ImportSecretsRequest,
) -> Result<ImportSecretsResponse, HttpError> {
  require_recent_browser_auth(identity)?;
  validate_import(&request)?;
  let (admin_id, email) = require_project_manager(identity)?;
  let env = environment(state, id).await?;
  // Applying takes the SQLite write reservation before reading. This makes
  // the diff, version selection, replacement deletes, layout, and audit one
  // serializable state transition. Previews only write their audit event.
  let mut tx = if request.dry_run {
    state.db.pool().begin().await?
  } else {
    state.db.pool().begin_with("BEGIN IMMEDIATE").await?
  };
  let rows = if request.dry_run {
    repository::rows(&mut *tx, id).await?
  } else {
    load_all(&mut tx, actor(identity), &env.project_id, id).await?
  };
  let layout: Option<String> =
    sqlx::query_scalar("SELECT layout FROM environment_env_layout WHERE environment_id=?")
      .bind(id)
      .fetch_optional(&mut *tx)
      .await?;
  let revision = collection_revision(id, rows.iter(), layout.as_deref());
  let existing: HashMap<String, repository::SecretRow> =
    rows.into_iter().map(|row| (row.key.clone(), row)).collect();
  let incoming: HashMap<&str, &SecretInput> = request
    .entries
    .iter()
    .map(|entry| (entry.key.as_str(), entry))
    .collect();
  let mut added = Vec::new();
  let mut updated = Vec::new();
  let mut unchanged = Vec::new();
  for entry in &request.entries {
    match existing.get(&entry.key) {
      None => added.push(entry.key.clone()),
      Some(row) if decrypt_row(state, id, row)? == entry.value => unchanged.push(entry.key.clone()),
      Some(_) => updated.push(entry.key.clone()),
    }
  }
  let mut deleted = if request.mode == ImportMode::Replace {
    existing
      .keys()
      .filter(|key| !incoming.contains_key(key.as_str()))
      .cloned()
      .collect()
  } else {
    Vec::new()
  };
  added.sort();
  updated.sort();
  unchanged.sort();
  deleted.sort();
  if request
    .expected_revision
    .as_deref()
    .is_some_and(|expected| expected != revision)
  {
    return Err(HttpError::conflict(
      "IMPORT_PREVIEW_STALE",
      "The secrets changed after the preview. Run the dry run again.",
    ));
  }
  if request.dry_run {
    tx.rollback().await?;
    common::audit_standalone(
      state.db.pool(),
      identity.audit_actor_type(),
      Some(admin_id),
      Some(email),
      "secret.import_previewed",
      Some(&env.project_id),
      Some(id),
      Some("environment"),
      Some(id),
      serde_json::json!({
        "mode": match request.mode { ImportMode::Merge => "merge", ImportMode::Replace => "replace" },
        "entryCount": request.entries.len(),
      }),
    ).await?;
    return Ok(ImportSecretsResponse {
      added_keys: added,
      updated_keys: updated,
      unchanged_keys: unchanged,
      deleted_keys: deleted,
      dry_run: true,
      revision,
    });
  }
  if request.mode == ImportMode::Replace && request.expected_revision.is_none() {
    return Err(HttpError::conflict(
      "IMPORT_PREVIEW_REQUIRED",
      "Run a dry run before applying a replace import.",
    ));
  }
  let now = Utc::now().to_rfc3339();
  let mut encrypted = Vec::new();
  for entry in request
    .entries
    .iter()
    .filter(|entry| added.contains(&entry.key) || updated.contains(&entry.key))
  {
    let version = existing.get(&entry.key).map_or(1, |row| row.version + 1);
    encrypted.push((
      entry,
      version,
      state
        .crypto
        .encrypt(entry.value.as_bytes(), id, &entry.key, version)
        .map_err(|_| HttpError::internal())?,
    ));
  }
  // Importing an updated key overwrites it, so any rotation grace period for
  // that key ends with the import.
  for (entry, version, value) in encrypted {
    sqlx::query("INSERT INTO secrets(environment_id,key,version,ciphertext,value_nonce,wrapped_key,key_nonce,created_at,updated_at)VALUES(?,?,?,?,?,?,?,?,?) ON CONFLICT(environment_id,key)DO UPDATE SET version=excluded.version,ciphertext=excluded.ciphertext,value_nonce=excluded.value_nonce,wrapped_key=excluded.wrapped_key,key_nonce=excluded.key_nonce,updated_at=excluded.updated_at,previous_version=NULL,previous_ciphertext=NULL,previous_value_nonce=NULL,previous_wrapped_key=NULL,previous_key_nonce=NULL,previous_expires_at=NULL").bind(id).bind(&entry.key).bind(version).bind(value.ciphertext).bind(value.value_nonce).bind(value.wrapped_key).bind(value.key_nonce).bind(existing.get(&entry.key).map_or(&now,|row|&row.created_at)).bind(&now).execute(&mut *tx).await?;
  }
  for key in &deleted {
    sqlx::query("DELETE FROM secrets WHERE environment_id=? AND key=?")
      .bind(id)
      .bind(key)
      .execute(&mut *tx)
      .await?;
  }
  if let Some(env_layout) = &request.env_layout {
    repository::upsert_layout(&mut tx, id, env_layout, &now).await?;
  }
  let resulting_rows = repository::rows(&mut *tx, id).await?;
  let resulting_layout: Option<String> =
    sqlx::query_scalar("SELECT layout FROM environment_env_layout WHERE environment_id=?")
      .bind(id)
      .fetch_optional(&mut *tx)
      .await?;
  let revision = collection_revision(id, resulting_rows.iter(), resulting_layout.as_deref());
  common::audit(&mut tx,identity.audit_actor_type(),Some(admin_id),Some(email),"secret.imported",Some(&env.project_id),Some(id),Some("environment"),Some(id),serde_json::json!({"added":added,"updated":updated,"unchanged":unchanged,"deleted":deleted,"layout":request.env_layout.is_some()})).await?;
  tx.commit().await?;
  Ok(ImportSecretsResponse {
    added_keys: added,
    updated_keys: updated,
    unchanged_keys: unchanged,
    deleted_keys: deleted,
    dry_run: false,
    revision,
  })
}
fn collection_revision<'a>(
  environment_id: &str,
  rows: impl Iterator<Item = &'a repository::SecretRow>,
  layout: Option<&str>,
) -> String {
  let mut hash = Sha256::new();
  hash.update(environment_id.as_bytes());
  for row in rows {
    hash.update([0]);
    hash.update(row.key.as_bytes());
    hash.update(row.version.to_be_bytes());
    hash.update(row.updated_at.as_bytes());
  }
  if let Some(layout) = layout {
    hash.update([1]);
    hash.update(layout.as_bytes());
  }
  format!("{:x}", hash.finalize())
}
/// Returns the stored `.env` editor layout for the environment. Contains no
/// secret values, so no recent-authentication is required.
pub async fn layout(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
) -> Result<EnvLayoutResponse, HttpError> {
  require_metadata_access(identity)?;
  environment(state, id).await?;
  Ok(EnvLayoutResponse {
    layout: repository::layout(state.db.pool(), id).await?,
  })
}
pub async fn export(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
) -> Result<ExportSecretsResponse, HttpError> {
  let (actor_type, actor_id, actor_label) = match identity {
    AuthIdentity::Admin { .. } => {
      require_recent_browser_auth(identity)?;
      let (admin_id, email) = require_project_manager(identity)?;
      (identity.audit_actor_type(), admin_id, Some(email))
    }
    AuthIdentity::Runner {
      token_id,
      environment_id,
    } if environment_id == id => ("runner", token_id.as_str(), None),
    AuthIdentity::Runner { .. } => {
      return Err(HttpError::forbidden(
        TOKEN_SCOPE_INVALID,
        "The runner token cannot access this environment.",
      ));
    }
    AuthIdentity::ServiceAccount { .. } => {
      return Err(HttpError::forbidden(
        crate::constants::errors::AUTHORIZATION_DENIED,
        "AI agents may access secret metadata but never secret values.",
      ));
    }
  };
  let env = environment(state, id).await?;
  let mut tx = state.db.pool().begin().await?;
  let rows = load_all(
    &mut tx,
    (actor_type, Some(actor_id), actor_label),
    &env.project_id,
    id,
  )
  .await?;
  let env_layout: Option<String> =
    sqlx::query_scalar("SELECT layout FROM environment_env_layout WHERE environment_id=?")
      .bind(id)
      .fetch_optional(&mut *tx)
      .await?;
  let revision = collection_revision(id, rows.iter(), env_layout.as_deref());
  let entries = rows
    .iter()
    .map(|row| {
      decrypt_row(state, id, row).map(|value| SecretInput {
        key: row.key.clone(),
        value,
      })
    })
    .collect::<Result<Vec<_>, _>>()?;
  // Committed rather than rolled back: the snapshot holds no writes, so the
  // only thing a commit preserves is a retirement the read performed.
  tx.commit().await?;
  common::audit_standalone(
    state.db.pool(),
    actor_type,
    Some(actor_id),
    actor_label,
    "secret.exported",
    Some(&env.project_id),
    Some(id),
    Some("environment"),
    Some(id),
    serde_json::json!({"count":entries.len()}),
  )
  .await?;
  Ok(ExportSecretsResponse {
    entries,
    revision,
    env_layout,
  })
}
pub async fn runtime(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
) -> Result<RuntimeSecretsResponse, HttpError> {
  let env = environment(state, id).await?;
  let (actor_type, actor_id, actor_label) = match identity {
    AuthIdentity::Admin {
      admin_id, email, ..
    } => (
      identity.audit_actor_type(),
      admin_id.as_str(),
      Some(email.as_str()),
    ),
    AuthIdentity::Runner {
      token_id,
      environment_id,
    } if environment_id == id => ("runner", token_id.as_str(), None),
    AuthIdentity::Runner { .. } => {
      return Err(HttpError::forbidden(
        TOKEN_SCOPE_INVALID,
        "The runner token cannot access this environment.",
      ));
    }
    AuthIdentity::ServiceAccount { .. } => {
      return Err(HttpError::forbidden(
        crate::constants::errors::AUTHORIZATION_DENIED,
        "AI agents may access secret metadata but never secret values.",
      ));
    }
  };
  let mut conn = state.db.pool().acquire().await?;
  // Runtime reads keep returning the current value only. Consumers pick up a
  // rotation at their own pace by reading the same key again.
  let rows = load_all(
    &mut conn,
    (actor_type, Some(actor_id), actor_label),
    &env.project_id,
    id,
  )
  .await?;
  let entries = rows
    .into_iter()
    .map(|row| {
      decrypt_row(state, id, &row).map(|value| SecretInput {
        key: row.key,
        value,
      })
    })
    .collect::<Result<Vec<_>, _>>()?;
  common::audit_standalone(
    state.db.pool(),
    actor_type,
    Some(actor_id),
    actor_label,
    "secret.runtime_accessed",
    Some(&env.project_id),
    Some(id),
    Some("environment"),
    Some(id),
    serde_json::json!({"count":entries.len()}),
  )
  .await?;
  Ok(RuntimeSecretsResponse {
    project: env.project_name,
    environment: env.name,
    environment_id: env.id,
    entries,
  })
}
fn actor(identity: &AuthIdentity) -> Actor<'_> {
  match identity {
    AuthIdentity::Admin {
      admin_id, email, ..
    } => (
      identity.audit_actor_type(),
      Some(admin_id.as_str()),
      Some(email.as_str()),
    ),
    AuthIdentity::Runner { token_id, .. } => ("runner", Some(token_id.as_str()), None),
    AuthIdentity::ServiceAccount { token_id, name, .. } => (
      "service_account",
      Some(token_id.as_str()),
      Some(name.as_str()),
    ),
  }
}
