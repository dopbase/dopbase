use super::{controller, model::*};
use crate::{
  http::{ErrorBody, HttpResponseFormat},
  models::SessionKind,
};
use utoipa::OpenApi;
#[derive(OpenApi)]
#[openapi(paths(controller::login,controller::launch,controller::logout,controller::session,controller::reauthenticate,controller::change_password,controller::methods,controller::cli_device,controller::cli_token,controller::cli_cancel,controller::cli_inspect,controller::cli_approve,controller::cli_deny),components(schemas(AuthMethodsResponse,SsoMethod,DeviceStartRequest,DevicePurpose,DeviceStartResponse,DeviceCodeRequest,DevicePollStatus,DeviceTokenResponse,UserCodeRequest,DeviceRequestDetails,HttpResponseFormat<AuthMethodsResponse>,HttpResponseFormat<DeviceStartResponse>,HttpResponseFormat<DeviceTokenResponse>,HttpResponseFormat<DeviceRequestDetails>,LoginRequest,LaunchRequest,LoginResponse,SessionResponse,ReauthenticateRequest,ChangePasswordRequest,SessionKind,ErrorBody,HttpResponseFormat<LoginResponse>,HttpResponseFormat<SessionResponse>,HttpResponseFormat<serde_json::Value>)),tags((name="authentication")))]
struct AuthApi;
pub fn build() -> utoipa::openapi::OpenApi {
  AuthApi::openapi()
}
