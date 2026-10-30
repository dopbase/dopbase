use crate::services::authx::{Capability, Level, SignIn};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// What a role's personal tokens are for.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TokenScope {
  /// No personal tokens.
  None,
  /// Project data, up to the token level.
  Data,
  /// The audit log only.
  Audit,
}

impl TokenScope {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::None => "none",
      Self::Data => "data",
      Self::Audit => "audit",
    }
  }

  pub fn parse(value: &str) -> Option<Self> {
    match value {
      "none" => Some(Self::None),
      "data" => Some(Self::Data),
      "audit" => Some(Self::Audit),
      _ => None,
    }
  }
}

/// A role: what its holders may do on the instance and how deep into
/// project data their grants can reach.
#[derive(Clone, Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RoleResponse {
  /// Role identifier with `rol_` prefix.
  pub id: String,
  pub name: String,
  pub description: String,
  /// Built-in roles can't be changed or deleted.
  pub builtin: bool,
  pub capabilities: Vec<Capability>,
  /// The deepest level any grant can give holders.
  pub max_level: Level,
  pub sign_in: SignIn,
  /// Holders need an accountable owner, as AI agents do.
  pub requires_owner: bool,
  pub token_scope: TokenScope,
  pub token_max_level: Level,
  pub token_default_ttl_days: i64,
  pub token_max_ttl_days: i64,
  /// Display order, lowest first.
  pub rank: i64,
  /// Accounts that hold the role.
  pub members: i64,
  /// Whether the caller may give this role to someone.
  pub assignable: bool,
  pub created_at: String,
  pub updated_at: String,
}

/// A custom role. Only root creates them.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateRoleRequest {
  /// Unique, at most 64 characters.
  pub name: String,
  #[serde(default)]
  pub description: Option<String>,
  /// Root-only capabilities can't be given to a role.
  #[serde(default)]
  pub capabilities: Vec<Capability>,
  /// `metadata`, `read`, `write`, `manage`, or `project_manager`.
  pub max_level: Level,
  /// `interactive` (default) or `token_only`. Token-only roles always need
  /// an owner. It can't change after the role is created.
  #[serde(default)]
  pub sign_in: Option<SignIn>,
  /// Defaults to `data`, or `none` when the role lacks tokens.personal.
  #[serde(default)]
  pub token_scope: Option<TokenScope>,
  /// `metadata`, `read`, or `write`, no deeper than `maxLevel`. Defaults to
  /// the lower of `write` and `maxLevel`.
  #[serde(default)]
  pub token_max_level: Option<Level>,
  /// Defaults to 30.
  #[serde(default)]
  pub token_default_ttl_days: Option<i64>,
  /// Defaults to 365. At most 1095.
  #[serde(default)]
  pub token_max_ttl_days: Option<i64>,
  /// Defaults to 100, after the built-in roles.
  #[serde(default)]
  pub rank: Option<i64>,
}

/// Change a custom role. Fields left out stay as they are. Holders' access
/// changes on their next request.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRoleRequest {
  #[serde(default)]
  pub name: Option<String>,
  #[serde(default)]
  pub description: Option<String>,
  #[serde(default)]
  pub capabilities: Option<Vec<Capability>>,
  #[serde(default)]
  pub max_level: Option<Level>,
  #[serde(default)]
  pub token_scope: Option<TokenScope>,
  #[serde(default)]
  pub token_max_level: Option<Level>,
  #[serde(default)]
  pub token_default_ttl_days: Option<i64>,
  #[serde(default)]
  pub token_max_ttl_days: Option<i64>,
  #[serde(default)]
  pub rank: Option<i64>,
}
