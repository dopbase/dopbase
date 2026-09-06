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
  /// Account role: `root`, `admin`, or `member`.
  pub role: AdminRole,
  /// ISO 8601 creation timestamp.
  pub created_at: String,
  /// ISO 8601 last update timestamp.
  pub updated_at: String,
}

/// Request to create a new user account.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
  /// User email address.
  pub email: String,
  /// Account password (minimum 12 characters).
  pub password: String,
  /// Account role (`admin` or `member`). Defaults to `admin`.
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
}
