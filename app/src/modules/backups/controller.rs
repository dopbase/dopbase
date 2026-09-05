use axum::{
  Json,
  extract::{Multipart, Path, State},
  http::{HeaderMap, HeaderValue, header},
  response::{IntoResponse, Response},
};

use super::{model::*, service};
use crate::{
  extractors::{self, require_mutation, require_recent_browser_auth},
  http::{HttpError, HttpResponse, HttpResponseFormat},
  models::AuthIdentity,
  modules::common,
  state::AppState,
};

/// List backups
///
/// Return the list of all backup snapshots stored on the server.
#[utoipa::path(
  get,
  path = "/api/v1/backups",
  tag = "backups",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  responses(
    (status = 200, description = "Backups listed successfully", body = inline(HttpResponseFormat<Vec<BackupItem>>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Only administrators may list backups", body = crate::http::ErrorBody),
  ),
)]
pub async fn list(
  State(state): State<AppState>,
  identity: AuthIdentity,
) -> Result<HttpResponse<Vec<BackupItem>>, HttpError> {
  Ok(HttpResponse::ok(
    service::list(&state, &identity).await?,
    "BACKUPS_LISTED",
  ))
}

/// Create a new backup
///
/// Trigger an atomic point-in-time snapshot of the database, encrypted with the server master key.
#[utoipa::path(
  post,
  path = "/api/v1/backups",
  tag = "backups",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  request_body = CreateBackupRequest,
  responses(
    (status = 201, description = "Backup created successfully", body = inline(HttpResponseFormat<BackupItem>)),
    (status = 400, description = "Invalid backup name", body = crate::http::ErrorBody),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Only administrators may create backups", body = crate::http::ErrorBody),
    (status = 409, description = "Backup with this name already exists", body = crate::http::ErrorBody),
  ),
)]
pub async fn create(
  State(state): State<AppState>,
  identity: AuthIdentity,
  headers: HeaderMap,
  Json(request): Json<CreateBackupRequest>,
) -> Result<HttpResponse<BackupItem>, HttpError> {
  require_mutation(&identity, &headers)?;
  let item = service::create(&state, &identity, request).await?;
  Ok(HttpResponse::created(item, "BACKUP_CREATED"))
}

/// Download a backup file
///
/// Download the encrypted backup archive.
#[utoipa::path(
  get,
  path = "/api/v1/backups/{key}",
  tag = "backups",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("key" = String, Path, description = "Backup key / filename")),
  responses(
    (status = 200, description = "Backup file download", content_type = "application/octet-stream"),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Only administrators may download backups", body = crate::http::ErrorBody),
    (status = 404, description = "Backup was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn download(
  State(state): State<AppState>,
  identity: AuthIdentity,
  Path(key): Path<String>,
) -> Result<Response, HttpError> {
  let (file_path, size) = service::read_for_download(&state, &identity, &key).await?;
  let file_bytes = tokio::fs::read(&file_path)
    .await
    .map_err(|_| HttpError::internal())?;

  let mut response = file_bytes.into_response();
  response.headers_mut().insert(
    header::CONTENT_TYPE,
    HeaderValue::from_static("application/octet-stream"),
  );
  if let Ok(val) = HeaderValue::from_str(&format!("attachment; filename=\"{key}\"")) {
    response
      .headers_mut()
      .insert(header::CONTENT_DISPOSITION, val);
  }
  if let Ok(val) = HeaderValue::from_str(&size.to_string()) {
    response.headers_mut().insert(header::CONTENT_LENGTH, val);
  }
  Ok(response)
}

/// Download the master encryption key
///
/// Returns the server's 32-byte master encryption key as a binary file.
#[utoipa::path(
  get,
  path = "/api/v1/backups/master-key",
  tag = "backups",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  responses(
    (status = 200, description = "Master key file download", content_type = "application/octet-stream"),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Recent password confirmation required", body = crate::http::ErrorBody),
  ),
)]
pub async fn download_master_key(
  State(state): State<AppState>,
  identity: AuthIdentity,
) -> Result<Response, HttpError> {
  let (admin_id, email) = extractors::require_admin(&identity)?;
  require_recent_browser_auth(&identity)?;
  let key_bytes = state.crypto.master_key_bytes();
  let _ = common::audit(
    state.db.pool(),
    "admin",
    Some(admin_id),
    Some(email),
    "master_key.downloaded",
    None,
    None,
    Some("master_key"),
    None,
    serde_json::json!({}),
  )
  .await;
  let mut response = key_bytes.into_response();
  response.headers_mut().insert(
    header::CONTENT_TYPE,
    HeaderValue::from_static("application/octet-stream"),
  );
  response.headers_mut().insert(
    header::CONTENT_DISPOSITION,
    HeaderValue::from_static("attachment; filename=\"master.key\""),
  );
  response
    .headers_mut()
    .insert(header::CONTENT_LENGTH, HeaderValue::from_static("32"));
  Ok(response)
}

