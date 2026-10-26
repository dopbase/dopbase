use super::{controller, model::*};
use crate::{
  http::{ErrorBody, HttpResponseFormat},
  services::authx::{Capability, Level, SignIn},
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
  paths(
    controller::list,
    controller::create,
    controller::update,
    controller::delete
  ),
  components(schemas(
    RoleResponse,
    CreateRoleRequest,
    UpdateRoleRequest,
    TokenScope,
    Capability,
    Level,
    SignIn,
    ErrorBody,
    HttpResponseFormat<Vec<RoleResponse>>,
    HttpResponseFormat<RoleResponse>,
    HttpResponseFormat<serde_json::Value>
  )),
  tags((name = "roles"))
)]
struct RolesApi;

pub fn build() -> utoipa::openapi::OpenApi {
  RolesApi::openapi()
}
