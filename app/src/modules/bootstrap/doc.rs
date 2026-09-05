use super::{controller, model::*};
use crate::http::{ErrorBody, HttpResponseFormat};
use utoipa::OpenApi;
#[derive(OpenApi)]
#[openapi(
  paths(controller::status, controller::create_admin, controller::restore),
  components(schemas(
    BootstrapStatus,
    BootstrapAdminRequest,
    BootstrapAdminResponse,
    BootstrapRestoreResponse,
    ErrorBody,
    HttpResponseFormat<BootstrapStatus>,
    HttpResponseFormat<BootstrapAdminResponse>,
    HttpResponseFormat<BootstrapRestoreResponse>
  )),
  tags((name = "bootstrap"))
)]
struct BootstrapApi;
pub fn build() -> utoipa::openapi::OpenApi {
  BootstrapApi::openapi()
}
