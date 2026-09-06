use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Service account (AI agent) representation.
#[derive(Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceAccountResponse {
  /// Unique service account identifier with `aia_` prefix.
  pub id: String,
  /// Service account name.
  pub name: String,
  /// Account role (`ai_agent`).
  pub role: &'static str,
  /// ISO 8601 creation timestamp.
  pub created_at: String,
  /// ISO 8601 last update timestamp.
  pub updated_at: String,
}

/// Agent bearer token metadata.
#[derive(Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentTokenResponse {
  /// Unique token identifier with `ait_` prefix.
  pub id: String,
  /// Associated service account identifier (`aia_...`).
  pub service_account_id: String,
  /// Human-readable token name or label.
  pub name: String,
  /// ISO 8601 creation timestamp.
  pub created_at: String,
  /// Optional ISO 8601 expiration timestamp.
  pub expires_at: Option<String>,
  /// Optional ISO 8601 timestamp of last token usage.
  pub last_used_at: Option<String>,
  /// Optional ISO 8601 revocation timestamp if revoked.
  pub revoked_at: Option<String>,
}

/// Newly created agent token containing the plaintext secret.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatedAgentTokenResponse {
  /// Metadata for the generated token.
  pub token: AgentTokenResponse,
  /// Plaintext bearer token secret string with `dpa_` prefix. Only shown once upon creation.
  pub plaintext_token: String,
}

/// Request to create a new service account.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateServiceAccountRequest {
  /// Unique name for the service account (2 to 100 characters).
  pub name: String,
}

/// Request to create a new agent bearer token.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateAgentTokenRequest {
  /// Name or label for the token.
  pub name: String,
  /// Optional ISO 8601 expiration timestamp (must expire within 90 days). Defaults to 30 days.
  pub expires_at: Option<String>,
}
