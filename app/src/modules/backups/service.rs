use std::{
  fs::{self, File},
  io::{Cursor, Write},
  path::PathBuf,
  sync::OnceLock,
};

use chrono::Utc;
use regex::Regex;
use sqlx::Connection;
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

use super::model::*;
use crate::{
  extractors::{require_admin, require_recent_browser_auth},
  http::HttpError,
  models::AuthIdentity,
  modules::common,
  state::AppState,
  utils::private_file,
};

const BACKUP_DIR_NAME: &str = "backups";
const BACKUP_EXTENSION: &str = ".dop";
const MANIFEST_MAGIC: &str = "DOPBASE_BACKUP_V1";
const MAX_BACKUP_FILE_BYTES: usize = 250 * 1024 * 1024; // 250 MB
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;

pub fn backup_dir(state: &AppState) -> PathBuf {
  let dir = state.config.data_dir.join(BACKUP_DIR_NAME);
  if !dir.exists() {
    let _ = fs::create_dir_all(&dir);
    #[cfg(unix)]
    {
      use std::os::unix::fs::PermissionsExt;
      let _ = fs::set_permissions(&dir, fs::Permissions::from_mode(0o700));
    }
  }
  dir
}

pub fn sanitize_key(key: &str) -> Result<String, HttpError> {
  static KEY_REGEX: OnceLock<Regex> = OnceLock::new();
  let regex = KEY_REGEX.get_or_init(|| Regex::new(r"^[a-zA-Z0-9_-]+\.dop$").unwrap());

  if !regex.is_match(key) || key.contains("..") || key.contains('/') || key.contains('\\') {
    return Err(HttpError::bad_request(
      "BACKUP_KEY_INVALID",
      "Backup key is invalid.",
    ));
  }
  Ok(key.to_string())
}

pub fn sanitize_name(name: &str) -> Result<String, HttpError> {
  static NAME_REGEX: OnceLock<Regex> = OnceLock::new();
  let regex = NAME_REGEX.get_or_init(|| Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap());

  let trimmed = name.trim();
  let base = trimmed.strip_suffix(BACKUP_EXTENSION).unwrap_or(trimmed);
  if base.is_empty() || base.len() > 64 || !regex.is_match(base) {
    return Err(HttpError::bad_request(
      "BACKUP_NAME_INVALID",
      "Backup name must only contain alphanumeric characters, dashes, and underscores.",
    ));
  }
  Ok(base.to_string())
}

