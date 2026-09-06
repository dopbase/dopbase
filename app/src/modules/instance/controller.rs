use super::{error::InstanceError, model::InstanceStatus, service};
use crate::{
  http::{HttpResponse, HttpResponseFormat},
  models::AuthIdentity,
  state::AppState,
};
use axum::{
  extract::State,
  http::{HeaderMap, HeaderName, HeaderValue},
};

/// Show instance status
///
/// Report the version, public URL, initialization state, and the health of
/// the database and key store. Administrator authentication is required.
#[utoipa::path(
  get,
  path = "/api/v1/instance",
  tag = "instance",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  responses(
    (status = 200, description = "Instance status fetched", body = inline(HttpResponseFormat<InstanceStatus>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Only administrators may view instance status", body = crate::http::ErrorBody),
  ),
)]
pub async fn status(
  State(state): State<AppState>,
  identity: AuthIdentity,
) -> Result<HttpResponse<InstanceStatus>, InstanceError> {
  Ok(HttpResponse::ok(
    service::status(&state, &identity).await?,
    "INSTANCE_STATUS_FETCHED",
  ))
}

/// Show public instance status
///
/// Report instance telemetry and resource counts, including projects,
/// environments, secrets, users, service accounts, and tokens.
/// Requires metadata read access (administrator session or service account token).
#[utoipa::path(
  get,
  path = "/api/v1/status",
  tag = "instance",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  responses(
    (status = 200, description = "Instance telemetry and status fetched", body = inline(HttpResponseFormat<super::model::StatusResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Metadata read access is required", body = crate::http::ErrorBody),
  ),
)]
pub async fn public_status(
  State(state): State<AppState>,
  identity: AuthIdentity,
) -> Result<HttpResponse<super::model::StatusResponse>, InstanceError> {
  Ok(
    HttpResponse::ok(
      service::public_status(&state, &identity).await?,
      "STATUS_FETCHED",
    )
    .with_header(
      HeaderName::from_static("cache-control"),
      HeaderValue::from_static("no-store"),
    ),
  )
}

/// Preview factory reset
///
/// Count existing resources across all tables and backup archives that
/// will be purged during a factory reset. Requires a root administrator browser session.
#[utoipa::path(
  get,
  path = "/api/v1/instance/factory-reset",
  tag = "instance",
  security(("cookieAuth" = [])),
  responses(
    (status = 200, description = "Factory reset resource counts fetched", body = inline(HttpResponseFormat<super::model::FactoryResetPreview>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Root administrator browser session is required", body = crate::http::ErrorBody),
  ),
)]
pub async fn factory_reset_preview(
  State(state): State<AppState>,
  identity: AuthIdentity,
) -> Result<HttpResponse<super::model::FactoryResetPreview>, InstanceError> {
  Ok(HttpResponse::ok(
    service::preview(&state, &identity).await?,
    "FACTORY_RESET_PREVIEW",
  ))
}

/// Perform a factory reset
///
/// Permanently erase all instance data, including users, projects, environments,
/// secrets, tokens, audit logs, and backups, returning the instance to setup mode.
/// Requires the root password, confirmation string, danger zone acknowledgment,
/// recent authentication, CSRF header, and a root administrator browser session.
#[utoipa::path(
  post,
  path = "/api/v1/instance/factory-reset",
  tag = "instance",
  security(("cookieAuth" = [])),
  request_body = super::model::FactoryResetRequest,
  responses(
    (status = 200, description = "Factory reset completed successfully", body = inline(HttpResponseFormat<serde_json::Value>)),
    (status = 400, description = "Confirmation text mismatch or unacknowledged warning", body = crate::http::ErrorBody),
    (status = 401, description = "Root password is incorrect", body = crate::http::ErrorBody),
    (status = 403, description = "Root administrator browser session with recent authentication and CSRF token is required", body = crate::http::ErrorBody),
    (status = 409, description = "A factory reset is already in progress", body = crate::http::ErrorBody),
  ),
)]
pub async fn factory_reset(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  axum::Json(request): axum::Json<super::model::FactoryResetRequest>,
) -> Result<HttpResponse<serde_json::Value>, InstanceError> {
  service::factory_reset(&state, &identity, &headers, request).await?;
  Ok(HttpResponse::done("FACTORY_RESET_COMPLETE"))
}
