use std::{
  fs::File,
  io::{Cursor, Write},
};

use chrono::{Duration, Utc};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

use super::{model::*, repository};
use crate::modules::common;
use crate::{
  constants::{
    errors::RATE_LIMITED,
    limits::{BROWSER_SESSION_ABSOLUTE_HOURS, BROWSER_SESSION_IDLE_HOURS},
    tokens::{ADMIN_ID_PREFIX, ADMIN_SESSION_PREFIX, CSRF_TOKEN_PREFIX, SESSION_ID_PREFIX},
  },
  http::HttpError,
  services::token,
  state::AppState,
};

const MAX_BACKUP_FILE_BYTES: u64 = 250 * 1024 * 1024;
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
pub async fn status(state: &AppState) -> Result<BootstrapStatus, HttpError> {
  let count = repository::admin_count(state.db.pool()).await?;
  Ok(BootstrapStatus {
    state: if count == 0 { "setupRequired" } else { "ready" },
  })
}
pub struct CreatedAdmin {
  pub response: BootstrapAdminResponse,
  pub session_token: String,
}
pub async fn create(
  state: &AppState,
  request: BootstrapAdminRequest,
) -> Result<CreatedAdmin, HttpError> {
  if !state.rate_limiter.check("bootstrap").await {
    return Err(HttpError::new(
      axum::http::StatusCode::TOO_MANY_REQUESTS,
      RATE_LIMITED,
      "Too many setup attempts. Please try again later.",
    ));
  }
  let email = common::validate_email(&request.email)?;
  common::validate_password(&request.password)?;
  let mut setup = state.setup.write().await;
  let expected = setup.token.clone().ok_or_else(|| {
    HttpError::conflict(
      "BOOTSTRAP_CLOSED",
      "This instance has already been initialized.",
    )
  })?;
  if !token::constant_time_eq(&request.setup_token, &expected) {
    state.rate_limiter.failure("bootstrap").await;
    return Err(HttpError::unauthorized(
      "SETUP_TOKEN_INVALID",
      "The setup token is invalid.",
    ));
  }
  let hash = common::hash_password_async(request.password.clone()).await?;
  let admin_id = token::public_id(ADMIN_ID_PREFIX);
  let session_id = token::public_id(SESSION_ID_PREFIX);
  let session_token = token::generate(ADMIN_SESSION_PREFIX).map_err(|_| HttpError::internal())?;
  let csrf = token::generate(CSRF_TOKEN_PREFIX).map_err(|_| HttpError::internal())?;
  let now = Utc::now();
  let mut tx = state.db.pool().begin().await?;
  let admin_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admins")
    .fetch_one(&mut *tx)
    .await?;
  if admin_count > 0 {
    return Err(HttpError::conflict(
      "BOOTSTRAP_CLOSED",
      "This instance has already been initialized.",
    ));
  }
  repository::insert_admin(&mut tx, &admin_id, &email, &hash, &now.to_rfc3339()).await?;
  repository::insert_session(
    &mut tx,
    &session_id,
    &admin_id,
    &token::hash(&session_token),
    &token::hash(&csrf),
    &now.to_rfc3339(),
    &(now + Duration::hours(BROWSER_SESSION_IDLE_HOURS)).to_rfc3339(),
    &(now + Duration::hours(BROWSER_SESSION_ABSOLUTE_HOURS)).to_rfc3339(),
  )
  .await?;
  common::audit(
    &mut *tx,
    "admin",
    Some(&admin_id),
    Some(&email),
    "admin.bootstrapped",
    None,
    None,
    Some("admin"),
    Some(&admin_id),
    serde_json::json!({}),
  )
  .await?;
  tx.commit().await?;
  setup.token = None;
  state.rate_limiter.clear("bootstrap").await;
  Ok(CreatedAdmin {
    response: BootstrapAdminResponse {
      admin_id,
      email,
      csrf_token: csrf,
    },
    session_token,
  })
}

