use super::{controller, model::*};
use crate::http::{ErrorBody, HttpResponseFormat};
use utoipa::OpenApi;
#[derive(OpenApi)]
#[openapi(paths(controller::list),components(schemas(AuditEvent,AuditPage,ErrorBody,HttpResponseFormat<AuditPage>)),tags((name="audit")))]
struct AuditApi;
pub fn build() -> utoipa::openapi::OpenApi {
  AuditApi::openapi()
}
