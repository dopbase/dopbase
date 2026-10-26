use super::{controller, model::*};
use crate::http::{ErrorBody, HttpResponseFormat};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
  paths(
    controller::list,
    controller::get,
    controller::create,
    controller::update,
    controller::delete,
    controller::add_member,
    controller::remove_member
  ),
  components(schemas(
    GroupResponse,
    GroupMember,
    GroupDetailResponse,
    CreateGroupRequest,
    UpdateGroupRequest,
    ErrorBody,
    HttpResponseFormat<Vec<GroupResponse>>,
    HttpResponseFormat<GroupDetailResponse>,
    HttpResponseFormat<serde_json::Value>
  )),
  tags((name = "groups"))
)]
struct GroupsApi;

pub fn build() -> utoipa::openapi::OpenApi {
  GroupsApi::openapi()
}
