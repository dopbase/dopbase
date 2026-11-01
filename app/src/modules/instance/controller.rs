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
  path = crate::constants::api::instance::ROOT,
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

/// Show instance status and health checks
///
/// Report instance telemetry and resource counts, including projects,
/// environments, secrets, users, AI agents, and tokens. Requires metadata
/// read access (a signed-in account or a personal token). Accounts holding
/// `instance.view` also get the health checks, the readings behind them, and
/// the last 24 hours of activity. Slow checks, such as the audit chain walk,
/// come from a cache refreshed every 10 minutes.
#[utoipa::path(
  get,
  path = crate::constants::api::instance::PUBLIC_STATUS,
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

/// Run the health checks now
///
/// Run every check, including the audit chain walk the status endpoint reads
/// from its cache, and return the same body as `GET /status`. Requires
/// `instance.view`, and a CSRF header from the Admin UI. Runs at most once
/// every 10 seconds.
#[utoipa::path(
  post,
  path = crate::constants::api::instance::RUN_CHECKS,
  tag = "instance",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  responses(
    (status = 200, description = "Checks ran", body = inline(HttpResponseFormat<super::model::StatusResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "The instance.view capability is required", body = crate::http::ErrorBody),
    (status = 429, description = "Checks ran less than 10 seconds ago", body = crate::http::ErrorBody),
  ),
)]
pub async fn run_checks(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
) -> Result<HttpResponse<super::model::StatusResponse>, InstanceError> {
  Ok(
    HttpResponse::ok(
      service::run_checks(&state, &identity, &headers).await?,
      "STATUS_CHECKS_RUN",
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
  path = crate::constants::api::instance::FACTORY_RESET,
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
  path = crate::constants::api::instance::FACTORY_RESET,
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
