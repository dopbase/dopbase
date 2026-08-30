use super::{error::HealthError, model::HealthResponse, service};
use crate::http::{ErrorBody, HttpResponse, HttpResponseFormat};
#[utoipa::path(get, path="/api/v1/health", tag="health", responses((status=200, body=inline(HttpResponseFormat<HealthResponse>)), (status=500, body=ErrorBody)))]
pub async fn health() -> Result<HttpResponse<HealthResponse>, HealthError> {
  Ok(HttpResponse::ok(service::health(), "HEALTH_OK"))
}
