use crate::models::AdminRole;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// User account representation.
#[derive(Clone, Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
  /// Unique user identifier with `usr_` prefix.
  pub id: String,
  /// User email address.
  pub email: String,
  /// The fixed role closest to the account's role: `root`, `admin`,
  /// `member`, or `ai_agent`. Custom roles show as `member`, or `ai_agent`
  /// when they sign in with tokens only. Prefer `roleId` and `roleName`.
  pub role: AdminRole,
  /// The account's role. Empty for root, which has no role.
  pub role_id: Option<String>,
  /// The role's name, `Root` for root.
  pub role_name: String,
  /// ISO 8601 creation timestamp.
  pub created_at: String,
  /// ISO 8601 last update timestamp.
  pub updated_at: String,
  /// The account can sign in with a password.
  pub has_password: bool,
  /// The account has a single sign-on identity linked.
  pub sso_linked: bool,
  /// The accountable person behind an AI agent account.
  pub owner_id: Option<String>,
}

/// Request to create a new user account.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
  /// User email address. For an AI agent, its name: letters, digits, dots,
  /// dashes, and underscores.
  pub email: String,
  /// Account password (minimum 12 characters). Leave it out to create an
  /// account that signs in only through single sign-on, which must be on.
  #[serde(default)]
  pub password: Option<String>,
  /// A role id from `GET /api/v1/roles`, built-in or custom. Takes
  /// precedence over `role`.
  #[serde(default)]
  pub role_id: Option<String>,
  /// Account role (`admin`, `member`, or `ai_agent`). Defaults to `admin`.
  /// An AI agent has no password, signs in with personal tokens only, and
  /// the person who creates it becomes its owner.
  #[serde(default)]
  pub role: Option<AdminRole>,
}

/// Request to update an existing user account.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRequest {
  /// Updated email address.
  pub email: Option<String>,
  /// New account password.
  pub password: Option<String>,
  /// Updated account role (`admin` or `member`).
  pub role: Option<AdminRole>,
  /// A role id from `GET /api/v1/roles`. Takes precedence over `role`.
  #[serde(default)]
  pub role_id: Option<String>,
  /// A new owner for an AI agent account: a person who signs in.
  #[serde(default)]
  pub owner_id: Option<String>,
}
