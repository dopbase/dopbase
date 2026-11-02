//! The health checks and 24-hour activity behind the Status page.
//!
//! Every check runs on its own. One that errors comes back as a warning
//! named after it, so a broken query never hides the other results.

use std::time::Instant;

use anyhow::Result;
use chrono::{DateTime, Duration, DurationRound, Utc};
use sqlx::SqlitePool;

use super::model::{
  ActiveSessions, Activity, AuditLogHealth, Check, CheckStatus, DatabaseHealth, DiskHealth,
  FailedSignIns, HealthReport, MasterKeyHealth, RateLimited, SecretReads,
};
use crate::{
  config::PublicUrlSource,
  services::status::{AuditCheck, ReleaseState},
  state::AppState,
};

const CONFIG_DOCS: &str = "https://docs.dopbase.com/cli/serve#configuration-files";
/// A backup older than this, with secrets changed since, is critical.
const BACKUP_STALE_DAYS: i64 = 7;
/// Tokens expiring within this many days are reported.
const TOKEN_WARNING_DAYS: i64 = 7;
/// A ping slower than this is reported.
const SLOW_PING_MS: f64 = 500.0;
/// Free space thresholds. Absolute, because a few percent of a large disk
/// is still plenty for a SQLite file.
const CRIT_FREE_BYTES: u64 = 1 << 30;
const WARN_FREE_BYTES: u64 = 5 << 30;

/// Audit actions that change a stored secret value.
const SECRET_CHANGES: &str = "'secret.created','secret.updated','secret.deleted','secret.imported','secret.rolled_back','secret.copied','secret.bulk_copied'";
/// Audit actions that hand a secret value to someone.
const SECRET_READS: &str =
  "'secret.runtime_accessed','secret.revealed','secret.version_revealed','secret.exported'";

/// What the viewer may see beyond the check results themselves.
pub struct Viewer {
  /// Names of runner and personal tokens.
  pub token_names: bool,
  /// Emails and client addresses of failed sign-ins.
  pub sign_in_details: bool,
}

pub struct Report {
  pub checks: Vec<Check>,
  pub health: HealthReport,
}

/// Run every check. The audit chain result comes from the cache.
pub async fn run(
  state: &AppState,
  audit: &AuditCheck,
  release: Option<&ReleaseState>,
  viewer: &Viewer,
) -> Report {
  let now = Utc::now();
  let pool = state.db.pool();
  let (database_check, database) = database(state, now).await;
  let (key_check, master_key) = master_key(state, now);
  let (disk_check, disk) = match disk(state, now) {
    Ok(Some((check, health))) => (Some(check), Some(health)),
    Ok(None) => (None, None),
    Err(error) => (
      Some(failed("disk.space", "free disk space", &error, now)),
      None,
    ),
  };
  let (audit_check, audit_log) = audit_chain(audit);
  let newest = newest_backup(state);

  let mut checks = vec![database_check, key_check];
  checks.extend(disk_check);
  checks.push(audit_check);
  checks.push(settle(
    "backup.age",
    "backup age",
    backup_age(pool, newest.as_ref(), now).await,
    now,
  ));
  if let Some(newest) = &newest {
    checks.push(settle(
      "backup.offsite",
      "backup downloads",
      backup_offsite(pool, newest, now).await,
      now,
    ));
  }
  checks.push(settle(
    "tokens.expiring",
    "token expiry",
    tokens_expiring(pool, viewer, now).await,
    now,
  ));
  checks.push(settle(
    "audit.retention",
    "audit retention",
    audit_retention(state, now).await,
    now,
  ));
  checks.push(public_url(state, now));
  checks.push(https(state, now));
  checks.push(docs(state, now));
  checks.push(settle(
    "people.managers",
    "who can manage the instance",
    managers(pool, now).await,
    now,
  ));
  checks.extend(release.and_then(|release| version(release, now)));
  // Stable sort: most severe first, the order above within a severity.
  checks.sort_by_key(|check| check.status);
  Report {
    checks,
    health: HealthReport {
      database,
      master_key,
      disk,
      audit_log,
    },
  }
}

