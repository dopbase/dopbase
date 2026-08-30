use super::{error::HealthError, model::HealthResponse, service};
use crate::http::{HttpResponse, HttpResponseFormat};

/// Health check
///
/// Report the service identity, version, and API version. Suitable for
/// load balancer and uptime probes.
#[utoipa::path(
  get,
  path = "/api/v1/health",
  tag = "health",
  responses(
    (status = 200, description = "Service is healthy", body = inline(HttpResponseFormat<HealthResponse>)),
    (status = 500, description = "An internal error occurred", body = crate::http::ErrorBody),
  ),
)]
pub async fn health() -> Result<HttpResponse<HealthResponse>, HealthError> {
  Ok(HttpResponse::ok(service::health(), "HEALTH_OK"))
}
