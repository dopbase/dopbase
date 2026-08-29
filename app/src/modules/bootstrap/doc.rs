use super::{controller, model::*};
use crate::http::{ErrorBody, HttpResponseFormat};
use utoipa::OpenApi;
#[derive(OpenApi)]
#[openapi(paths(controller::status,controller::create_admin),components(schemas(BootstrapStatus,BootstrapAdminRequest,BootstrapAdminResponse,ErrorBody,HttpResponseFormat<BootstrapStatus>,HttpResponseFormat<BootstrapAdminResponse>)),tags((name="bootstrap")))]
struct BootstrapApi;
pub fn build() -> utoipa::openapi::OpenApi {
    BootstrapApi::openapi()
}
