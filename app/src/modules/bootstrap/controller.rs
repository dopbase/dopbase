use super::{error::BootstrapError, model::*, service};
use crate::{
  http::{HttpResponse, HttpResponseFormat},
  state::AppState,
};
use axum::{
  extract::State,
  http::{HeaderName, HeaderValue},
};

/// Show bootstrap status
///
/// Report whether the instance still needs its first administrator
/// (`setupRequired`) or is already ready.
#[utoipa::path(
  get,
  path = "/api/v1/bootstrap/status",
  tag = "bootstrap",
  responses(
    (status = 200, description = "Bootstrap state fetched", body = inline(HttpResponseFormat<BootstrapStatus>)),
    (status = 500, description = "An internal error occurred", body = crate::http::ErrorBody),
  ),
)]
pub async fn status(
  State(state): State<AppState>
) -> Result<HttpResponse<BootstrapStatus>, BootstrapError> {
  Ok(HttpResponse::ok(
    service::status(&state).await?,
    "BOOTSTRAP_STATUS",
  ))
}

/// Create the first administrator
///
/// Complete instance setup by creating the initial admin account. The
/// request must carry the setup token printed at first startup. On success
/// the setup window closes permanently, the response sets the session
/// cookie, and a CSRF token is returned for subsequent mutations.
#[utoipa::path(
  post,
  path = "/api/v1/bootstrap/admin",
  tag = "bootstrap",
  request_body = BootstrapAdminRequest,
  responses(
    (status = 201, description = "Administrator created and session started", body = inline(HttpResponseFormat<BootstrapAdminResponse>)),
    (status = 401, description = "The setup token is invalid", body = crate::http::ErrorBody),
    (status = 409, description = "This instance has already been initialized", body = crate::http::ErrorBody),
    (status = 422, description = "Email or password is invalid", body = crate::http::ErrorBody),
    (status = 429, description = "Too many setup attempts; try again later", body = crate::http::ErrorBody),
  ),
)]
pub async fn create_admin(
  State(state): State<AppState>,
  axum::Json(request): axum::Json<BootstrapAdminRequest>,
) -> Result<HttpResponse<BootstrapAdminResponse>, BootstrapError> {
  let created = service::create(&state, request).await?;
  let secure = if state.config.public_url.starts_with("https://") {
    "; Secure"
  } else {
    ""
  };
  let cookie = format!(
    "dopbase_session={}; Path=/; HttpOnly; SameSite=Strict; Max-Age=86400{}",
    created.session_token, secure
  );
  Ok(
    HttpResponse::created(created.response, "ADMIN_BOOTSTRAPPED").with_header(
      HeaderName::from_static("set-cookie"),
      HeaderValue::from_str(&cookie).map_err(|_| crate::http::HttpError::internal())?,
    ),
  )
}

/// Restore server from backup during first run
///
/// Restores an uninitialized instance from an encrypted backup archive (.dop).
/// The archive must be encrypted with this server's master encryption key.
#[utoipa::path(
  post,
  path = "/api/v1/bootstrap/restore",
  tag = "bootstrap",
  responses(
    (status = 200, description = "Backup restored and server initialized", body = inline(HttpResponseFormat<BootstrapRestoreResponse>)),
    (status = 400, description = "Failed to decrypt or corrupted backup", body = crate::http::ErrorBody),
    (status = 409, description = "This instance has already been initialized", body = crate::http::ErrorBody),
    (status = 429, description = "Too many attempts; try again later", body = crate::http::ErrorBody),
  ),
)]
pub async fn restore(
  State(state): State<AppState>,
  mut multipart: axum::extract::Multipart,
) -> Result<HttpResponse<BootstrapRestoreResponse>, crate::http::HttpError> {
  let mut file_name = None;
  let mut file_bytes = None;
  let mut master_key_bytes = None;
  let mut setup_token = None;

  while let Some(field) = multipart.next_field().await.map_err(|_| {
    crate::http::HttpError::bad_request("MULTIPART_ERROR", "Failed to process multipart upload.")
  })? {
    match field.name() {
      Some("file") | Some("backup") => {
        file_name = field.file_name().map(|s| s.to_string());
        let data = field.bytes().await.map_err(|_| {
          crate::http::HttpError::bad_request(
            "MULTIPART_ERROR",
            "Failed to read uploaded file data.",
          )
        })?;
        file_bytes = Some(data.to_vec());
      }
      Some("key") | Some("master_key") => {
        let data = field.bytes().await.map_err(|_| {
          crate::http::HttpError::bad_request(
            "MULTIPART_ERROR",
            "Failed to read uploaded master key.",
          )
        })?;
        if !data.is_empty() {
          let parsed = crate::services::crypto::parse_master_key(&data).map_err(|e| {
            crate::http::HttpError::bad_request("INVALID_MASTER_KEY", &e.to_string())
          })?;
          master_key_bytes = Some(parsed);
        }
      }
      Some("setup_token") | Some("setupToken") => {
        let data = field.text().await.map_err(|_| {
          crate::http::HttpError::bad_request("MULTIPART_ERROR", "Failed to read setup token.")
        })?;
        setup_token = Some(data);
      }
      _ => {}
    }
  }

  let name = file_name.unwrap_or_else(|| "backup.dop".to_string());
  let bytes = file_bytes.ok_or_else(|| {
    crate::http::HttpError::bad_request(
      "BACKUP_FILE_MISSING",
      "No file provided in the restore request.",
    )
  })?;
  let setup_token = setup_token.unwrap_or_default();

  let response = service::restore_bootstrap(
    &state,
    &name,
    &bytes,
    &setup_token,
    master_key_bytes.as_deref(),
  )
  .await?;
  Ok(HttpResponse::ok(response, "BACKUP_RESTORED"))
}
