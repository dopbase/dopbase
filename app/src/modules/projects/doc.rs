use super::{controller, model::*};
use crate::{
  http::{ErrorBody, HttpResponseFormat},
  models::{AffectedCounts, SecretInput},
};
use utoipa::OpenApi;
#[derive(OpenApi)]
#[openapi(paths(controller::list,controller::create,controller::init,controller::show,controller::rename,controller::delete),components(schemas(ProjectResponse,CreateProjectRequest,RenameProjectRequest,InitProjectRequest,InitProjectResponse,DeleteProjectResponse,AffectedCounts,SecretInput,ErrorBody,HttpResponseFormat<Vec<ProjectResponse>>,HttpResponseFormat<ProjectResponse>,HttpResponseFormat<InitProjectResponse>,HttpResponseFormat<DeleteProjectResponse>)),tags((name="projects")))]
struct ProjectsApi;
pub fn build() -> utoipa::openapi::OpenApi {
  ProjectsApi::openapi()
}