fn check(
  id: &'static str,
  status: CheckStatus,
  title: impl Into<String>,
  detail: impl Into<String>,
  now: DateTime<Utc>,
) -> Check {
  Check {
    id,
    status,
    title: title.into(),
    detail: detail.into(),
    value: None,
    fix_path: None,
    last_run_at: now.to_rfc3339(),
  }
}

impl Check {
  fn value(
    mut self,
    value: impl Into<String>,
  ) -> Self {
    self.value = Some(value.into());
    self
  }

  fn fix(
    mut self,
    path: impl Into<String>,
  ) -> Self {
    self.fix_path = Some(path.into());
    self
  }
}

/// A check that could not run. The error goes to the server log, never to
/// the page.
fn failed(
  id: &'static str,
  what: &str,
  error: &dyn std::fmt::Display,
  now: DateTime<Utc>,
) -> Check {
  tracing::warn!(check = id, %error, "status check failed to run");
  check(
    id,
    CheckStatus::Warn,
    format!("Couldn't check {what}"),
    "The check failed to run. The server log has the error.",
    now,
  )
}

fn settle(
  id: &'static str,
  what: &str,
  result: Result<Check>,
  now: DateTime<Utc>,
) -> Check {
  result.unwrap_or_else(|error| failed(id, what, &error, now))
}

async fn database(
  state: &AppState,
  now: DateTime<Utc>,
) -> (Check, DatabaseHealth) {
  let started = Instant::now();
  let ping = state.db.ping().await;
  let ping_ms = started.elapsed().as_secs_f64() * 1000.0;
  let size_bytes = crate::config::database_path(&state.config.database_url)
    .ok()
    .and_then(|path| {
      let main = std::fs::metadata(&path).ok()?.len();
      let mut wal = path.into_os_string();
      wal.push("-wal");
      Some(main + std::fs::metadata(wal).map_or(0, |meta| meta.len()))
    });
  let (result, ping_ms) = match ping {
    Err(error) => {
      tracing::warn!(%error, "status database ping failed");
      (
        check(
          "database.ping",
          CheckStatus::Crit,
          "The database isn't answering",
          "Queries fail, so most pages won't load. The server log has the error.",
          now,
        ),
        None,
      )
    }
    Ok(()) if ping_ms > SLOW_PING_MS => (
      check(
        "database.ping",
        CheckStatus::Warn,
        "The database is slow to answer",
        format!(
          "A trivial query took {}. The disk may be overloaded.",
          format_ms(ping_ms)
        ),
        now,
      )
      .value(format_ms(ping_ms)),
      Some(ping_ms),
    ),
    Ok(()) => (
      check(
        "database.ping",
        CheckStatus::Ok,
        "Database answers",
        format!("A trivial query took {}.", format_ms(ping_ms)),
        now,
      )
      .value(format_ms(ping_ms)),
      Some(ping_ms),
    ),
  };
  let health = DatabaseHealth {
    status: result.status,
    ping_ms,
    size_bytes,
  };
  (result, health)
}

/// Compare the key file with the key the server loaded at startup. The
/// server refuses to start on a key that doesn't match the database, so the
/// key in memory is always right; the file is what can drift.
fn master_key(
  state: &AppState,
  now: DateTime<Utc>,
) -> (Check, MasterKeyHealth) {
  const RESTART: &str = "Download the key in use now and put it back in place before you restart.";
  let file = state
    .crypto
    .key_file_state(&state.config.master_key.path)
    .as_str();
  let result = match file {
    "matches" => check(
      "master_key.file",
      CheckStatus::Ok,
      "Master key matches the database",
      "The key file on disk is the key the server is using.",
      now,
    ),
    "changed" => check(
      "master_key.file",
      CheckStatus::Crit,
      "The key file no longer matches the key in use",
      RESTART,
      now,
    )
    .fix("/backups"),
    "missing" => check(
      "master_key.file",
      CheckStatus::Crit,
      "The master key file is missing",
      RESTART,
      now,
    )
    .fix("/backups"),
    _ => check(
      "master_key.file",
      CheckStatus::Crit,
      "The master key file can't be read",
      RESTART,
      now,
    )
    .fix("/backups"),
  };
  let health = MasterKeyHealth {
    status: result.status,
    file,
  };
  (result, health)
}