/// Upload a backup file
///
/// Upload an existing encrypted backup file to the server.
#[utoipa::path(
  post,
  path = "/api/v1/backups/upload",
  tag = "backups",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  responses(
    (status = 201, description = "Backup uploaded successfully", body = inline(HttpResponseFormat<BackupItem>)),
    (status = 400, description = "Invalid or corrupted backup file", body = crate::http::ErrorBody),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Only administrators may upload backups", body = crate::http::ErrorBody),
  ),
)]
pub async fn upload(
  State(state): State<AppState>,
  identity: AuthIdentity,
  headers: HeaderMap,
  mut multipart: Multipart,
) -> Result<HttpResponse<BackupItem>, HttpError> {
  require_mutation(&identity, &headers)?;
  let mut file_name = None;
  let mut file_bytes = None;
  let mut master_key_bytes = None;

  while let Some(field) = multipart
    .next_field()
    .await
    .map_err(|_| HttpError::bad_request("MULTIPART_ERROR", "Failed to process multipart upload."))?
  {
    match field.name() {
      Some("file") | Some("backup") => {
        file_name = field.file_name().map(|s| s.to_string());
        let data = field.bytes().await.map_err(|_| {
          HttpError::bad_request("MULTIPART_ERROR", "Failed to read uploaded file data.")
        })?;
        file_bytes = Some(data.to_vec());
      }
      Some("key") | Some("master_key") => {
        let data = field.bytes().await.map_err(|_| {
          HttpError::bad_request("MULTIPART_ERROR", "Failed to read uploaded master key.")
        })?;
        if !data.is_empty() {
          let parsed = crate::services::crypto::parse_master_key(&data)
            .map_err(|e| HttpError::bad_request("INVALID_MASTER_KEY", &e.to_string()))?;
          master_key_bytes = Some(parsed);
        }
      }
      _ => {}
    }
  }

  let name = file_name.unwrap_or_else(|| "backup.dop".to_string());
  let bytes = file_bytes.ok_or_else(|| {
    HttpError::bad_request(
      "BACKUP_FILE_MISSING",
      "No file provided in the upload request.",
    )
  })?;

  let item = service::upload(&state, &identity, &name, bytes, master_key_bytes.as_deref()).await?;
  Ok(HttpResponse::created(item, "BACKUP_UPLOADED"))
}

/// Restore a backup
///
/// Restores the database from an encrypted backup archive.
#[utoipa::path(
  post,
  path = "/api/v1/backups/{key}/restore",
  tag = "backups",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("key" = String, Path, description = "Backup key / filename")),
  request_body(content = Option<RestoreBackupRequest>),
  responses(
    (status = 200, description = "Backup restored successfully", body = inline(HttpResponseFormat<serde_json::Value>)),
    (status = 400, description = "Corrupted or incompatible backup", body = crate::http::ErrorBody),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Recent password confirmation required", body = crate::http::ErrorBody),
    (status = 404, description = "Backup was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn restore(
  State(state): State<AppState>,
  identity: AuthIdentity,
  headers: HeaderMap,
  Path(key): Path<String>,
  body: Option<Json<RestoreBackupRequest>>,
) -> Result<HttpResponse<serde_json::Value>, HttpError> {
  require_mutation(&identity, &headers)?;
  let provided_key = body
    .and_then(|Json(b)| b.master_key)
    .filter(|k| !k.trim().is_empty());
  let key_bytes = if let Some(k) = provided_key {
    Some(
      crate::services::crypto::parse_master_key(k.as_bytes())
        .map_err(|e| HttpError::bad_request("INVALID_MASTER_KEY", &e.to_string()))?,
    )
  } else {
    None
  };
  service::restore(&state, &identity, &key, key_bytes.as_deref()).await?;
  Ok(HttpResponse::ok(
    serde_json::json!({ "restored": true }),
    "BACKUP_RESTORED",
  ))
}

/// Delete a backup
///
/// Deletes an encrypted backup archive from the server.
#[utoipa::path(
  delete,
  path = "/api/v1/backups/{key}",
  tag = "backups",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("key" = String, Path, description = "Backup key / filename")),
  responses(
    (status = 200, description = "Backup deleted successfully", body = inline(HttpResponseFormat<serde_json::Value>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Only administrators may delete backups", body = crate::http::ErrorBody),
    (status = 404, description = "Backup was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn delete(
  State(state): State<AppState>,
  identity: AuthIdentity,
  headers: HeaderMap,
  Path(key): Path<String>,
) -> Result<HttpResponse<serde_json::Value>, HttpError> {
  require_mutation(&identity, &headers)?;
  service::delete(&state, &identity, &key).await?;
  Ok(HttpResponse::ok(
    serde_json::json!({ "deleted": true }),
    "BACKUP_DELETED",
  ))
}
