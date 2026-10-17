use crate::models::{AdminRole, SessionKind};
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
  pub role: AdminRole,
  pub session_kind: SessionKind,
  pub token: Option<String>,
  pub csrf_token: Option<String>,
  pub last_login_at: Option<String>,
}
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionResponse {
  pub admin_id: String,
  pub email: String,
  pub role: AdminRole,
  pub session_kind: SessionKind,
  pub recent_authentication: bool,
  pub last_login_at: Option<String>,
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

/// Browser sign-in availability, read by the login screen.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OidcConfigResponse {
  /// True when an operator configured single sign-on.
  pub enabled: bool,
  /// The trusted issuer hosting the sign-in, when enabled.
  pub issuer: Option<String>,
}

/// Query for the SSO start endpoint.
#[derive(Deserialize, ToSchema)]
pub struct OidcStartQuery {
  /// In-app path to return to after a successful sign-in.
  pub redirect: Option<String>,
}

/// Query returned by the provider to the SSO callback endpoint.
#[derive(Deserialize, ToSchema)]
pub struct OidcCallbackQuery {
  pub code: Option<String>,
  pub state: Option<String>,
  pub error: Option<String>,
}

/// A pending browser sign-in, held between the redirect to the provider and the
/// callback that completes it.
#[derive(Clone, Debug)]
pub struct LoginFlow {
  pub state: String,
  pub nonce: String,
  pub code_verifier: String,
  pub redirect_to: Option<String>,
}