#[cfg(unix)]
fn disk(
  state: &AppState,
  now: DateTime<Utc>,
) -> Result<Option<(Check, DiskHealth)>> {
  let stats = nix::sys::statvfs::statvfs(&state.config.data_dir)?;
  let block = stats.fragment_size() as u64;
  let free = stats.blocks_available() as u64 * block;
  let total = stats.blocks() as u64 * block;
  let detail = format!("{} free of {}.", format_bytes(free), format_bytes(total));
  let result = if free < CRIT_FREE_BYTES {
    check(
      "disk.space",
      CheckStatus::Crit,
      "The data disk is almost full",
      format!("{detail} The database stops accepting changes when it fills up."),
      now,
    )
  } else if free < WARN_FREE_BYTES {
    check(
      "disk.space",
      CheckStatus::Warn,
      "The data disk is filling up",
      format!("{detail} Free some space or move the data directory."),
      now,
    )
  } else {
    check("disk.space", CheckStatus::Ok, "Disk has room", detail, now)
  }
  .value(format!("{} free", format_bytes(free)));
  let health = DiskHealth {
    status: result.status,
    free_bytes: free,
    total_bytes: total,
  };
  Ok(Some((result, health)))
}

#[cfg(not(unix))]
fn disk(
  _state: &AppState,
  _now: DateTime<Utc>,
) -> Result<Option<(Check, DiskHealth)>> {
  Ok(None)
}

fn audit_chain(audit: &AuditCheck) -> (Check, AuditLogHealth) {
  let now = audit.checked_at;
  let result = match &audit.outcome {
    Err(reason) => check(
      "audit.chain",
      CheckStatus::Warn,
      "Couldn't check the audit log chain",
      reason.clone(),
      now,
    ),
    Ok(outcome) if outcome.intact => check(
      "audit.chain",
      CheckStatus::Ok,
      "Audit log chain is intact",
      "Every record's hash matches the one before it.",
      now,
    )
    .value(format!("{} events", format_count(outcome.records))),
    Ok(outcome) => check(
      "audit.chain",
      CheckStatus::Crit,
      "The audit log chain is broken",
      match outcome.broken_seq {
        Some(seq) => format!(
          "Record {seq} doesn't match the hash chain, so the log may have been changed outside Dopbase."
        ),
        None => "The log may have been changed outside Dopbase.".to_owned(),
      },
      now,
    )
    .fix("/audit"),
  };
  let health = AuditLogHealth {
    status: result.status,
    events: audit.outcome.as_ref().ok().map(|outcome| outcome.records),
    intact: audit.outcome.as_ref().ok().map(|outcome| outcome.intact),
    checked_at: Some(audit.checked_at.to_rfc3339()),
  };
  (result, health)
}

pub struct NewestBackup {
  pub key: String,
  pub created_at: DateTime<Utc>,
}

/// The newest backup archive on disk, by modification time, as the Backups
/// page lists them.
pub fn newest_backup(state: &AppState) -> Option<NewestBackup> {
  std::fs::read_dir(state.config.data_dir.join("backups"))
    .ok()?
    .flatten()
    .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "dop"))
    .filter_map(|entry| {
      let meta = entry.metadata().ok()?;
      let modified = meta.modified().or_else(|_| meta.created()).ok()?;
      Some(NewestBackup {
        key: entry.file_name().to_string_lossy().into_owned(),
        created_at: modified.into(),
      })
    })
    .max_by_key(|backup| backup.created_at)
}

