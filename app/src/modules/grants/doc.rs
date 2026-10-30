use super::{controller, model::*};
use crate::{
  http::{ErrorBody, HttpResponseFormat},
  services::authx::Level,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
  paths(
    controller::set,
    controller::delete,
    controller::project_access,
    controller::user_access
  ),
  components(schemas(
    GrantResponse,
    SetGrantRequest,
    PrincipalType,
    ScopeType,
    Level,
    GroupRef,
    ProjectAccessResponse,
    UserAccessResponse,
    ErrorBody,
    HttpResponseFormat<GrantResponse>,
    HttpResponseFormat<ProjectAccessResponse>,
    HttpResponseFormat<UserAccessResponse>,
    HttpResponseFormat<serde_json::Value>
  )),
  tags((name = "grants"))
)]
struct GrantsApi;

pub fn build() -> utoipa::openapi::OpenApi {
  GrantsApi::openapi()
}
