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
  /// Account role: `root`, `admin`, `member`, or `ai_agent`.
  pub role: AdminRole,
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
  /// A new owner for an AI agent account: a person who signs in.
  #[serde(default)]
  pub owner_id: Option<String>,
}
