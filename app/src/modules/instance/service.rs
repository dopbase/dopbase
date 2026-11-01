use super::{
  model::{CheckStatus, InstanceStatus},
  repository,
};
use crate::{
  constants::errors::RATE_LIMITED,
  extractors::{
    require_capability, require_capability_browser, require_metadata_access, require_mutation,
    require_recent_browser_auth,
  },
  http::HttpError,
  models::AuthIdentity,
  services::{authx::Capability, crypto::KeyFileState},
  state::AppState,
};
use axum::http::HeaderMap;
pub async fn status(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<InstanceStatus, HttpError> {
  require_capability(identity, Capability::InstanceView)?;
  state.db.ping().await?;
  Ok(InstanceStatus {
    version: env!("CARGO_PKG_VERSION"),
    public_url: state.config.public_url.clone(),
    initialization_state: if repository::initialized(state.db.pool()).await? {
      "ready"
    } else {
      "setupRequired"
    },
    database_health: "healthy",
    key_availability: match state.crypto.key_file_state(&state.config.master_key.path) {
      KeyFileState::Matches => "available",
      other => other.as_str(),
    },
    configuration_reload: "restartRequired",
  })
}

pub async fn public_status(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<super::model::StatusResponse, HttpError> {
  require_metadata_access(identity)?;
  Ok(build_status(state, identity, AuditRead::Cached).await)
}

/// Run every check now, including the audit chain walk the page otherwise
/// reads from the cache.
pub async fn run_checks(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
) -> Result<super::model::StatusResponse, HttpError> {
  require_mutation(identity, headers)?;
  require_capability(identity, Capability::InstanceView)?;
  if !state.status.claim_forced_run().await {
    return Err(HttpError::new(
      axum::http::StatusCode::TOO_MANY_REQUESTS,
      RATE_LIMITED,
      "Checks just ran. Wait a few seconds and try again.",
    ));
  }
  Ok(build_status(state, identity, AuditRead::Fresh).await)
}

/// Where the audit chain result comes from.
enum AuditRead {
  Cached,
  Fresh,
}

async fn build_status(
  state: &AppState,
  identity: &AuthIdentity,
  audit: AuditRead,
) -> super::model::StatusResponse {
  let pool = state.db.pool();
  // A failed count is reported as null; it never fails the response.
  async fn count(
    pool: &sqlx::SqlitePool,
    sql: &str,
    now: Option<&str>,
  ) -> Option<i64> {
    let mut query = sqlx::query_scalar::<_, i64>(sql);
    if let Some(now) = now {
      query = query.bind(now.to_owned());
    }
    query
      .fetch_one(pool)
      .await
      .inspect_err(|error| tracing::warn!(%error, sql, "status count failed"))
      .ok()
  }
  let now = chrono::Utc::now().to_rfc3339();
  let projects = count(pool, "SELECT COUNT(*) FROM projects", None).await;
  let environments = count(pool, "SELECT COUNT(*) FROM environments", None).await;
  let secrets = count(pool, "SELECT COUNT(*) FROM secrets", None).await;
  let human_users = count(pool, "SELECT COUNT(*) FROM admins a LEFT JOIN roles r ON r.id = a.role_id WHERE COALESCE(r.sign_in, 'interactive') = 'interactive'", None).await;
  let ai_agents = count(
    pool,
    "SELECT COUNT(*) FROM admins a JOIN roles r ON r.id = a.role_id WHERE r.sign_in = 'token_only'",
    None,
  )
  .await;
  let active_runner_tokens = count(
    pool,
    "SELECT COUNT(*) FROM runner_tokens WHERE revoked_at IS NULL AND (expires_at IS NULL OR expires_at > ?)",
    Some(&now),
  )
  .await;
  let active_agent_tokens = count(pool, "SELECT COUNT(*) FROM personal_tokens t JOIN admins a ON a.id = t.user_id JOIN roles r ON r.id = a.role_id WHERE r.sign_in = 'token_only' AND t.revoked_at IS NULL AND (t.expires_at IS NULL OR t.expires_at > ?)", Some(&now)).await;
  let backups = std::fs::read_dir(state.config.data_dir.join("backups"))
    .map(|entries| {
      entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "dop"))
        .count() as i64
    })
    .unwrap_or(0);
  let (summary, checks, health, activity, latest_release) = match identity {
    AuthIdentity::Admin { access, .. } if access.has(Capability::InstanceView) => {
      // Only viewers who get the checks back pay for the chain walk.
      let audit = match audit {
        AuditRead::Cached => state.status.audit(pool).await,
        AuditRead::Fresh => state.status.refresh(pool).await,
      };
      let viewer = super::checks::Viewer {
        token_names: access.has(Capability::TokensRevokeOthers),
        sign_in_details: access.has(Capability::AuditRead),
      };
      let release = if state.config.update_check {
        let release = state.status.release().await;
        // Ask GitHub off the request when a lookup is due; the answer shows
        // up on a later poll.
        let monitor = state.status.clone();
        tokio::spawn(async move { monitor.refresh_release_if_due().await });
        Some(release)
      } else {
        None
      };
      let report = super::checks::run(state, &audit, release.as_ref(), &viewer).await;
      let latest_release =
        release
          .and_then(|release| release.last)
          .map(|last| super::model::LatestRelease {
            version: last.latest,
            update_available: last.update_available,
            url: last.release_url,
            checked_at: last.checked_at.to_rfc3339(),
          });
      let activity = super::checks::activity(state, &viewer)
        .await
        .inspect_err(|error| tracing::warn!(%error, "status activity failed"))
        .ok();
      let tally = |status| {
        report
          .checks
          .iter()
          .filter(|check| check.status == status)
          .count()
      };
      let summary = super::model::StatusSummary {
        critical: tally(CheckStatus::Crit),
        warnings: tally(CheckStatus::Warn),
        tips: tally(CheckStatus::Tip),
        passed: tally(CheckStatus::Ok),
        checked_at: report
          .checks
          .iter()
          .map(|check| check.last_run_at.as_str())
          .max()
          .unwrap_or(&now)
          .to_owned(),
      };
      (
        Some(summary),
        Some(report.checks),
        Some(report.health),
        activity,
        latest_release,
      )
    }
    _ => (None, None, None, None, None),
  };
  super::model::StatusResponse {
    version: env!("CARGO_PKG_VERSION"),
    uptime_seconds: state.started_at.elapsed().as_secs(),
    initialization_state: if human_users.unwrap_or(1) > 0 {
      "ready"
    } else {
      "setupRequired"
    },
    platform: super::model::Platform {
      os: std::env::consts::OS,
      arch: std::env::consts::ARCH,
    },
    projects,
    environments,
    secrets,
    human_users,
    ai_agents,
    active_runner_tokens,
    active_agent_tokens,
    backups,
    summary,
    checks,
    health,
    activity,
    latest_release,
    observed_at: now,
  }
}

