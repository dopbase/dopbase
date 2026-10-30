use crate::{
  modules::{environments::model::EnvironmentResponse, projects::model::ProjectResponse},
  services::authx::Level,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Who a grant is for.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalType {
  User,
  Group,
}

impl PrincipalType {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::User => "user",
      Self::Group => "group",
    }
  }

  pub fn parse(value: &str) -> Option<Self> {
    match value {
      "user" => Some(Self::User),
      "group" => Some(Self::Group),
      _ => None,
    }
  }
}

/// What a grant covers.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScopeType {
  All,
  Project,
  Environment,
  /// The audit log, for audit tokens. Never a grant's scope.
  Audit,
}

impl ScopeType {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::All => "all",
      Self::Project => "project",
      Self::Environment => "environment",
      Self::Audit => "audit",
    }
  }

  pub fn parse(value: &str) -> Option<Self> {
    match value {
      "all" => Some(Self::All),
      "project" => Some(Self::Project),
      "environment" => Some(Self::Environment),
      "audit" => Some(Self::Audit),
      _ => None,
    }
  }
}

/// One grant: a level for a user or group on all projects, a project, or
/// an environment.
#[derive(Clone, Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GrantResponse {
  /// Grant identifier with `grt_` prefix.
  pub id: String,
  pub principal_type: PrincipalType,
  /// User id (`usr_`) or group id (`grp_`).
  pub principal_id: String,
  /// The user's email or the group's name.
  pub principal_label: String,
  pub scope_type: ScopeType,
  /// Project or environment id. Empty for all projects.
  pub scope_id: String,
  /// `project` or `project/environment`. Empty for all projects.
  pub scope_label: String,
  /// The project the scope belongs to. Empty for all projects.
  pub project_id: String,
  pub level: Level,
  pub granted_by: Option<String>,
  pub created_at: String,
}

/// Give a user or group a level on all projects, a project, or an
/// environment. A second grant on the same scope replaces the first.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetGrantRequest {
  pub principal_type: PrincipalType,
  /// User id or group id.
  pub principal_id: String,
  pub scope_type: ScopeType,
  /// Project id or name for a project, environment id for an environment.
  /// Leave it out for all projects.
  #[serde(default)]
  pub scope: Option<String>,
  /// `project_manager` is for projects only. `none` is for environments
  /// only and cuts the environment out of a wider grant.
  pub level: Level,
}

/// Everyone with access to a project, and its environments.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAccessResponse {
  pub project: ProjectResponse,
  pub environments: Vec<EnvironmentResponse>,
  /// Grants on the project and on each of its environments.
  pub grants: Vec<GrantResponse>,
}

/// A group an account belongs to.
#[derive(Clone, Debug, Serialize, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct GroupRef {
  pub id: String,
  pub name: String,
}

/// What a personal token adds on top of its account's access.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TokenLimit {
  pub name: String,
  pub scope_type: ScopeType,
  pub scope_id: String,
  /// `project` or `project/environment`. Empty for all projects.
  pub scope_label: String,
  pub level: Level,
}

/// Where an account's data access comes from.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserAccessResponse {
  pub user_id: String,
  /// The account's email, or an AI agent's name.
  pub email: String,
  /// The account's role. `Root` for root.
  pub role_name: String,
  /// The deepest level the account's role allows.
  pub max_level: Level,
  /// The limit of the personal token that asked, when one did.
  pub token: Option<TokenLimit>,
  pub groups: Vec<GroupRef>,
  /// The account's own grants and those of its groups. A group grant has
  /// the group as its principal.
  pub grants: Vec<GrantResponse>,
}
