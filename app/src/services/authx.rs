//! Authorization: roles, capabilities, grants, and access levels.
//!
//! Two questions decide every request. A role's **capabilities** say what an
//! account may do to the instance, such as managing users or reading the
//! audit log. **Grants** say how far into a project or environment it may
//! go. Root holds every capability and needs no grants. Role names never
//! appear in a decision, so a new role is a row in `roles`, not new code.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sqlx::SqliteConnection;
use utoipa::ToSchema;

/// How far into a project or environment an account may go. Each level
/// includes every level below it.
#[derive(
  Clone, Copy, Debug, Default, Deserialize, Serialize, ToSchema, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "snake_case")]
pub enum Level {
  /// The environment does not exist as far as the account can tell.
  #[default]
  None,
  /// Names, versions, history entries, and layout. No values.
  Metadata,
  /// Reveal, export, and run with values.
  Read,
  /// Change, import, and roll back secrets.
  Write,
  /// Rename and delete the environment and manage its runner tokens.
  Manage,
  /// Manage every environment of a project, create environments, and grant
  /// the project to others. Granted on a project or on all projects only.
  ProjectManager,
}

impl Level {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::None => "none",
      Self::Metadata => "metadata",
      Self::Read => "read",
      Self::Write => "write",
      Self::Manage => "manage",
      Self::ProjectManager => "project_manager",
    }
  }

  pub fn parse(value: &str) -> Option<Self> {
    match value {
      "none" => Some(Self::None),
      "metadata" => Some(Self::Metadata),
      "read" => Some(Self::Read),
      "write" => Some(Self::Write),
      "manage" => Some(Self::Manage),
      "project_manager" => Some(Self::ProjectManager),
      _ => None,
    }
  }
}

/// An instance-level power a role can hold. Adding a variant is a code
/// change because it is a new kind of power; giving it to a role is data.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq, PartialOrd, Ord)]
pub enum Capability {
  #[serde(rename = "users.view")]
  UsersView,
  #[serde(rename = "users.manage")]
  UsersManage,
  #[serde(rename = "roles.manage")]
  RolesManage,
  #[serde(rename = "groups.manage")]
  GroupsManage,
  #[serde(rename = "grants.manage")]
  GrantsManage,
  #[serde(rename = "grants.all_projects")]
  GrantsAllProjects,
  #[serde(rename = "projects.create")]
  ProjectsCreate,
  #[serde(rename = "audit.read")]
  AuditRead,
  #[serde(rename = "audit.verify")]
  AuditVerify,
  #[serde(rename = "access.report")]
  AccessReport,
  #[serde(rename = "tokens.personal")]
  TokensPersonal,
  #[serde(rename = "tokens.revoke_others")]
  TokensRevokeOthers,
  #[serde(rename = "sso.manage")]
  SsoManage,
  #[serde(rename = "instance.view")]
  InstanceView,
  #[serde(rename = "instance.backup")]
  InstanceBackup,
  #[serde(rename = "instance.reset")]
  InstanceReset,
  #[serde(rename = "password.breakglass")]
  PasswordBreakglass,
}

impl Capability {
  pub const ALL: [Self; 17] = [
    Self::UsersView,
    Self::UsersManage,
    Self::RolesManage,
    Self::GroupsManage,
    Self::GrantsManage,
    Self::GrantsAllProjects,
    Self::ProjectsCreate,
    Self::AuditRead,
    Self::AuditVerify,
    Self::AccessReport,
    Self::TokensPersonal,
    Self::TokensRevokeOthers,
    Self::SsoManage,
    Self::InstanceView,
    Self::InstanceBackup,
    Self::InstanceReset,
    Self::PasswordBreakglass,
  ];

  /// Capabilities only root holds. No role can be given them.
  pub const ROOT_ONLY: [Self; 6] = [
    Self::RolesManage,
    Self::GrantsAllProjects,
    Self::SsoManage,
    Self::InstanceBackup,
    Self::InstanceReset,
    Self::PasswordBreakglass,
  ];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::UsersView => "users.view",
      Self::UsersManage => "users.manage",
      Self::RolesManage => "roles.manage",
      Self::GroupsManage => "groups.manage",
      Self::GrantsManage => "grants.manage",
      Self::GrantsAllProjects => "grants.all_projects",
      Self::ProjectsCreate => "projects.create",
      Self::AuditRead => "audit.read",
      Self::AuditVerify => "audit.verify",
      Self::AccessReport => "access.report",
      Self::TokensPersonal => "tokens.personal",
      Self::TokensRevokeOthers => "tokens.revoke_others",
      Self::SsoManage => "sso.manage",
      Self::InstanceView => "instance.view",
      Self::InstanceBackup => "instance.backup",
      Self::InstanceReset => "instance.reset",
      Self::PasswordBreakglass => "password.breakglass",
    }
  }

  pub fn parse(value: &str) -> Option<Self> {
    Self::ALL
      .into_iter()
      .find(|capability| capability.as_str() == value)
  }
}

