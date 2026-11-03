use super::{controller, model::*};
use crate::{
  http::{ErrorBody, HttpResponseFormat},
  models::SecretInput,
};
use utoipa::OpenApi;
#[derive(OpenApi)]
#[openapi(paths(controller::secrets),components(schemas(RuntimeSnapshot,SecretInput,ErrorBody,HttpResponseFormat<RuntimeSnapshot>)),tags((name="runtime")))]
struct RuntimeApi;
pub fn build() -> utoipa::openapi::OpenApi {
  RuntimeApi::openapi()
}
