use super::{error::InstanceError, model::InstanceStatus, service};
use crate::{
  http::{HttpResponse, HttpResponseFormat},
  models::AuthIdentity,
  state::AppState,
};
use axum::extract::State;

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