pub async fn restore_bootstrap(
  state: &AppState,
  filename: &str,
  bytes: &[u8],
  setup_token: &str,
  provided_master_key: Option<&[u8]>,
) -> Result<BootstrapRestoreResponse, HttpError> {
  if !state.rate_limiter.check("bootstrap").await {
    return Err(HttpError::new(
      axum::http::StatusCode::TOO_MANY_REQUESTS,
      RATE_LIMITED,
      "Too many setup attempts. Please try again later.",
    ));
  }

  // Claim the setup window for the whole operation. This serializes restore
  // with normal bootstrap and prevents two expensive restores racing.
  let mut setup = state.setup.write().await;
  let expected = setup.token.clone().ok_or_else(|| {
    HttpError::conflict(
      "BOOTSTRAP_CLOSED",
      "This instance has already been initialized.",
    )
  })?;
  if !token::constant_time_eq(setup_token, &expected) {
    state.rate_limiter.failure("bootstrap").await;
    return Err(HttpError::unauthorized(
      "SETUP_TOKEN_INVALID",
      "The setup token is invalid.",
    ));
  }

  // Verify server is uninitialized
  let admin_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admins")
    .fetch_one(state.db.pool())
    .await
    .map_err(|_| HttpError::internal())?;

  if admin_count > 0 {
    return Err(HttpError::conflict(
      "BOOTSTRAP_CLOSED",
      "This instance has already been initialized.",
    ));
  }

  // 1. Try decrypting with current system master key first.
  let (decrypted, needs_rekey) = match state.crypto.decrypt_backup(bytes) {
    Ok(data) => (data, false),
    Err(_) => {
      if let Some(key) = provided_master_key {
        let data = crate::services::crypto::CryptoService::decrypt_backup_with_key(bytes, key)
          .map_err(|_| {
            HttpError::bad_request(
              "BACKUP_DECRYPT_FAILED",
              "Failed to decrypt backup. Ensure you provided the correct master key for this backup.",
            )
          })?;
        (data, true)
      } else {
        return Err(HttpError::bad_request(
          "BACKUP_KEY_REQUIRED",
          "This backup was created on a different server or with a different master key. Please provide the master key from that server to restore it.",
        ));
      }
    }
  };

  // Generate safe filename for storage
  let target_key = if filename.ends_with(".dop")
    && filename
      .chars()
      .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
  {
    filename.to_string()
  } else {
    format!("restore_{}.dop", chrono::Utc::now().format("%Y%m%d_%H%M%S"))
  };

  // 2. If needs_rekey, re-key the database to the server's master key and re-encrypt the stored backup!
  let (final_bytes, restore_decrypted) = if needs_rekey {
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
        std::io::copy(&mut manifest_entry, &mut manifest_bytes)
          .map_err(|_| HttpError::internal())?;
      }

      {
        let mut db_entry = archive.by_name("dopbase.db").map_err(|_| {
          HttpError::bad_request("BACKUP_INVALID", "Backup archive is missing dopbase.db.")
        })?;
        if db_entry.size() > MAX_BACKUP_FILE_BYTES {
          return Err(HttpError::bad_request(
            "BACKUP_FILE_TOO_LARGE",
            "Database payload exceeds the backup size limit.",
          ));
        }
        let mut db_file = File::create(&temp_db_path).map_err(|_| HttpError::internal())?;
        std::io::copy(&mut db_entry, &mut db_file).map_err(|_| HttpError::internal())?;
      }
    }

    let restore_url = format!("sqlite://{}", temp_db_path.to_string_lossy());
    let temp_client = crate::services::db::DbClient::connect(&restore_url)
      .await
      .map_err(|e| {
        tracing::error!(%e, "failed to connect to uploaded sqlite db");
        HttpError::bad_request("BACKUP_INVALID_DB", "Invalid SQLite database in backup.")
      })?;

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

    let rekeyed_db_bytes = std::fs::read(&temp_db_path).map_err(|_| HttpError::internal())?;

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
    let enc_bytes = state
      .crypto
      .encrypt_backup(&zip_data)
      .map_err(|_| HttpError::internal())?;
    (enc_bytes, zip_data)
  } else {
    (bytes.to_vec(), decrypted)
  };

  let backup_dir = state.config.data_dir.join("backups");
  let _ = tokio::fs::create_dir_all(&backup_dir).await;
  let target_path = backup_dir.join(&target_key);
  tokio::fs::write(&target_path, &final_bytes)
    .await
    .map_err(|_| HttpError::internal())?;

  // Restore database tables (now fully keyed to system master key)
  crate::modules::backups::service::restore_database_from_archive(
    state,
    restore_decrypted,
    None,
    None,
    Some(&target_key),
  )
  .await?;

  // Audit event for bootstrap restore
  let _ = common::audit(
    state.db.pool(),
    "bootstrap",
    None,
    None,
    "backup.restored_bootstrap",
    None,
    None,
    Some("backup"),
    Some(&target_key),
    serde_json::json!({
      "backup": target_key,
      "size": final_bytes.len(),
      "rekeyed": needs_rekey
    }),
  )
  .await;

  // Close bootstrap window
  setup.token = None;
  state.rate_limiter.clear("bootstrap").await;

  Ok(BootstrapRestoreResponse {
    message: "Backup restored successfully.".to_string(),
    restored: true,
    key: target_key,
    size: final_bytes.len() as u64,
  })
}
