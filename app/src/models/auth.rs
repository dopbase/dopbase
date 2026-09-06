use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
  },
  Runner {
    token_id: String,
    environment_id: String,
  },
  ServiceAccount {
    service_account_id: String,
    name: String,
    token_id: String,
  },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AdminRole {
  Root,
  Admin,
  Member,
  AiAgent,
}

impl AdminRole {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Root => "root",
      Self::Admin => "admin",
      Self::Member => "member",
      Self::AiAgent => "ai_agent",
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
