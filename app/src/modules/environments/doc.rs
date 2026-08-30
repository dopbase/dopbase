use super::{controller, model::*};
use crate::{
  http::{ErrorBody, HttpResponseFormat},
  models::AffectedCounts,
};
use utoipa::OpenApi;
#[derive(OpenApi)]
#[openapi(paths(controller::list,controller::resolve,controller::create,controller::show,controller::rename,controller::delete),components(schemas(EnvironmentResponse,CreateEnvironmentRequest,RenameEnvironmentRequest,DeleteEnvironmentResponse,AffectedCounts,ErrorBody,HttpResponseFormat<Vec<EnvironmentResponse>>,HttpResponseFormat<EnvironmentResponse>,HttpResponseFormat<DeleteEnvironmentResponse>)),tags((name="environments")))]
struct EnvironmentsApi;
pub fn build() -> utoipa::openapi::OpenApi {
  EnvironmentsApi::openapi()
}
