use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::services::authx::{Access, Level, Scope};

#[derive(Clone, Debug)]
pub enum AuthIdentity {
  Admin {
    admin_id: String,
    email: String,
    role: AdminRole,
    session_id: String,
    kind: SessionKind,
    recent_auth_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    csrf_hash: Option<Vec<u8>>,
    /// The account's capabilities and level ceiling, from its role.
    access: Access,
  },
  Runner {
    token_id: String,
    environment_id: String,
  },
  /// A personal token. It acts for its account, within the token's scope
  /// and level, and never holds the account's instance capabilities.
  Token {
    admin_id: String,
    /// The account's email, or an AI agent's name.
    email: String,
    token_id: String,
    token_name: String,
    /// The account's role, for its level ceiling.
    access: Access,
    scope: Scope,
    level: Level,
    /// The accountable owner of an AI agent account. The agent never
    /// reaches more than its owner does.
    owner_id: Option<String>,
    /// An audit token reads the audit log as its account would, and no
    /// project data.
    audit: bool,
  },
}

impl AuthIdentity {
  /// The actor type audit records store for this identity. The root user
  /// is recorded as `root` so its actions stand apart from other
  /// administrators.
  pub fn audit_actor_type(&self) -> &'static str {
    match self {
      Self::Admin { role, .. } => role.audit_actor_type(),
      Self::Runner { .. } => "runner",
      Self::Token { .. } => "personal_token",
    }
  }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AdminRole {
  Root,
  Admin,
  Member,
  #[serde(rename = "ai_agent")]
  AiAgent,
}

impl AdminRole {
  /// The actor type audit records store for a user with this role: `root`
  /// for the root user, `admin` for everyone else.
  pub fn audit_actor_type(self) -> &'static str {
    match self {
      Self::Root => "root",
      _ => "admin",
    }
  }

  pub fn as_str(self) -> &'static str {
    match self {
      Self::Root => "root",
      Self::Admin => "admin",
      Self::Member => "member",
      Self::AiAgent => "ai_agent",
    }
  }

  /// The role of a person who signs in, read from `admins.role`. AI agents
  /// never sign in, so they and unknown values give `None`.
  // TODO(rbac): maps the fixed role names. Load the account's roles from
  // the role tables instead once roles are configurable.
  pub fn parse_human(value: &str) -> Option<Self> {
    match value {
      "root" => Some(Self::Root),
      "admin" => Some(Self::Admin),
      "member" => Some(Self::Member),
      _ => None,
    }
  }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SessionKind {
  Browser,
  Cli,
}

impl SessionKind {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Browser => "browser",
      Self::Cli => "cli",
    }
  }
}
