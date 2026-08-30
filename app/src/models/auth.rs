use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug)]
pub enum AuthIdentity {
  Admin {
    admin_id: String,
    email: String,
    session_id: String,
    kind: SessionKind,
    recent_auth_at: DateTime<Utc>,
    csrf_hash: Option<Vec<u8>>,
  },
  Runner {
    token_id: String,
    environment_id: String,
  },
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
