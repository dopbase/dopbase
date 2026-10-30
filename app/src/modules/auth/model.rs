use crate::{
  models::{AdminRole, SessionKind},
  services::authx::{Capability, Level},
};
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
  /// The display name of the account's role, from the role tables.
  pub role_name: String,
  /// What the account may do to the instance. Gate UI on these, not on
  /// `role`.
  pub capabilities: Vec<Capability>,
  /// The deepest project and environment access any grant can give.
  pub max_level: Level,
  pub session_kind: SessionKind,
  pub recent_authentication: bool,
  pub last_login_at: Option<String>,
  /// The account has a password, whether or not it may use it.
  pub has_password: bool,
  /// The account can confirm sensitive actions with its password.
  pub password_confirmation: bool,
  /// The provider name, when the account can confirm with single sign-on.
  pub sso_confirmation: Option<String>,
  /// A CSRF token for this tab, for browser sessions. A tab opened later,
  /// such as a CLI approval link, has no token from the sign-in response.
  pub csrf_token: Option<String>,
}
#[derive(Deserialize, ToSchema)]
pub struct ReauthenticateRequest {
  pub password: String,
}
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
  /// Leave empty when the account has no password yet.
  #[serde(default)]
  pub current_password: String,
  pub new_password: String,
}
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SsoMethod {
  pub name: String,
}
/// Which ways of signing in this server accepts. Public, so the login page
/// and the CLI can pick a default before anyone is signed in.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthMethodsResponse {
  pub password: bool,
  /// Whether the login page shows the email and password form. Root can
  /// still open it at `/login?password` when it is hidden.
  pub password_form: bool,
  pub sso: Option<SsoMethod>,
  pub cli_browser: bool,
}
/// What approving a browser CLI request does.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DevicePurpose {
  /// Sign a new CLI session in.
  #[default]
  Login,
  /// Confirm the CLI session that made the request, for an account
  /// without a password.
  Reauth,
}
#[derive(Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeviceStartRequest {
  /// Shown to the approver so they can tell which machine asked.
  pub hostname: Option<String>,
  /// `reauth` needs the CLI session's bearer token. Defaults to `login`.
  #[serde(default)]
  pub purpose: Option<DevicePurpose>,
}
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeviceStartResponse {
  pub device_code: String,
  pub user_code: String,
  pub verification_uri: String,
  pub verification_uri_complete: String,
  pub expires_in: i64,
  pub interval: i64,
}
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCodeRequest {
  pub device_code: String,
}
#[derive(Serialize, ToSchema, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DevicePollStatus {
  Pending,
  SlowDown,
  Approved,
}
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeviceTokenResponse {
  pub status: DevicePollStatus,
  /// Present only when `status` is `approved`.
  pub session: Option<LoginResponse>,
}
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserCodeRequest {
  pub user_code: String,
}
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeviceRequestDetails {
  pub hostname: Option<String>,
  pub ip_address: Option<String>,
  pub created_at: String,
  pub expires_at: String,
  pub purpose: DevicePurpose,
}