async fn backup_age(
  pool: &SqlitePool,
  newest: Option<&NewestBackup>,
  now: DateTime<Utc>,
) -> Result<Check> {
  let Some(newest) = newest else {
    let secrets: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM secrets")
      .fetch_one(pool)
      .await?;
    return Ok(if secrets > 0 {
      check(
        "backup.age",
        CheckStatus::Crit,
        "No backups yet",
        format!(
          "Nothing protects the {} {} on this server if its disk fails.",
          format_count(secrets as u64),
          plural(secrets, "secret", "secrets")
        ),
        now,
      )
      .fix("/backups")
    } else {
      check(
        "backup.age",
        CheckStatus::Tip,
        "No backups yet",
        "Create one once you've added secrets.",
        now,
      )
      .fix("/backups")
    });
  };
  let changes: i64 = sqlx::query_scalar(&format!(
    "SELECT COUNT(*) FROM audit_events WHERE action IN ({SECRET_CHANGES}) AND created_at > ?"
  ))
  .bind(newest.created_at.to_rfc3339())
  .fetch_one(pool)
  .await?;
  let days = (now - newest.created_at).num_days();
  Ok(if days >= BACKUP_STALE_DAYS && changes > 0 {
    check(
      "backup.age",
      CheckStatus::Crit,
      format!("No backup in {days} days"),
      format!(
        "The newest backup is from {}. Secrets changed {} {} since then.",
        newest.created_at.format("%-d %b"),
        format_count(changes as u64),
        plural(changes, "time", "times")
      ),
      now,
    )
    .fix("/backups")
  } else if changes > 0 {
    check(
      "backup.age",
      CheckStatus::Ok,
      "Latest backup is recent",
      format!(
        "Secrets changed {} {} since it was made.",
        format_count(changes as u64),
        plural(changes, "time", "times")
      ),
      now,
    )
    .value(backup_age_value(days))
  } else {
    check(
      "backup.age",
      CheckStatus::Ok,
      "Latest backup has every change",
      "No secret changed since it was made.",
      now,
    )
    .value(backup_age_value(days))
  })
}

/// Backups live in the data directory, next to the database, so one disk
/// failure takes both. A copy is off the server once the latest archive was
/// downloaded, or when it was uploaded from elsewhere in the first place.
async fn backup_offsite(
  pool: &SqlitePool,
  newest: &NewestBackup,
  now: DateTime<Utc>,
) -> Result<Check> {
  let copied: Option<String> = sqlx::query_scalar(
    "SELECT MAX(created_at) FROM audit_events WHERE action IN ('backup.downloaded','backup.uploaded') AND resource_id = ?",
  )
  .bind(&newest.key)
  .fetch_one(pool)
  .await?;
  Ok(
    match copied.and_then(|at| DateTime::parse_from_rfc3339(&at).ok()) {
      Some(at) => {
        let days = (now - at.with_timezone(&Utc)).num_days();
        check(
          "backup.offsite",
          CheckStatus::Ok,
          "A copy of the latest backup is off this server",
          "It was downloaded, so a disk failure here won't take it.",
          now,
        )
        .value(if days <= 0 {
          "today".to_owned()
        } else {
          format!("{} ago", age(days))
        })
      }
      None => check(
        "backup.offsite",
        CheckStatus::Warn,
        "The latest backup was never downloaded",
        "It only exists on this server, so a disk failure would take it with the database.",
        now,
      )
      .fix("/backups"),
    },
  )
}

