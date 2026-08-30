use super::{controller, model::HealthResponse};
use crate::http::{ErrorBody, HttpResponseFormat};
use utoipa::OpenApi;
#[derive(OpenApi)]
#[openapi(paths(controller::health), components(schemas(HealthResponse, ErrorBody, HttpResponseFormat<HealthResponse>)), tags((name="health")))]
struct HealthApi;
pub fn build() -> utoipa::openapi::OpenApi {
  HealthApi::openapi()
}
