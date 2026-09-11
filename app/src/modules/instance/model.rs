use serde::Serialize;
use utoipa::ToSchema;

/// Instance operational status and health indicators.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InstanceStatus {
  /// Running Dopbase server version.
  pub version: &'static str,
  /// Configured public base URL for this instance.
  pub public_url: String,
  /// Instance setup state (`ready` or `setupRequired`).
  pub initialization_state: &'static str,
  /// SQLite database operational status (`healthy`).
  pub database_health: &'static str,
  /// Master encryption key operational status (`available`).
  pub key_availability: &'static str,
  /// Server configuration reload policy (`restartRequired`).
  pub configuration_reload: &'static str,
}

/// Instance telemetry and resource summary metrics.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
  /// Running Dopbase server version.
  pub version: &'static str,
  /// Server process uptime in seconds.
  pub uptime_seconds: u64,
  /// Instance setup state (`ready` or `setupRequired`).
  pub initialization_state: &'static str,
  /// SQLite database operational status (`healthy`).
  pub database_health: &'static str,
  /// Master encryption key operational status (`available`).
  pub key_availability: &'static str,
  /// Total count of projects.
  pub projects: i64,
  /// Total count of environments across all projects.
  pub environments: i64,
  /// Total count of stored secrets across all environments.
  pub secrets: i64,
  /// Total count of administrator and member user accounts.
  pub human_users: i64,
  /// Total count of service accounts.
  pub ai_agents: i64,
  /// Count of active unrevoked runner tokens.
  pub active_runner_tokens: i64,
  /// Count of active unexpired and unrevoked agent tokens.
  pub active_agent_tokens: i64,
  /// Count of backup archives on disk.
  pub backups: i64,
  /// ISO 8601 timestamp when telemetry was sampled.
  pub observed_at: String,
}

/// Resource counts that will be purged during a factory reset.
#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FactoryResetPreview {
  /// Number of user accounts to be deleted.
  pub users: i64,
  /// Number of projects to be deleted.
  pub projects: i64,
  /// Number of environments to be deleted.
  pub environments: i64,
  /// Number of secrets to be deleted.
  pub secrets: i64,
  /// Number of runner tokens to be deleted.
  pub runner_tokens: i64,
  /// Number of backup archives to be deleted.
  pub backups: i64,
  /// Number of service accounts to be deleted.
  pub ai_agents: i64,
  /// Number of agent tokens to be deleted.
  pub agent_tokens: i64,
}

/// Request to perform an irreversible instance factory reset.
#[derive(serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FactoryResetRequest {
  /// Current root administrator password.
  pub current_password: String,
  /// Exact confirmation text, which must match `FACTORY RESET`.
  pub confirmation: String,
  /// Confirmation acknowledging complete data loss.
  pub acknowledged: bool,
}
