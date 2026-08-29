use super::{controller, model::*};
use crate::{
    http::{ErrorBody, HttpResponseFormat},
    models::SessionKind,
};
use utoipa::OpenApi;
#[derive(OpenApi)]
#[openapi(paths(controller::login,controller::logout,controller::session,controller::reauthenticate,controller::change_password),components(schemas(LoginRequest,LoginResponse,SessionResponse,ReauthenticateRequest,ChangePasswordRequest,SessionKind,ErrorBody,HttpResponseFormat<LoginResponse>,HttpResponseFormat<SessionResponse>,HttpResponseFormat<serde_json::Value>)),tags((name="authentication")))]
struct AuthApi;
pub fn build() -> utoipa::openapi::OpenApi {
    AuthApi::openapi()
}
