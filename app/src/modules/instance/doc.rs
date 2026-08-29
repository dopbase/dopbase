use super::{controller, model::InstanceStatus};
use crate::http::{ErrorBody, HttpResponseFormat};
use utoipa::OpenApi;
#[derive(OpenApi)]
#[openapi(paths(controller::status),components(schemas(InstanceStatus,ErrorBody,HttpResponseFormat<InstanceStatus>)),tags((name="instance")))]
struct InstanceApi;
pub fn build() -> utoipa::openapi::OpenApi {
    InstanceApi::openapi()
}
