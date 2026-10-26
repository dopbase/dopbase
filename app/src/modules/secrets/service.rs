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
  extractors::{require_account, require_metadata_access, require_recent_browser_auth},
  http::HttpError,
  models::{AuthIdentity, SecretInput},
  services::authx::Level,
  state::AppState,
};
use chrono::Utc;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::OnceLock;
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
async fn environment(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
  needed: Level,
) -> Result<crate::modules::environments::model::EnvironmentResponse, HttpError> {
  crate::modules::environments::service::authorize(state, identity, id, needed).await
}
pub async fn list(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
) -> Result<Vec<SecretMetadata>, HttpError> {
  require_metadata_access(identity)?;
  environment(state, identity, id, Level::Metadata).await?;
  Ok(repository::list(state.db.pool(), id).await?)
}
pub async fn get(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
  key: &str,
) -> Result<SecretMetadata, HttpError> {
  require_metadata_access(identity)?;
  environment(state, identity, id, Level::Metadata).await?;
  repository::find(state.db.pool(), id, key)
    .await?
    .map(|row| row.metadata())
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
  let (admin_id, email) = require_account(identity)?;
  let env = environment(state, identity, id, Level::Write).await?;
  let mut tx = state.db.pool().begin_with("BEGIN IMMEDIATE").await?;
  let existing: Option<repository::SecretRow> = sqlx::query_as("SELECT key,version,ciphertext,value_nonce,wrapped_key,key_nonce,created_at,updated_at FROM secrets WHERE environment_id=? AND key=?")
    .bind(id).bind(&key).fetch_optional(&mut *tx).await?;
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
  };
  let action = if existing.is_some() {
    "secret.updated"
  } else {
    "secret.created"
  };
  repository::write_version(
    &mut tx,
    repository::NewVersion {
      environment_id: id,
      key: &key,
      version,
      encrypted,
      created_at: &created_at,
      now: &now,
      change: if existing.is_some() {
        repository::ChangeType::Updated
      } else {
        repository::ChangeType::Created
      },
      source_version: None,
      actor: repository::VersionActor {
        actor_type: identity.audit_actor_type(),
        actor_id: Some(admin_id),
        actor_label: Some(email),
      },
      keep: state.config.secret_history_versions,
    },
  )
  .await?;
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
pub async fn delete(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
  key: &str,
) -> Result<(), HttpError> {
  let (admin_id, email) = require_account(identity)?;
  let env = environment(state, identity, id, Level::Write).await?;
  let mut tx = state.db.pool().begin_with("BEGIN IMMEDIATE").await?;
  // The version history goes with it through the foreign key cascade.
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
fn decrypt_row(
  state: &AppState,
  id: &str,
  row: &repository::SecretRow,
) -> Result<String, HttpError> {
  let clear = state
    .crypto
    .decrypt(&row.encrypted(), id, &row.key, row.version)
    .map_err(|error| {
      tracing::error!(%error,"secret decryption failed");
      HttpError::internal()
    })?;
  String::from_utf8(clear.to_vec()).map_err(|_| HttpError::internal())
}
pub async fn reveal(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
  key: &str,
) -> Result<RevealedSecret, HttpError> {
  require_recent_browser_auth(identity)?;
  let (admin_id, email) = require_account(identity)?;
  let env = environment(state, identity, id, Level::Read).await?;
  let row = repository::find(state.db.pool(), id, key)
    .await?
    .ok_or_else(|| {
      HttpError::not_found("SECRET_NOT_FOUND", "The requested secret was not found.")
    })?;
  let value = decrypt_row(state, id, &row)?;
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
    serde_json::json!({"key":key}),
  )
  .await?;
  Ok(RevealedSecret {
    key: key.into(),
    value,
    version: row.version,
  })
}
fn secret_not_found() -> HttpError {
  HttpError::not_found("SECRET_NOT_FOUND", "The requested secret was not found.")
}
fn version_not_found() -> HttpError {
  HttpError::not_found(
    "SECRET_VERSION_NOT_FOUND",
    "That version of the secret was not found.",
  )
}
fn version_expired() -> HttpError {
  HttpError::new(
    axum::http::StatusCode::GONE,
    "SECRET_VERSION_EXPIRED",
    "That version is past the retention limit, so its value is no longer stored.",
  )
}
/// Decrypts one stored version. Fails closed: a version without ciphertext
/// is expired, and ciphertext that does not authenticate is an internal
/// error that writes nothing.
fn decrypt_version(
  state: &AppState,
  id: &str,
  key: &str,
  stored: &repository::StoredVersion,
) -> Result<zeroize::Zeroizing<String>, HttpError> {
  let encrypted = stored.encrypted().ok_or_else(version_expired)?;
  let clear = state
    .crypto
    .decrypt(&encrypted, id, key, stored.version)
    .map_err(|error| {
      tracing::error!(%error, version = stored.version, "secret version decryption failed");
      HttpError::internal()
    })?;
  String::from_utf8(clear.to_vec())
    .map(zeroize::Zeroizing::new)
    .map_err(|_| HttpError::internal())
}
pub async fn versions(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
  key: &str,
  query: SecretVersionQuery,
) -> Result<SecretVersionPage, HttpError> {
  require_metadata_access(identity)?;
  environment(state, identity, id, Level::Metadata).await?;
  let current = repository::find(state.db.pool(), id, key)
    .await?
    .ok_or_else(secret_not_found)?;
  let limit = query.limit.unwrap_or(50).clamp(1, 200);
  let mut rows =
    repository::list_versions(state.db.pool(), id, key, query.before, limit + 1).await?;
  let next_before = if rows.len() > limit as usize {
    rows.truncate(limit as usize);
    rows.last().map(|row| row.version)
  } else {
    None
  };
  Ok(SecretVersionPage {
    items: rows
      .into_iter()
      .map(|row| SecretVersion {
        current: row.version == current.version,
        restorable: row.expired_at.is_none(),
        version: row.version,
        change_type: row.change_type,
        source_version: row.source_version,
        actor_type: row.actor_type,
        actor_label: row.actor_label,
        created_at: row.created_at,
      })
      .collect(),
    next_before,
    retained_versions: state.config.secret_history_versions,
  })
}
pub async fn reveal_version(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
  key: &str,
  version: i64,
) -> Result<RevealedSecret, HttpError> {
  require_recent_browser_auth(identity)?;
  let (admin_id, email) = require_account(identity)?;
  let env = environment(state, identity, id, Level::Read).await?;
  let mut conn = state.db.pool().acquire().await?;
  let stored = repository::find_version(&mut conn, id, key, version)
    .await?
    .ok_or_else(version_not_found)?;
  drop(conn);
  let value = decrypt_version(state, id, key, &stored)?;
  common::audit_standalone(
    state.db.pool(),
    identity.audit_actor_type(),
    Some(admin_id),
    Some(email),
    "secret.version_revealed",
    Some(&env.project_id),
    Some(id),
    Some("secret"),
    Some(key),
    serde_json::json!({"key":key,"version":version}),
  )
  .await?;
  Ok(RevealedSecret {
    key: key.into(),
    value: value.to_string(),
    version,
  })
}
/// Saves the value of an earlier version as a new current version. Earlier
/// versions are never changed.
pub async fn rollback(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
  key: &str,
  request: RollbackSecretRequest,
) -> Result<RollbackSecretResponse, HttpError> {
  require_recent_browser_auth(identity)?;
  let (admin_id, email) = require_account(identity)?;
  let env = environment(state, identity, id, Level::Write).await?;
  // The write reservation is taken before reading, so the version check,
  // the new version number, and the write are one state transition.
  let mut tx = state.db.pool().begin_with("BEGIN IMMEDIATE").await?;
  let current: repository::SecretRow = sqlx::query_as("SELECT key,version,ciphertext,value_nonce,wrapped_key,key_nonce,created_at,updated_at FROM secrets WHERE environment_id=? AND key=?")
    .bind(id).bind(key).fetch_optional(&mut *tx).await?
    .ok_or_else(secret_not_found)?;
  if request
    .expected_version
    .is_some_and(|expected| expected != current.version)
  {
    return Err(HttpError::conflict(
      "SECRET_VERSION_CONFLICT",
      "The secret changed since you loaded it. Reload its history and try again.",
    ));
  }
  if request.version == current.version {
    return Err(HttpError::conflict(
      "SECRET_VERSION_IS_CURRENT",
      "That version is already the current value.",
    ));
  }
  let stored = repository::find_version(&mut tx, id, key, request.version)
    .await?
    .ok_or_else(version_not_found)?;
  let value = decrypt_version(state, id, key, &stored)?;
  let version = current.version + 1;
  let encrypted = state
    .crypto
    .encrypt(value.as_bytes(), id, key, version)
    .map_err(|_| HttpError::internal())?;
  let now = Utc::now().to_rfc3339();
  repository::write_version(
    &mut tx,
    repository::NewVersion {
      environment_id: id,
      key,
      version,
      encrypted,
      created_at: &current.created_at,
      now: &now,
      change: repository::ChangeType::RolledBack,
      source_version: Some(request.version),
      actor: repository::VersionActor {
        actor_type: identity.audit_actor_type(),
        actor_id: Some(admin_id),
        actor_label: Some(email),
      },
      keep: state.config.secret_history_versions,
    },
  )
  .await?;
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(admin_id),
    Some(email),
    "secret.rolled_back",
    Some(&env.project_id),
    Some(id),
    Some("secret"),
    Some(key),
    serde_json::json!({
      "key": key,
      "sourceVersion": request.version,
      "previousVersion": current.version,
      "version": version,
    }),
  )
  .await?;
  tx.commit().await?;
  Ok(RollbackSecretResponse {
    key: key.into(),
    version,
    source_version: request.version,
    previous_version: current.version,
    created_at: current.created_at,
    updated_at: now,
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
  let (admin_id, email) = require_account(identity)?;
  let env = environment(state, identity, id, Level::Write).await?;
  // Applying takes the SQLite write reservation before reading. This makes
  // the diff, version selection, replacement deletes, layout, and audit one
  // serializable state transition. Previews only write their audit event.
  let mut tx = if request.dry_run {
    state.db.pool().begin().await?
  } else {
    state.db.pool().begin_with("BEGIN IMMEDIATE").await?
  };
  let rows: Vec<repository::SecretRow> = sqlx::query_as("SELECT key,version,ciphertext,value_nonce,wrapped_key,key_nonce,created_at,updated_at FROM secrets WHERE environment_id=? ORDER BY key")
    .bind(id).fetch_all(&mut *tx).await?;
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
  for (entry, version, value) in encrypted {
    repository::write_version(
      &mut tx,
      repository::NewVersion {
        environment_id: id,
        key: &entry.key,
        version,
        encrypted: value,
        created_at: existing
          .get(&entry.key)
          .map_or(now.as_str(), |row| row.created_at.as_str()),
        now: &now,
        change: repository::ChangeType::Imported,
        source_version: None,
        actor: repository::VersionActor {
          actor_type: identity.audit_actor_type(),
          actor_id: Some(admin_id),
          actor_label: Some(email),
        },
        keep: state.config.secret_history_versions,
      },
    )
    .await?;
  }
  // Deleting a secret cascades to its version history.
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
  let resulting_rows: Vec<repository::SecretRow> = sqlx::query_as("SELECT key,version,ciphertext,value_nonce,wrapped_key,key_nonce,created_at,updated_at FROM secrets WHERE environment_id=? ORDER BY key")
    .bind(id).fetch_all(&mut *tx).await?;
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
async fn decrypt_all(
  state: &AppState,
  id: &str,
) -> Result<Vec<SecretInput>, HttpError> {
  repository::rows(state.db.pool(), id)
    .await?
    .into_iter()
    .map(|row| {
      decrypt_row(state, id, &row).map(|value| SecretInput {
        key: row.key,
        value,
      })
    })
    .collect()
}
/// Returns the stored `.env` editor layout for the environment. Contains no
/// secret values, so no recent-authentication is required.
pub async fn layout(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
) -> Result<EnvLayoutResponse, HttpError> {
  require_metadata_access(identity)?;
  environment(state, identity, id, Level::Metadata).await?;
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
      let (admin_id, email) = require_account(identity)?;
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
    AuthIdentity::Token {
      admin_id, email, ..
    } => (
      identity.audit_actor_type(),
      admin_id.as_str(),
      Some(email.as_str()),
    ),
  };
  let env = environment(state, identity, id, Level::Read).await?;
  let mut tx = state.db.pool().begin().await?;
  let rows: Vec<repository::SecretRow> = sqlx::query_as("SELECT key,version,ciphertext,value_nonce,wrapped_key,key_nonce,created_at,updated_at FROM secrets WHERE environment_id=? ORDER BY key")
    .bind(id).fetch_all(&mut *tx).await?;
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
  tx.rollback().await?;
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
  crate::modules::environments::service::show(state, id).await?;
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
    AuthIdentity::Token {
      admin_id, email, ..
    } => (
      identity.audit_actor_type(),
      admin_id.as_str(),
      Some(email.as_str()),
    ),
  };
  let env = environment(state, identity, id, Level::Read).await?;
  let entries = decrypt_all(state, id).await?;
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