pub async fn preview(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<super::model::FactoryResetPreview, HttpError> {
  require_capability_browser(identity, Capability::InstanceReset)?;
  async fn count(
    pool: &sqlx::SqlitePool,
    table: &str,
  ) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
      .fetch_one(pool)
      .await
  }
  let backups = std::fs::read_dir(state.config.data_dir.join("backups"))
    .map(|entries| {
      entries
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == "dop"))
        .count() as i64
    })
    .unwrap_or(0);
  Ok(super::model::FactoryResetPreview {
    users: count(state.db.pool(), "admins").await?,
    projects: count(state.db.pool(), "projects").await?,
    environments: count(state.db.pool(), "environments").await?,
    secrets: count(state.db.pool(), "secrets").await?,
    runner_tokens: count(state.db.pool(), "runner_tokens").await?,
    backups,
    ai_agents: sqlx::query_scalar("SELECT COUNT(*) FROM admins WHERE role_id = 'rol_ai_agent'")
      .fetch_one(state.db.pool())
      .await?,
    agent_tokens: count(state.db.pool(), "personal_tokens").await?,
  })
}

pub async fn factory_reset(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  request: super::model::FactoryResetRequest,
) -> Result<(), HttpError> {
  require_mutation(identity, headers)?;
  require_recent_browser_auth(identity)?;
  let (root_id, _) = require_capability_browser(identity, Capability::InstanceReset)?;
  // The root password verification here shares the reauthentication
  // limiter budget for this account.
  let limit_key = format!("password_verify:{root_id}");
  if !state.rate_limiter.check(&limit_key).await {
    return Err(HttpError::new(
      axum::http::StatusCode::TOO_MANY_REQUESTS,
      RATE_LIMITED,
      "Too many attempts. Please try again later.",
    ));
  }
  if request.confirmation != "FACTORY RESET" || !request.acknowledged {
    return Err(HttpError::bad_request(
      "RESET_CONFIRMATION_REQUIRED",
      "Type FACTORY RESET and acknowledge the danger zone to continue.",
    ));
  }
  let hash: String = sqlx::query_scalar("SELECT password_hash FROM admins WHERE id=?")
    .bind(root_id)
    .fetch_one(state.db.pool())
    .await?;
  if !crate::modules::common::verify_password_async(request.current_password, hash).await? {
    state.rate_limiter.failure(&limit_key).await;
    return Err(HttpError::unauthorized(
      "AUTHENTICATION_INVALID",
      "The root password is incorrect.",
    ));
  }
  state.rate_limiter.clear(&limit_key).await;
  if state
    .maintenance
    .swap(true, std::sync::atomic::Ordering::SeqCst)
  {
    return Err(HttpError::conflict(
      "RESET_IN_PROGRESS",
      "A factory reset is already in progress.",
    ));
  }
  let marker = state.config.data_dir.join(".factory-reset.pending");
  std::fs::write(&marker, b"reset in progress").map_err(|_| HttpError::internal())?;
  let result = async {
    let mut tx = state.db.pool().begin().await?;
    for table in [
      "audit_events",
      "audit_chain_anchor",
      "audit_chain_genesis",
      "runner_tokens",
      "personal_tokens",
      "grants",
      "group_members",
      "groups",
      "secrets",
      "environment_env_layout",
      "environments",
      "projects",
      "cli_device_requests",
      "sso_flows",
      "admin_identities",
      "sso_providers",
      "sessions",
      "admins",
    ] {
      sqlx::query(&format!("DELETE FROM {table}"))
        .execute(&mut *tx)
        .await?;
    }
    // Built-in roles come from migrations and stay; custom roles go.
    sqlx::query("DELETE FROM roles WHERE builtin = 0")
      .execute(&mut *tx)
      .await?;
    tx.commit().await?;
    // Best-effort scrub: deleted secret ciphertext must not linger in the
    // database file or WAL after a reset.
    if let Err(error) = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
      .execute(state.db.pool())
      .await
    {
      tracing::warn!(%error, "failed to checkpoint WAL after factory reset");
    }
    if let Err(error) = sqlx::query("VACUUM").execute(state.db.pool()).await {
      tracing::warn!(%error, "failed to vacuum after factory reset");
    }
    let backup_dir = state.config.data_dir.join("backups");
    if let Ok(entries) = std::fs::read_dir(&backup_dir) {
      for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|x| x == "dop") {
          std::fs::remove_file(path).map_err(|_| HttpError::internal())?;
        }
      }
    }
    state.setup.write().await.token =
      Some(crate::services::token::generate("setup_").map_err(|_| HttpError::internal())?);
    let _ = std::fs::remove_file(&marker);
    Ok::<(), HttpError>(())
  }
  .await;
  if result.is_ok() {
    state
      .maintenance
      .store(false, std::sync::atomic::Ordering::SeqCst);
  }
  result
}
