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
  /// SQLite database operational status (`healthy`). The request fails when
  /// the database doesn't answer.
  pub database_health: &'static str,
  /// How the master key file compares with the key in use: `available` when
  /// they match, `changed`, `missing` or `unreadable` when the server would
  /// not start again.
  pub key_availability: &'static str,
  /// Server configuration reload policy (`restartRequired`).
  pub configuration_reload: &'static str,
}

/// Instance telemetry, resource counts, and the results of the health checks.
///
/// Counts are `null` when their query failed, so one broken query never hides
/// the rest of the page. `summary`, `checks`, `health`, and `activity` are only
/// filled in for viewers who hold `instance.view`.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
  /// Running Dopbase server version.
  pub version: &'static str,
  /// Server process uptime in seconds.
  pub uptime_seconds: u64,
  /// Instance setup state (`ready` or `setupRequired`).
  pub initialization_state: &'static str,
  /// Operating system and CPU architecture of the server.
  pub platform: Platform,
  /// Total count of projects.
  pub projects: Option<i64>,
  /// Total count of environments across all projects.
  pub environments: Option<i64>,
  /// Total count of stored secrets across all environments.
  pub secrets: Option<i64>,
  /// Accounts that sign in, root included.
  pub human_users: Option<i64>,
  /// Accounts that use tokens only, such as AI agents.
  pub ai_agents: Option<i64>,
  /// Count of active unrevoked runner tokens.
  pub active_runner_tokens: Option<i64>,
  /// Active, unexpired personal tokens of AI agent accounts.
  pub active_agent_tokens: Option<i64>,
  /// Count of backup archives on disk.
  pub backups: i64,
  /// Check counts by result.
  pub summary: Option<StatusSummary>,
  /// Every check, most severe first.
  pub checks: Option<Vec<Check>>,
  /// Readings behind the four health cards.
  pub health: Option<HealthReport>,
  /// What happened in the last 24 hours.
  pub activity: Option<Activity>,
  /// The latest published release, when the opt-in `update_check` setting
  /// is on and GitHub answered at least once.
  pub latest_release: Option<LatestRelease>,
  /// ISO 8601 timestamp when telemetry was sampled.
  pub observed_at: String,
}

/// The latest Dopbase release, as last seen on GitHub.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LatestRelease {
  /// Release tag, such as `v0.3.1`.
  pub version: String,
  /// Whether it is newer than the running server.
  pub update_available: bool,
  /// Release notes page.
  pub url: String,
  /// ISO 8601 timestamp of the lookup.
  pub checked_at: String,
}

/// Operating system and CPU architecture the server was built for.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Platform {
  /// For example `linux` or `macos`.
  pub os: &'static str,
  /// For example `x86_64` or `aarch64`.
  pub arch: &'static str,
}

/// How a check came out. Ordered most severe first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
  /// Needs fixing now.
  Crit,
  /// Worth fixing soon.
  Warn,
  /// A suggestion that depends on the setup.
  Tip,
  /// Nothing to do.
  Ok,
}

/// One health check result.
#[derive(Clone, Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Check {
  /// Stable identifier, such as `backup.age`.
  pub id: &'static str,
  pub status: CheckStatus,
  /// One line saying what was found.
  pub title: String,
  /// Why it matters, or what was measured.
  pub detail: String,
  /// Short reading shown beside a passing check, such as `3 ms`.
  pub value: Option<String>,
  /// Admin UI path or documentation URL that fixes the problem. Returned to
  /// everyone; the UI hides fixes the viewer cannot use.
  pub fix_path: Option<String>,
  /// ISO 8601 timestamp of the run that produced this result.
  pub last_run_at: String,
}

/// Check counts by result.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StatusSummary {
  /// Failing checks by severity.
  pub critical: usize,
  pub warnings: usize,
  pub tips: usize,
  pub passed: usize,
  /// ISO 8601 timestamp of the newest check run.
  pub checked_at: String,
}

/// Readings behind the Database, Master key, Disk, and Audit log cards.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthReport {
  pub database: DatabaseHealth,
  pub master_key: MasterKeyHealth,
  /// `null` when the data directory can't be read.
  pub disk: Option<DiskHealth>,
  pub audit_log: AuditLogHealth,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseHealth {
  pub status: CheckStatus,
  /// Round trip of a trivial query, in milliseconds. `null` when it failed.
  pub ping_ms: Option<f64>,
  /// Size of the database file plus its write-ahead log, in bytes.
  pub size_bytes: Option<u64>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MasterKeyHealth {
  pub status: CheckStatus,
  /// `matches`, `changed`, `missing`, or `unreadable`: how the key file on
  /// disk compares with the key the server loaded at startup.
  pub file: &'static str,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DiskHealth {
  /// Size of everything in the data directory, plus the database and its
  /// write-ahead log when they live outside it, in bytes.
  pub used_bytes: u64,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogHealth {
  pub status: CheckStatus,
  /// Records the last verification walked. `null` before the first run.
  pub events: Option<u64>,
  /// Whether the hash chain verified. `null` when verification could not run.
  pub intact: Option<bool>,
  /// ISO 8601 timestamp of the last verification.
  pub checked_at: Option<String>,
}

/// Activity over the last 24 hours, in hourly buckets oldest first. The
/// last bucket is the current hour.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
  /// ISO 8601 start of each hourly bucket.
  pub hours: Vec<String>,
  pub secret_reads: SecretReads,
  pub failed_sign_ins: FailedSignIns,
  pub rate_limited: RateLimited,
  pub active_sessions: ActiveSessions,
}

/// Secret values read by runners or shown, exported, or copied in the UI.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SecretReads {
  pub total: i64,
  pub hourly: Vec<i64>,
  /// Distinct runner tokens that read secrets.
  pub runner_tokens: i64,
  /// Distinct people who read secrets.
  pub people: i64,
  /// Distinct AI agents that read secrets.
  pub ai_agents: i64,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FailedSignIns {
  pub total: i64,
  pub hourly: Vec<i64>,
  /// Emails the attempts named. `null` without `audit.read`.
  pub accounts: Option<Vec<String>>,
  /// Client addresses the attempts came from. `null` without `audit.read`.
  pub addresses: Option<Vec<String>>,
}

/// Requests refused by the sign-in and password rate limits. Counted in
/// memory, so the count starts again when the server restarts.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RateLimited {
  pub total: u64,
  pub hourly: Vec<u64>,
}

/// Sessions that are signed in right now.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActiveSessions {
  pub browser: i64,
  pub cli: i64,
}

/// Resource counts that will be purged during a factory reset.
#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FactoryResetPreview {
  /// Number of accounts to be deleted, AI agents included.
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
  /// Number of AI agent accounts to be deleted.
  pub ai_agents: i64,
  /// Number of personal tokens to be deleted.
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