pub async fn list(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<Vec<BackupItem>, HttpError> {
  require_admin(identity)?;
  let dir = backup_dir(state);
  let mut items = Vec::new();

  if let Ok(entries) = fs::read_dir(&dir) {
    for entry in entries.flatten() {
      let path = entry.path();
      if path.is_file()
        && path
          .extension()
          .and_then(|ext| ext.to_str())
          .is_some_and(|ext| ext == "dop")
      {
        let key = entry.file_name().to_string_lossy().to_string();
        if sanitize_key(&key).is_err() {
          continue;
        }
        if let Ok(meta) = entry.metadata() {
          let size = meta.len();
          let created_at = meta
            .modified()
            .or_else(|_| meta.created())
            .map(|t| chrono::DateTime::<Utc>::from(t).to_rfc3339())
            .unwrap_or_else(|_| Utc::now().to_rfc3339());
          items.push(BackupItem {
            key,
            size,
            created_at,
          });
        }
      }
    }
  }

  // Sort descending by created_at (newest first)
  items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
  Ok(items)
}

pub async fn create(
  state: &AppState,
  identity: &AuthIdentity,
  request: CreateBackupRequest,
) -> Result<BackupItem, HttpError> {
  let (admin_id, email) = require_admin(identity)?;
  let dir = backup_dir(state);

  let key = if let Some(raw_name) = request.name.filter(|n| !n.trim().is_empty()) {
    let base = sanitize_name(&raw_name)?;
    format!("{base}.dop")
  } else {
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    format!("dopbase_backup_{timestamp}.dop")
  };

  let file_path = dir.join(&key);
  if file_path.exists() {
    return Err(HttpError::conflict(
      "BACKUP_EXISTS",
      "A backup with this name already exists.",
    ));
  }

  // Snapshot database using SQLite VACUUM INTO
  let temp_dir = tempfile::tempdir().map_err(|_| HttpError::internal())?;
  let temp_db_path = temp_dir.path().join("snapshot.db");
  let escaped_path = temp_db_path.to_string_lossy().replace('\'', "''");

  sqlx::query(&format!("VACUUM INTO '{escaped_path}'"))
    .execute(state.db.pool())
    .await
    .map_err(|error| {
      tracing::error!(%error, "failed to vacuum sqlite database for backup");
      HttpError::internal()
    })?;

  let db_bytes = fs::read(&temp_db_path).map_err(|_| HttpError::internal())?;

  // Package snapshot and manifest into a compressed zip archive
  let mut zip_buffer = Cursor::new(Vec::new());
  {
    let mut zip = ZipWriter::new(&mut zip_buffer);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let manifest = BackupManifest {
      version: env!("CARGO_PKG_VERSION").into(),
      created_at: Utc::now().to_rfc3339(),
      backup_name: key.clone(),
      magic: MANIFEST_MAGIC.into(),
    };

    zip
      .start_file("manifest.json", options)
      .map_err(|_| HttpError::internal())?;
    zip
      .write_all(&serde_json::to_vec_pretty(&manifest).map_err(|_| HttpError::internal())?)
      .map_err(|_| HttpError::internal())?;

    zip
      .start_file("dopbase.db", options)
      .map_err(|_| HttpError::internal())?;
    zip
      .write_all(&db_bytes)
      .map_err(|_| HttpError::internal())?;

    zip.finish().map_err(|_| HttpError::internal())?;
  }

  let zip_data = zip_buffer.into_inner();

  // Encrypt with Server Master Key
  let encrypted = state
    .crypto
    .encrypt_backup(&zip_data)
    .map_err(|_| HttpError::internal())?;

  // Atomically write file
  private_file::write(&file_path, &encrypted, false).map_err(|error| {
    tracing::error!(%error, "failed to write backup file");
    HttpError::internal()
  })?;

  let size = encrypted.len() as u64;
  let created_at = Utc::now().to_rfc3339();

  // Audit
  let _ = common::audit(
    state.db.pool(),
    "admin",
    Some(admin_id),
    Some(email),
    "backup.created",
    None,
    None,
    Some("backup"),
    Some(&key),
    serde_json::json!({ "key": key, "size": size }),
  )
  .await;

  Ok(BackupItem {
    key,
    size,
    created_at,
  })
}

pub async fn read_for_download(
  state: &AppState,
  identity: &AuthIdentity,
  key: &str,
) -> Result<(PathBuf, u64), HttpError> {
  let (admin_id, email) = require_admin(identity)?;
  let safe_key = sanitize_key(key)?;
  let file_path = backup_dir(state).join(&safe_key);

  if !file_path.exists() {
    return Err(HttpError::not_found(
      "BACKUP_NOT_FOUND",
      "The requested backup was not found.",
    ));
  }

  let size = fs::metadata(&file_path)
    .map_err(|_| HttpError::internal())?
    .len();

  let _ = common::audit(
    state.db.pool(),
    "admin",
    Some(admin_id),
    Some(email),
    "backup.downloaded",
    None,
    None,
    Some("backup"),
    Some(&safe_key),
    serde_json::json!({ "key": safe_key, "size": size }),
  )
  .await;

  Ok((file_path, size))
}

pub async fn upload(
  state: &AppState,
  identity: &AuthIdentity,
  file_name: &str,
  bytes: Vec<u8>,
  provided_master_key: Option<&[u8]>,
) -> Result<BackupItem, HttpError> {
  let (admin_id, email) = require_admin(identity)?;

  if bytes.len() > MAX_BACKUP_FILE_BYTES {
    return Err(HttpError::bad_request(
      "BACKUP_FILE_TOO_LARGE",
      "Backup file exceeds the 250 MB limit.",
    ));
  }

  // 1. Try decrypting with current system master key first.
  let (decrypted, needs_rekey) = match state.crypto.decrypt_backup(&bytes) {
    Ok(data) => (data, false),
    Err(_) => {
      if let Some(key) = provided_master_key {
        let data = crate::services::crypto::CryptoService::decrypt_backup_with_key(&bytes, key)
          .map_err(|error| {
            tracing::warn!(%error, "uploaded backup verification failed with provided key");
            HttpError::bad_request(
              "BACKUP_INVALID",
              "Backup could not be decrypted or authenticated with the provided master key.",
            )
          })?;
        (data, true)
      } else {
        return Err(HttpError::bad_request(
          "BACKUP_KEY_REQUIRED",
          "This backup was created on a different server or with a different master key. Please provide the master key from that server to import it.",
        ));
      }
    }
  };

  // 2. Verify zip integrity and manifest
  let temp_dir = tempfile::tempdir().map_err(|_| HttpError::internal())?;
  let temp_db_path = temp_dir.path().join("dopbase.db");
  let mut manifest_bytes = Vec::new();
  {
    let mut archive = ZipArchive::new(Cursor::new(&decrypted)).map_err(|_| {
      HttpError::bad_request(
        "BACKUP_INVALID",
        "Backup archive payload is invalid or corrupted.",
      )
    })?;

    {
      let mut manifest_entry = archive.by_name("manifest.json").map_err(|_| {
        HttpError::bad_request("BACKUP_INVALID", "Backup archive is missing manifest.json.")
      })?;
      if manifest_entry.size() > MAX_MANIFEST_BYTES {
        return Err(HttpError::bad_request(
          "BACKUP_INVALID",
          "Backup manifest is too large.",
        ));
      }
      std::io::copy(&mut manifest_entry, &mut manifest_bytes).map_err(|_| HttpError::internal())?;
    }

    {
      let mut db_entry = archive.by_name("dopbase.db").map_err(|_| {
        HttpError::bad_request("BACKUP_INVALID", "Backup archive is missing dopbase.db.")
      })?;
      if db_entry.size() > MAX_BACKUP_FILE_BYTES as u64 {
        return Err(HttpError::bad_request(
          "BACKUP_FILE_TOO_LARGE",
          "Database payload exceeds the backup size limit.",
        ));
      }
      let mut db_file = File::create(&temp_db_path).map_err(|_| HttpError::internal())?;
      std::io::copy(&mut db_entry, &mut db_file).map_err(|_| HttpError::internal())?;
    }
  }

  let manifest: BackupManifest = serde_json::from_slice(&manifest_bytes)
    .map_err(|_| HttpError::bad_request("BACKUP_INVALID", "Backup manifest is invalid."))?;
  if manifest.magic != MANIFEST_MAGIC {
    return Err(HttpError::bad_request(
      "BACKUP_INVALID",
      "Backup manifest version is unsupported.",
    ));
  }

  // 3. If needs_rekey, re-key the database to the server's master key!
  let final_bytes = if needs_rekey {
    let restore_url = format!("sqlite://{}", temp_db_path.to_string_lossy());
    let temp_client = crate::services::db::DbClient::connect(&restore_url)
      .await
      .map_err(|e| {
        tracing::error!(%e, "failed to connect to uploaded sqlite db");
        HttpError::bad_request("BACKUP_INVALID_DB", "Invalid SQLite database in backup.")
      })?;

    temp_client.migrate().await.map_err(|_| {
      HttpError::bad_request(
        "BACKUP_INVALID_DB",
        "Backup database schema cannot be migrated.",
      )
    })?;
    let integrity: String = sqlx::query_scalar("PRAGMA quick_check")
      .fetch_one(temp_client.pool())
      .await
      .map_err(|_| HttpError::bad_request("BACKUP_CORRUPT", "Database integrity check failed."))?;
    if integrity != "ok" {
      return Err(HttpError::bad_request(
        "BACKUP_CORRUPT",
        "Database integrity check failed.",
      ));
    }

    crate::services::crypto::rekey_database(
      temp_client.pool(),
      provided_master_key.unwrap(),
      &state.crypto.master_key_bytes(),
    )
    .await
    .map_err(|e| {
      tracing::warn!(%e, "rekeying uploaded database failed");
      HttpError::bad_request(
        "BACKUP_REKEY_FAILED",
        &format!("Failed to re-key backup: {e}"),
      )
    })?;

    temp_client
      .checkpoint()
      .await
      .map_err(|_| HttpError::internal())?;
    temp_client.close().await;

    // Read re-keyed db bytes
    let rekeyed_db_bytes = fs::read(&temp_db_path).map_err(|_| HttpError::internal())?;

    // Re-package zip with manifest and rekeyed dopbase.db
    let mut zip_buffer = Cursor::new(Vec::new());
    {
      let mut zip = ZipWriter::new(&mut zip_buffer);
      let options =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
      zip
        .start_file("manifest.json", options)
        .map_err(|_| HttpError::internal())?;
      zip
        .write_all(&manifest_bytes)
        .map_err(|_| HttpError::internal())?;
      zip
        .start_file("dopbase.db", options)
        .map_err(|_| HttpError::internal())?;
      zip
        .write_all(&rekeyed_db_bytes)
        .map_err(|_| HttpError::internal())?;
      zip.finish().map_err(|_| HttpError::internal())?;
    }
    let zip_data = zip_buffer.into_inner();

    // Encrypt with Server Master Key!
    state
      .crypto
      .encrypt_backup(&zip_data)
      .map_err(|_| HttpError::internal())?
  } else {
    let temp_client = crate::services::db::DbClient::connect(&format!(
      "sqlite://{}",
      temp_db_path.to_string_lossy()
    ))
    .await
    .map_err(|_| {
      HttpError::bad_request("BACKUP_INVALID_DB", "Invalid SQLite database in backup.")
    })?;
    temp_client.migrate().await.map_err(|_| {
      HttpError::bad_request(
        "BACKUP_INVALID_DB",
        "Backup database schema cannot be migrated.",
      )
    })?;
    let integrity: String = sqlx::query_scalar("PRAGMA quick_check")
      .fetch_one(temp_client.pool())
      .await
      .map_err(|_| HttpError::bad_request("BACKUP_CORRUPT", "Database integrity check failed."))?;
    temp_client.close().await;
    if integrity != "ok" {
      return Err(HttpError::bad_request(
        "BACKUP_CORRUPT",
        "Database integrity check failed.",
      ));
    }
    bytes
  };

  let dir = backup_dir(state);
  let base_name = file_name
    .strip_suffix(BACKUP_EXTENSION)
    .unwrap_or(file_name);
  let safe_base = sanitize_name(base_name).unwrap_or_else(|_| {
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    format!("uploaded_backup_{timestamp}")
  });

  let mut target_key = format!("{safe_base}.dop");
  let mut counter = 1;
  while dir.join(&target_key).exists() {
    target_key = format!("{safe_base}_{counter}.dop");
    counter += 1;
  }

  let target_path = dir.join(&target_key);
  private_file::write(&target_path, &final_bytes, false).map_err(|error| {
    tracing::error!(%error, "failed to save uploaded backup");
    HttpError::internal()
  })?;

  let size = final_bytes.len() as u64;
  let created_at = Utc::now().to_rfc3339();

  let _ = common::audit(
    state.db.pool(),
    "admin",
    Some(admin_id),
    Some(email),
    "backup.uploaded",
    None,
    None,
    Some("backup"),
    Some(&target_key),
    serde_json::json!({ "key": target_key, "size": size, "rekeyed": needs_rekey }),
  )
  .await;

  Ok(BackupItem {
    key: target_key,
    size,
    created_at,
  })
}

pub async fn restore(
  state: &AppState,
  identity: &AuthIdentity,
  key: &str,
  provided_master_key: Option<&[u8]>,
) -> Result<(), HttpError> {
  require_recent_browser_auth(identity)?;
  let (admin_id, email) = require_admin(identity)?;
  let safe_key = sanitize_key(key)?;
  let file_path = backup_dir(state).join(&safe_key);

  if !file_path.exists() {
    return Err(HttpError::not_found(
      "BACKUP_NOT_FOUND",
      "The requested backup was not found.",
    ));
  }

  let encrypted = fs::read(&file_path).map_err(|_| HttpError::internal())?;

  // Decrypt backup: try system master key first
  let (decrypted, source_key_to_rekey) = match state.crypto.decrypt_backup(&encrypted) {
    Ok(data) => (data, None),
    Err(_) => {
      if let Some(k) = provided_master_key {
        let data = crate::services::crypto::CryptoService::decrypt_backup_with_key(&encrypted, k)
          .map_err(|_| {
          HttpError::bad_request(
            "BACKUP_DECRYPT_FAILED",
            "Failed to decrypt backup with the provided master key.",
          )
        })?;
        (data, Some(k))
      } else {
        return Err(HttpError::bad_request(
          "BACKUP_DECRYPT_FAILED",
          "Failed to decrypt backup. Ensure you provided the correct master key for this backup.",
        ));
      }
    }
  };

  restore_database_from_archive(
    state,
    decrypted,
    source_key_to_rekey,
    Some((admin_id, email, session_id_from_identity(identity))),
    Some(&safe_key),
  )
  .await
}

pub async fn restore_database_from_archive(
  state: &AppState,
  decrypted: Vec<u8>,
  source_master_key_to_rekey: Option<&[u8]>,
  preserve_admin: Option<(&str, &str, &str)>,
  audit_key: Option<&str>,
) -> Result<(), HttpError> {
  let temp_dir = tempfile::tempdir().map_err(|_| HttpError::internal())?;
  let temp_db_path = temp_dir.path().join("restore.db");
  let mut manifest_bytes = Vec::new();
  {
    let mut archive = ZipArchive::new(Cursor::new(decrypted))
      .map_err(|_| HttpError::bad_request("BACKUP_CORRUPT", "Corrupted archive."))?;

    {
      let mut manifest_entry = archive
        .by_name("manifest.json")
        .map_err(|_| HttpError::bad_request("BACKUP_CORRUPT", "Missing manifest in backup."))?;
      if manifest_entry.size() > MAX_MANIFEST_BYTES {
        return Err(HttpError::bad_request(
          "BACKUP_CORRUPT",
          "Backup manifest is too large.",
        ));
      }
      std::io::copy(&mut manifest_entry, &mut manifest_bytes).map_err(|_| HttpError::internal())?;
    }
    let mut db_entry = archive
      .by_name("dopbase.db")
      .map_err(|_| HttpError::bad_request("BACKUP_CORRUPT", "Missing database in backup."))?;
    if db_entry.size() > MAX_BACKUP_FILE_BYTES as u64 {
      return Err(HttpError::bad_request(
        "BACKUP_FILE_TOO_LARGE",
        "Database payload exceeds the backup size limit.",
      ));
    }

    let mut temp_file = File::create(&temp_db_path).map_err(|_| HttpError::internal())?;
    std::io::copy(&mut db_entry, &mut temp_file).map_err(|_| HttpError::internal())?;
  }
  let manifest: BackupManifest = serde_json::from_slice(&manifest_bytes)
    .map_err(|_| HttpError::bad_request("BACKUP_CORRUPT", "Backup manifest is invalid."))?;
  if manifest.magic != MANIFEST_MAGIC {
    return Err(HttpError::bad_request(
      "BACKUP_CORRUPT",
      "Backup manifest version is unsupported.",
    ));
  }

  // Validate database integrity
  let restore_url = format!("sqlite://{}", temp_db_path.to_string_lossy());
  let restore_client = crate::services::db::DbClient::connect(&restore_url)
    .await
    .map_err(|_| HttpError::bad_request("BACKUP_INVALID_DB", "Invalid SQLite database."))?;

  restore_client.migrate().await.map_err(|_| {
    HttpError::bad_request(
      "BACKUP_INVALID_DB",
      "Backup database schema cannot be migrated.",
    )
  })?;

  let integrity: String = sqlx::query_scalar("PRAGMA quick_check")
    .fetch_one(restore_client.pool())
    .await
    .map_err(|_| HttpError::bad_request("BACKUP_CORRUPT", "Database integrity check failed."))?;

  if integrity != "ok" {
    return Err(HttpError::bad_request(
      "BACKUP_CORRUPT",
      "Database integrity check failed.",
    ));
  }

  // If source_master_key_to_rekey is provided, re-key the database to this server's master key!
  if let Some(old_key) = source_master_key_to_rekey {
    crate::services::crypto::rekey_database(
      restore_client.pool(),
      old_key,
      &state.crypto.master_key_bytes(),
    )
    .await
    .map_err(|e| {
      tracing::warn!(%e, "rekeying database during restore failed");
      HttpError::bad_request(
        "BACKUP_REKEY_FAILED",
        &format!("Failed to re-key backup: {e}"),
      )
    })?;
  }

  restore_client
    .checkpoint()
    .await
    .map_err(|_| HttpError::internal())?;
  restore_client.close().await;

  // Restore atomically using ATTACH DATABASE on a disposable connection pool.
  // If the request is cancelled, dropping this pool prevents connection-level
  // ATTACH state from being returned to the live request pool.
  let restore_db = crate::services::db::DbClient::connect(&state.config.database_url)
    .await
    .map_err(|_| HttpError::internal())?;
  let mut conn = restore_db
    .pool()
    .acquire()
    .await
    .map_err(|_| HttpError::internal())?;

  let escaped_attach = temp_db_path.to_string_lossy().replace('\'', "''");
  sqlx::query(&format!("ATTACH DATABASE '{escaped_attach}' AS backup_db"))
    .execute(&mut *conn)
    .await
    .map_err(|error| {
      tracing::error!(%error, "failed to attach backup database");
      HttpError::internal()
    })?;

  let restore_result = async {
    let mut tx = conn.begin().await?;

    // Capture current active admin and session to preserve login if requested
    type AdminRowTuple = (String, String, String, String, String);
    type SessionRowTuple = (
      String,
      String,
      String,
      Vec<u8>,
      Option<Vec<u8>>,
      String,
      String,
      String,
      String,
      String,
    );

    let mut current_admin: Option<AdminRowTuple> = None;
    let mut current_session: Option<SessionRowTuple> = None;

    if let Some((admin_id, _, session_id)) = preserve_admin {
      current_admin = sqlx::query_as(
        "SELECT id, email, password_hash, created_at, updated_at FROM admins WHERE id = ?",
      )
      .bind(admin_id)
      .fetch_optional(&mut *tx)
      .await?;

      current_session = sqlx::query_as(
        "SELECT id, admin_id, kind, token_hash, csrf_hash, created_at, last_used_at, recent_auth_at, idle_expires_at, absolute_expires_at FROM sessions WHERE id = ? AND admin_id = ? AND revoked_at IS NULL"
      )
      .bind(session_id)
      .bind(admin_id)
      .fetch_optional(&mut *tx)
      .await?;
    }

    // Wipe existing data
    sqlx::query("DELETE FROM audit_events").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM runner_tokens").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM secrets").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM environment_env_layout").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM environments").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM projects").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM sessions").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM admins").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM instance_metadata").execute(&mut *tx).await?;

    // Copy snapshot data from backup_db
    sqlx::query("INSERT INTO instance_metadata SELECT * FROM backup_db.instance_metadata").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO admins SELECT * FROM backup_db.admins").execute(&mut *tx).await?;
    sqlx::query("INSERT OR IGNORE INTO sessions SELECT * FROM backup_db.sessions WHERE admin_id IN (SELECT id FROM admins)").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO projects SELECT * FROM backup_db.projects").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO environments SELECT * FROM backup_db.environments").execute(&mut *tx).await?;
    sqlx::query("INSERT OR REPLACE INTO environment_env_layout SELECT * FROM backup_db.environment_env_layout").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO secrets SELECT * FROM backup_db.secrets").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO runner_tokens SELECT * FROM backup_db.runner_tokens").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO audit_events SELECT * FROM backup_db.audit_events").execute(&mut *tx).await?;

    let mut effective_admin_id = preserve_admin.map(|(id, _, _)| id.to_string());

    // Re-preserve current admin and session if requested
    if let (Some(admin), Some(mut sess)) = (current_admin, current_session) {
      // Check if an admin with the same email already exists in the restored database
      let restored_admin_with_same_email: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM admins WHERE email = ?",
      )
      .bind(&admin.1)
      .fetch_optional(&mut *tx)
      .await?;

      if let Some((target_admin_id,)) = restored_admin_with_same_email {
        sess.1 = target_admin_id.clone();
        effective_admin_id = Some(target_admin_id);
      } else {
        sqlx::query("INSERT OR IGNORE INTO admins(id, email, password_hash, created_at, updated_at) VALUES(?, ?, ?, ?, ?)")
          .bind(&admin.0).bind(&admin.1).bind(&admin.2).bind(&admin.3).bind(&admin.4)
          .execute(&mut *tx).await?;
        let actual_admin: (String,) = sqlx::query_as("SELECT id FROM admins LIMIT 1")
          .fetch_one(&mut *tx)
          .await?;
        sess.1 = actual_admin.0.clone();
        effective_admin_id = Some(actual_admin.0);
      }

      sqlx::query("INSERT OR REPLACE INTO sessions(id, admin_id, kind, token_hash, csrf_hash, created_at, last_used_at, recent_auth_at, idle_expires_at, absolute_expires_at) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(sess.0).bind(sess.1).bind(sess.2).bind(sess.3).bind(sess.4).bind(sess.5).bind(sess.6).bind(sess.7).bind(sess.8).bind(sess.9)
        .execute(&mut *tx).await?;
    }

    if let Some((_, email, _)) = preserve_admin {
      let key_str = audit_key.unwrap_or("unknown");
      common::audit(
        &mut *tx,
        "admin",
        effective_admin_id.as_deref(),
        Some(email),
        "backup.restored",
        None,
        None,
        Some("backup"),
        Some(key_str),
        serde_json::json!({ "backup": key_str }),
      )
      .await?;
    }

    tx.commit().await?;
    Ok::<(), sqlx::Error>(())
  }
  .await;

  let _ = sqlx::query("DETACH DATABASE backup_db")
    .execute(&mut *conn)
    .await;

  drop(conn);
  restore_db.close().await;
  restore_result.map_err(|error| {
    tracing::error!(%error, "failed to apply backup restore");
    HttpError::internal()
  })?;

  // Run migrations on restored database if needed
  Ok(())
}

fn session_id_from_identity(identity: &AuthIdentity) -> &str {
  match identity {
    AuthIdentity::Admin { session_id, .. } => session_id,
    AuthIdentity::Runner { .. } => "",
  }
}

pub async fn delete(
  state: &AppState,
  identity: &AuthIdentity,
  key: &str,
) -> Result<(), HttpError> {
  let (admin_id, email) = require_admin(identity)?;
  let safe_key = sanitize_key(key)?;
  let file_path = backup_dir(state).join(&safe_key);

  if !file_path.exists() {
    return Err(HttpError::not_found(
      "BACKUP_NOT_FOUND",
      "The requested backup was not found.",
    ));
  }

  fs::remove_file(&file_path).map_err(|error| {
    tracing::error!(%error, "failed to delete backup file");
    HttpError::internal()
  })?;

  let _ = common::audit(
    state.db.pool(),
    "admin",
    Some(admin_id),
    Some(email),
    "backup.deleted",
    None,
    None,
    Some("backup"),
    Some(&safe_key),
    serde_json::json!({ "key": safe_key }),
  )
  .await;

  Ok(())
}
