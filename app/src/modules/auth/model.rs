use crate::models::SessionKind;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
  pub email: String,
  pub password: String,
  pub session_kind: SessionKind,
}
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
  pub admin_id: String,
  pub email: String,
  pub session_kind: SessionKind,
  pub token: Option<String>,
  pub csrf_token: Option<String>,
}
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionResponse {
  pub admin_id: String,
  pub email: String,
  pub session_kind: SessionKind,
  pub recent_authentication: bool,
}
#[derive(Deserialize, ToSchema)]
pub struct ReauthenticateRequest {
  pub password: String,
}
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
  pub current_password: String,
  pub new_password: String,
}
