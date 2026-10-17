use super::{controller, model::*};
use crate::{
  http::{ErrorBody, HttpResponseFormat},
  models::SessionKind,
};
use utoipa::OpenApi;
#[derive(OpenApi)]
#[openapi(paths(controller::login,controller::logout,controller::session,controller::reauthenticate,controller::change_password,controller::oidc_config,controller::oidc_start,controller::oidc_callback),components(schemas(LoginRequest,LoginResponse,SessionResponse,ReauthenticateRequest,ChangePasswordRequest,OidcConfigResponse,OidcStartQuery,OidcCallbackQuery,SessionKind,ErrorBody,HttpResponseFormat<LoginResponse>,HttpResponseFormat<SessionResponse>,HttpResponseFormat<OidcConfigResponse>,HttpResponseFormat<serde_json::Value>)),tags((name="authentication")))]
struct AuthApi;
pub fn build() -> utoipa::openapi::OpenApi {
  AuthApi::openapi()
}