async fn tokens_expiring(
  pool: &SqlitePool,
  viewer: &Viewer,
  now: DateTime<Utc>,
) -> Result<Check> {
  let horizon = now + Duration::days(TOKEN_WARNING_DAYS);
  // Runner tokens name their environment; personal tokens their account.
  let expiring: Vec<(String, String, String, Option<String>)> = sqlx::query_as(
    "SELECT t.name || ' · ' || e.name, t.expires_at, 'runner', '/projects/p/' || e.project_id || '/e/' || e.id || '/tokens'
       FROM runner_tokens t JOIN environments e ON e.id = t.environment_id
      WHERE t.revoked_at IS NULL AND t.expires_at > ? AND t.expires_at <= ?
     UNION ALL
     SELECT t.name || ' · ' || a.email, t.expires_at, 'personal', NULL
       FROM personal_tokens t JOIN admins a ON a.id = t.user_id
      WHERE t.revoked_at IS NULL AND t.expires_at > ? AND t.expires_at <= ?
     ORDER BY 2",
  )
  .bind(now.to_rfc3339())
  .bind(horizon.to_rfc3339())
  .bind(now.to_rfc3339())
  .bind(horizon.to_rfc3339())
  .fetch_all(pool)
  .await?;
  if let Some((_, first_expiry, _, _)) = expiring.first() {
    let count = expiring.len() as i64;
    let first = DateTime::parse_from_rfc3339(first_expiry)?;
    let detail = if viewer.token_names {
      let mut names: Vec<&str> = expiring
        .iter()
        .take(3)
        .map(|(name, ..)| name.as_str())
        .collect();
      let more = expiring.len().saturating_sub(3);
      let more_text = format!("{more} more");
      if more > 0 {
        names.push(&more_text);
      }
      format!(
        "{} {} working on {}.",
        join_list(&names),
        if count == 1 { "stops" } else { "stop" },
        first.format("%-d %b")
      )
    } else {
      format!("The first one stops working on {}.", first.format("%-d %b"))
    };
    let fix = expiring
      .iter()
      .find_map(|(_, _, _, path)| path.clone())
      .unwrap_or_else(|| "/users".to_owned());
    return Ok(
      check(
        "tokens.expiring",
        CheckStatus::Warn,
        format!(
          "{count} {} expire{} this week",
          plural(count, "token", "tokens"),
          if count == 1 { "s" } else { "" }
        ),
        detail,
        now,
      )
      .fix(fix),
    );
  }
  let next: Option<String> = sqlx::query_scalar(
    "SELECT MIN(expires_at) FROM (
       SELECT expires_at FROM runner_tokens WHERE revoked_at IS NULL AND expires_at > ?
       UNION ALL
       SELECT expires_at FROM personal_tokens WHERE revoked_at IS NULL AND expires_at > ?)",
  )
  .bind(now.to_rfc3339())
  .bind(now.to_rfc3339())
  .fetch_one(pool)
  .await?;
  let value = match next.and_then(|at| DateTime::parse_from_rfc3339(&at).ok()) {
    Some(at) => {
      let days = (at.with_timezone(&Utc) - now).num_days();
      format!("next one in {days} {}", plural(days, "day", "days"))
    }
    None => "none set to expire".to_owned(),
  };
  Ok(
    check(
      "tokens.expiring",
      CheckStatus::Ok,
      "No tokens expire in the next 7 days",
      "Runner and personal tokens keep working this week.",
      now,
    )
    .value(value),
  )
}

async fn audit_retention(
  state: &AppState,
  now: DateTime<Utc>,
) -> Result<Check> {
  Ok(match state.config.audit_retention_days {
    Some(days) => check(
      "audit.retention",
      CheckStatus::Ok,
      "Audit log has a size limit",
      format!("Records older than {days} days are archived and removed."),
      now,
    )
    .value(format!("{days} {}", plural(days as i64, "day", "days"))),
    None => {
      let events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_events")
        .fetch_one(state.db.pool())
        .await?;
      check(
        "audit.retention",
        CheckStatus::Tip,
        "The audit log has no size limit",
        format!(
          "It holds {} {} and keeps growing until you set audit_retention_days.",
          format_count(events as u64),
          plural(events, "event", "events")
        ),
        now,
      )
      .fix(CONFIG_DOCS)
    }
  })
}

