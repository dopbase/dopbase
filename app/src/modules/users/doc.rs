use super::{controller, model::*};
use crate::{
  http::{ErrorBody, HttpResponseFormat},
  models::AdminRole,
  modules::grants::model::GroupRef,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
  paths(
    controller::list,
    controller::get,
    controller::create,
    controller::update,
    controller::delete,
    controller::unlink_sso,
    controller::disable,
    controller::enable
  ),
  components(schemas(
    UserResponse,
    UserStatus,
    GroupRef,
    CreateUserRequest,
    UpdateUserRequest,
    AdminRole,
    ErrorBody,
    HttpResponseFormat<Vec<UserResponse>>,
    HttpResponseFormat<UserResponse>,
    HttpResponseFormat<serde_json::Value>
  )),
  tags((name = "users"))
)]
struct UsersApi;

pub fn build() -> utoipa::openapi::OpenApi {
  UsersApi::openapi()
}