/// How a role's holders prove who they are.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SignIn {
  /// Password or single sign-on.
  Interactive,
  /// Personal tokens only, as for AI agents.
  TokenOnly,
}

/// What an account may do, resolved from its role when it authenticates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Access {
  pub is_root: bool,
  /// `None` for root, which has no role.
  pub role_id: Option<String>,
  pub role_name: String,
  capabilities: BTreeSet<Capability>,
  /// The deepest level any grant can give this account.
  pub max_level: Level,
  pub sign_in: SignIn,
}

impl Access {
  pub fn root() -> Self {
    Self {
      is_root: true,
      role_id: None,
      role_name: "Root".into(),
      capabilities: Capability::ALL.into_iter().collect(),
      max_level: Level::ProjectManager,
      sign_in: SignIn::Interactive,
    }
  }

  /// A role's access. Root-only capabilities are dropped, so a role row
  /// can never hand them out.
  pub fn role(
    role_id: impl Into<String>,
    role_name: impl Into<String>,
    capabilities: impl IntoIterator<Item = Capability>,
    max_level: Level,
    sign_in: SignIn,
  ) -> Self {
    Self {
      is_root: false,
      role_id: Some(role_id.into()),
      role_name: role_name.into(),
      capabilities: capabilities
        .into_iter()
        .filter(|capability| !Capability::ROOT_ONLY.contains(capability))
        .collect(),
      max_level,
      sign_in,
    }
  }

  /// An account that matches no role holds nothing. It can authenticate,
  /// but every decision denies it.
  pub fn none() -> Self {
    Self {
      is_root: false,
      role_id: None,
      role_name: String::new(),
      capabilities: BTreeSet::new(),
      max_level: Level::None,
      sign_in: SignIn::Interactive,
    }
  }

  pub fn has(
    &self,
    capability: Capability,
  ) -> bool {
    self.is_root || self.capabilities.contains(&capability)
  }

  pub fn capabilities(&self) -> impl Iterator<Item = Capability> + '_ {
    self.capabilities.iter().copied()
  }
}

#[derive(sqlx::FromRow)]
struct RoleRow {
  is_root: i64,
  role_id: Option<String>,
  name: Option<String>,
  max_level: Option<String>,
  sign_in: Option<String>,
}

/// The access of an account by id, or `None` when it does not exist or is
/// disabled.
pub async fn load_access(
  conn: &mut SqliteConnection,
  admin_id: &str,
) -> Result<Option<Access>, sqlx::Error> {
  let row: Option<RoleRow> = sqlx::query_as(
    "SELECT a.is_root, r.id AS role_id, r.name, r.max_level, r.sign_in FROM admins a LEFT JOIN roles r ON r.id = a.role_id WHERE a.id = ? AND a.status = 'active'",
  )
  .bind(admin_id)
  .fetch_optional(&mut *conn)
  .await?;
  let Some(row) = row else {
    return Ok(None);
  };
  if row.is_root == 1 {
    return Ok(Some(Access::root()));
  }
  let (Some(role_id), Some(name), Some(max_level), Some(sign_in)) =
    (row.role_id, row.name, row.max_level, row.sign_in)
  else {
    return Ok(Some(Access::none()));
  };
  let capabilities: Vec<String> =
    sqlx::query_scalar("SELECT capability FROM role_capabilities WHERE role_id = ?")
      .bind(&role_id)
      .fetch_all(&mut *conn)
      .await?;
  Ok(Some(Access::role(
    role_id,
    name,
    capabilities
      .iter()
      .filter_map(|value| Capability::parse(value)),
    Level::parse(&max_level).unwrap_or(Level::None),
    if sign_in == "token_only" {
      SignIn::TokenOnly
    } else {
      SignIn::Interactive
    },
  )))
}

/// What a grant covers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Scope {
  All,
  Project(String),
  Environment(String),
}

impl Scope {
  fn from_row(
    scope_type: &str,
    scope_id: String,
  ) -> Option<Self> {
    match scope_type {
      "all" => Some(Self::All),
      "project" => Some(Self::Project(scope_id)),
      "environment" => Some(Self::Environment(scope_id)),
      _ => None,
    }
  }
}