/// Whether the public URL only reaches this machine, such as
/// `http://localhost:8840`.
fn local_only(url: &str) -> bool {
  url::Url::parse(url).is_ok_and(|url| match url.host() {
    Some(url::Host::Domain(domain)) => domain == "localhost",
    Some(url::Host::Ipv4(address)) => address.is_loopback(),
    Some(url::Host::Ipv6(address)) => address.is_loopback(),
    None => false,
  })
}

/// A local-only server needs no public URL; one on the network does.
fn public_url(
  state: &AppState,
  now: DateTime<Utc>,
) -> Check {
  let url = &state.config.public_url;
  match state.config.public_url_source {
    PublicUrlSource::Explicit => check(
      "config.public_url",
      CheckStatus::Ok,
      "Public URL is set in config",
      "Sign-in links and the CLI use this address.",
      now,
    )
    .value(url.clone()),
    PublicUrlSource::LoopbackDefault => check(
      "config.public_url",
      CheckStatus::Ok,
      "Running on this machine only",
      "Links point to this machine, which is all that connects. Set public_url before anyone else does.",
      now,
    )
    .value(url.clone()),
    PublicUrlSource::NetworkPlaceholder => check(
      "config.public_url",
      CheckStatus::Warn,
      "Public URL isn't set",
      "The server listens on the network but doesn't know its own address, so sign-in links and the CLI can point to the wrong place.",
      now,
    )
    .fix(CONFIG_DOCS),
  }
}

/// HTTP is fine when only this machine connects. On the network it sends
/// passwords and secret values in the clear, so it's critical.
fn https(
  state: &AppState,
  now: DateTime<Utc>,
) -> Check {
  let url = &state.config.public_url;
  let secure = url::Url::parse(url).is_ok_and(|url| url.scheme() == "https");
  let proxies = state.config.trusted_proxies.len();
  if secure {
    let result = check(
      "config.https",
      CheckStatus::Ok,
      "Served over HTTPS",
      "Sessions and secret values are encrypted in transit.",
      now,
    );
    return if proxies > 0 {
      result.value(format!(
        "through {proxies} trusted {}",
        plural(proxies as i64, "proxy", "proxies")
      ))
    } else {
      result
    };
  }
  if local_only(url) {
    return check(
      "config.https",
      CheckStatus::Ok,
      "HTTPS isn't needed on this machine",
      "Only this machine connects, so nothing crosses the network. Put the server behind HTTPS before anyone else connects.",
      now,
    )
    .value("localhost only");
  }
  check(
    "config.https",
    CheckStatus::Crit,
    "Not served over HTTPS",
    "This server is reachable over the network without encryption, so passwords, sessions, and secret values can be read in transit.",
    now,
  )
  .fix(CONFIG_DOCS)
}

fn docs(
  state: &AppState,
  now: DateTime<Utc>,
) -> Check {
  if state.config.docs_enabled {
    check(
      "config.docs",
      CheckStatus::Tip,
      "API docs are on",
      "Anyone who can reach the server can read the API reference. Turn it off if nobody needs it.",
      now,
    )
    .value("docs = true")
    .fix(CONFIG_DOCS)
  } else {
    check(
      "config.docs",
      CheckStatus::Ok,
      "API docs are off",
      "The API reference isn't served.",
      now,
    )
    .value("docs = false")
  }
}

async fn managers(
  pool: &SqlitePool,
  now: DateTime<Utc>,
) -> Result<Check> {
  // Root, plus active people whose role can manage accounts.
  let admins: i64 = sqlx::query_scalar(
    "SELECT COUNT(*) FROM admins a JOIN roles r ON r.id = a.role_id
      WHERE a.is_root = 0 AND a.status = 'active' AND r.sign_in = 'interactive'
        AND EXISTS (SELECT 1 FROM role_capabilities c WHERE c.role_id = r.id AND c.capability = 'users.manage')",
  )
  .fetch_one(pool)
  .await?;
  Ok(if admins > 0 {
    check(
      "people.managers",
      CheckStatus::Ok,
      "More than one person can manage the instance",
      "Someone else can recover access if root is locked out.",
      now,
    )
    .value(format!(
      "root + {admins} {}",
      plural(admins, "admin", "admins")
    ))
  } else {
    check(
      "people.managers",
      CheckStatus::Tip,
      "Only root can manage this instance",
      "If root loses access, nobody else can recover it. Give a second person the Admin role.",
      now,
    )
    .fix("/users")
  })
}

