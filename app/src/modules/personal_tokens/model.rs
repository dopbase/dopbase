use crate::{modules::grants::model::ScopeType, services::authx::Level};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A personal token's metadata. Its value is shown only once, when created.
#[derive(Clone, Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PersonalTokenResponse {
  /// Token identifier with `ptk_` prefix, or `ait_` for a former agent token.
  pub id: String,
  /// The account the token acts for.
  pub user_id: String,
  pub name: String,
  pub scope_type: ScopeType,
  /// Project or environment id. Empty for all projects.
  pub scope_id: String,
  /// `project` or `project/environment`. Empty for all projects.
  pub scope_label: String,
  /// The most the token reaches, even where the account reaches further.
  pub level: Level,
  pub created_by: Option<String>,
  pub created_at: String,
  pub expires_at: Option<String>,
  pub last_used_at: Option<String>,
  pub revoked_at: Option<String>,
}

/// Create a personal token for yourself, or for an AI agent account you
/// answer for.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatePersonalTokenRequest {
  /// Unique among the account's tokens, at most 64 characters.
  pub name: String,
  /// `all`, `project`, or `environment` for a data token, or `audit` for
  /// an audit token, as the role allows. Defaults to `all`, or to `audit`
  /// for roles whose tokens read the audit log.
  #[serde(default)]
  pub scope_type: Option<ScopeType>,
  /// Project id or name, or environment id.
  #[serde(default)]
  pub scope: Option<String>,
  /// `metadata`, `read`, or `write`, within what the role allows. Audit
  /// tokens take none.
  #[serde(default)]
  pub level: Option<Level>,
  /// Lifetime such as `12h` or `30d`, within the role's limit. Defaults to
  /// the role's default.
  #[serde(default)]
  pub expires_in: Option<String>,
}

/// A new token with its value.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatedPersonalTokenResponse {
  pub token: PersonalTokenResponse,
  /// The bearer value, beginning with `dpp_`. Shown once.
  pub plaintext_token: String,
}