/// One grant that applies to an account, held directly or through a group.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grant {
  pub scope: Scope,
  pub level: Level,
}

impl Grant {
  pub fn new(
    scope: Scope,
    level: Level,
  ) -> Self {
    Self { scope, level }
  }
}

/// The environment a decision is about.
#[derive(Clone, Copy, Debug)]
pub struct EnvironmentTarget<'a> {
  pub project_id: &'a str,
  pub environment_id: &'a str,
  /// Protected environments are reached only by an environment grant or a
  /// project-manager grant.
  pub protected: bool,
}

/// The highest level among grants of one tier, or `None` when the tier has
/// no grant at all.
fn tier_level<'a>(grants: impl Iterator<Item = &'a Grant>) -> Option<Level> {
  grants.map(|grant| grant.level).max()
}

/// The level the grants give on one environment, clamped to `ceiling`.
///
/// The most specific tier with any grant decides: environment, then
/// project, then all projects. Within a tier the highest level wins, so a
/// group grant and a direct grant on the same scope combine upward, while a
/// direct environment grant can still narrow what a project grant gives.
pub fn environment_level(
  grants: &[Grant],
  target: EnvironmentTarget<'_>,
  ceiling: Level,
) -> Level {
  let environment =
    tier_level(grants.iter().filter(
      |grant| matches!(&grant.scope, Scope::Environment(id) if id == target.environment_id),
    ));
  let inherited = || {
    tier_level(
      grants
        .iter()
        .filter(|grant| matches!(&grant.scope, Scope::Project(id) if id == target.project_id)),
    )
    .or_else(|| tier_level(grants.iter().filter(|grant| grant.scope == Scope::All)))
    .map(|level| {
      // A project manager manages every environment of the project. Any
      // lower inherited level stops at a protected environment.
      if level == Level::ProjectManager {
        Level::Manage
      } else if target.protected {
        Level::None
      } else {
        level
      }
    })
  };
  environment
    .or_else(inherited)
    .unwrap_or(Level::None)
    .min(ceiling)
    .min(Level::Manage)
}

/// The level the grants give on a project itself, clamped to `ceiling`.
/// Only project and all-projects grants count here; environment grants
/// make a project visible without giving rights over it.
pub fn project_level(
  grants: &[Grant],
  project_id: &str,
  ceiling: Level,
) -> Level {
  tier_level(
    grants
      .iter()
      .filter(|grant| matches!(&grant.scope, Scope::Project(id) if id == project_id)),
  )
  .or_else(|| tier_level(grants.iter().filter(|grant| grant.scope == Scope::All)))
  .unwrap_or(Level::None)
  .min(ceiling)
}

/// Every grant that applies to an account: its own and its groups'.
pub async fn grants_for(
  conn: &mut SqliteConnection,
  admin_id: &str,
) -> Result<Vec<Grant>, sqlx::Error> {
  let rows: Vec<(String, String, String)> = sqlx::query_as(
    "SELECT scope_type, scope_id, level FROM grants WHERE (principal_type = 'user' AND principal_id = ?1) OR (principal_type = 'group' AND principal_id IN (SELECT group_id FROM group_members WHERE user_id = ?1))",
  )
  .bind(admin_id)
  .fetch_all(&mut *conn)
  .await?;
  Ok(
    rows
      .into_iter()
      .filter_map(|(scope_type, scope_id, level)| {
        Some(Grant::new(
          Scope::from_row(&scope_type, scope_id)?,
          Level::parse(&level)?,
        ))
      })
      .collect(),
  )
}

/// The level an account has on one environment, or `None` when the
/// environment does not exist.
pub async fn level_on_environment(
  conn: &mut SqliteConnection,
  admin_id: &str,
  access: &Access,
  environment_id: &str,
) -> Result<Option<Level>, sqlx::Error> {
  let row: Option<(String, i64)> =
    sqlx::query_as("SELECT project_id, protected FROM environments WHERE id = ?")
      .bind(environment_id)
      .fetch_optional(&mut *conn)
      .await?;
  let Some((project_id, protected)) = row else {
    return Ok(None);
  };
  if access.is_root {
    return Ok(Some(Level::Manage));
  }
  let grants = grants_for(conn, admin_id).await?;
  Ok(Some(environment_level(
    &grants,
    EnvironmentTarget {
      project_id: &project_id,
      environment_id,
      protected: protected == 1,
    },
    access.max_level,
  )))
}