/// Only present when `update_check` is on. Nothing is reported until the
/// first lookup finishes.
fn version(
  release: &ReleaseState,
  now: DateTime<Utc>,
) -> Option<Check> {
  let current = env!("CARGO_PKG_VERSION");
  Some(match &release.last {
    Some(last) if last.update_available => check(
      "version.latest",
      CheckStatus::Tip,
      format!("Dopbase {} is out", last.latest.trim_start_matches('v')),
      format!("This server runs {current}. Read the release notes before you upgrade."),
      last.checked_at,
    )
    .fix(last.release_url.clone()),
    Some(last) => check(
      "version.latest",
      CheckStatus::Ok,
      "Running the latest version",
      format!("No release newer than {current} is published."),
      last.checked_at,
    )
    .value(current),
    None if release.failed => check(
      "version.latest",
      CheckStatus::Tip,
      "Couldn't check for a newer version",
      "GitHub didn't answer. The server tries again in an hour.",
      now,
    ),
    None => return None,
  })
}

/// Activity over the last 24 hours in hourly buckets. Fails as a whole if
/// the audit log can't be read.
pub async fn activity(
  state: &AppState,
  viewer: &Viewer,
) -> Result<Activity> {
  let pool = state.db.pool();
  let now = Utc::now();
  let current_hour = now.duration_trunc(Duration::hours(1))?;
  let start = current_hour - Duration::hours(23);
  let hours: Vec<DateTime<Utc>> = (0..24).map(|hour| start + Duration::hours(hour)).collect();
  // created_at is RFC 3339 in UTC, so its first 13 characters name the hour.
  let bucket = |rows: Vec<(String, i64)>| {
    hours
      .iter()
      .map(|hour| {
        let key = hour.format("%Y-%m-%dT%H").to_string();
        rows
          .iter()
          .find(|(at, _)| *at == key)
          .map_or(0, |(_, count)| *count)
      })
      .collect::<Vec<i64>>()
  };
  let hourly = |actions: &str| {
    format!(
      "SELECT substr(created_at, 1, 13), COUNT(*) FROM audit_events
        WHERE action IN ({actions}) AND created_at >= ? GROUP BY 1"
    )
  };
  let reads = bucket(
    sqlx::query_as(&hourly(SECRET_READS))
      .bind(start.to_rfc3339())
      .fetch_all(pool)
      .await?,
  );
  // AI agents read through personal tokens like people can, so their role
  // tells them apart.
  let readers: Vec<(String, i64)> = sqlx::query_as(&format!(
    "SELECT CASE WHEN e.actor_type = 'runner' THEN 'runner'
                 WHEN r.sign_in = 'token_only' THEN 'agent'
                 ELSE 'person' END,
            COUNT(DISTINCT e.actor_id) FROM audit_events e
       LEFT JOIN admins a ON a.id = e.actor_id
       LEFT JOIN roles r ON r.id = a.role_id
      WHERE e.action IN ({SECRET_READS}) AND e.created_at >= ? GROUP BY 1"
  ))
  .bind(start.to_rfc3339())
  .fetch_all(pool)
  .await?;
  let readers_of = |kind: &str| {
    readers
      .iter()
      .find(|(actor, _)| actor == kind)
      .map_or(0, |(_, count)| *count)
  };
  let failures = bucket(
    sqlx::query_as(&hourly("'login.failed'"))
      .bind(start.to_rfc3339())
      .fetch_all(pool)
      .await?,
  );
  let (accounts, addresses) = if viewer.sign_in_details {
    let accounts: Vec<String> = sqlx::query_scalar(
      "SELECT DISTINCT actor_label FROM audit_events
        WHERE action = 'login.failed' AND created_at >= ? AND actor_label IS NOT NULL
        ORDER BY 1 LIMIT 10",
    )
    .bind(start.to_rfc3339())
    .fetch_all(pool)
    .await?;
    let addresses: Vec<String> = sqlx::query_scalar(
      "SELECT DISTINCT ip_address FROM audit_events
        WHERE action = 'login.failed' AND created_at >= ? AND ip_address IS NOT NULL
        ORDER BY 1 LIMIT 10",
    )
    .bind(start.to_rfc3339())
    .fetch_all(pool)
    .await?;
    (Some(accounts), Some(addresses))
  } else {
    (None, None)
  };
  let sessions: Vec<(String, i64)> = sqlx::query_as(
    "SELECT kind, COUNT(*) FROM sessions
      WHERE revoked_at IS NULL AND idle_expires_at > ? AND absolute_expires_at > ?
      GROUP BY kind",
  )
  .bind(now.to_rfc3339())
  .bind(now.to_rfc3339())
  .fetch_all(pool)
  .await?;
  let sessions_of = |kind: &str| {
    sessions
      .iter()
      .find(|(k, _)| k == kind)
      .map_or(0, |(_, count)| *count)
  };
  let refused = state.rate_limiter.refusals_by_hour();
  Ok(Activity {
    hours: hours.iter().map(DateTime::to_rfc3339).collect(),
    secret_reads: SecretReads {
      total: reads.iter().sum(),
      hourly: reads,
      runner_tokens: readers_of("runner"),
      people: readers_of("person"),
      ai_agents: readers_of("agent"),
    },
    failed_sign_ins: FailedSignIns {
      total: failures.iter().sum(),
      hourly: failures,
      accounts,
      addresses,
    },
    rate_limited: RateLimited {
      total: refused.iter().sum(),
      hourly: refused,
    },
    active_sessions: ActiveSessions {
      browser: sessions_of("browser"),
      cli: sessions_of("cli"),
    },
  })
}

fn plural<'a>(
  count: i64,
  one: &'a str,
  many: &'a str,
) -> &'a str {
  if count == 1 { one } else { many }
}

fn age(days: i64) -> String {
  match days {
    ..=0 => "today".to_owned(),
    1 => "1 day".to_owned(),
    days => format!("{days} days"),
  }
}

fn backup_age_value(days: i64) -> String {
  if days <= 0 {
    "made today".to_owned()
  } else {
    format!("{} old", age(days))
  }
}

fn join_list(items: &[&str]) -> String {
  match items {
    [] => String::new(),
    [one] => (*one).to_owned(),
    [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
  }
}

fn format_ms(ms: f64) -> String {
  if ms < 1.0 {
    "under 1 ms".to_owned()
  } else {
    format!("{ms:.0} ms")
  }
}

/// `1284` as `1,284`.
pub fn format_count(count: u64) -> String {
  let digits = count.to_string();
  let mut out = String::with_capacity(digits.len() + digits.len() / 3);
  for (index, digit) in digits.chars().enumerate() {
    if index > 0 && (digits.len() - index).is_multiple_of(3) {
      out.push(',');
    }
    out.push(digit);
  }
  out
}

/// Binary units, rounded the way the Admin UI's `formatBytes` rounds them,
/// so a check's value and its card agree.
pub fn format_bytes(bytes: u64) -> String {
  const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
  if bytes < 1024 {
    return format!("{bytes} B");
  }
  let mut value = bytes as f64;
  let mut unit = 0;
  value /= 1024.0;
  while value >= 1024.0 && unit < UNITS.len() - 1 {
    value /= 1024.0;
    unit += 1;
  }
  if value >= 10.0 {
    format!("{} {}", value.round(), UNITS[unit])
  } else {
    format!("{value:.1} {}", UNITS[unit])
  }
}
