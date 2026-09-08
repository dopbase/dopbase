use super::{model::InstanceStatus, repository};
use crate::{
  constants::errors::RATE_LIMITED,
  extractors::{
    require_admin, require_metadata_access, require_mutation, require_recent_browser_auth,
    require_root_browser,
  },
  http::HttpError,
  models::AuthIdentity,
  state::AppState,
};
use axum::http::HeaderMap;
pub async fn status(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<InstanceStatus, HttpError> {
  require_admin(identity)?;
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
    key_availability: "available",
    configuration_reload: "restartRequired",
  })
}

pub async fn public_status(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<super::model::StatusResponse, HttpError> {
  require_metadata_access(identity)?;
  state.db.ping().await?;
  async fn count(
    pool: &sqlx::SqlitePool,
    sql: &str,
  ) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(sql).fetch_one(pool).await
  }
  let projects = count(state.db.pool(), "SELECT COUNT(*) FROM projects").await?;
  let environments = count(state.db.pool(), "SELECT COUNT(*) FROM environments").await?;
  let secrets = count(state.db.pool(), "SELECT COUNT(*) FROM secrets").await?;
  let human_users = count(state.db.pool(), "SELECT COUNT(*) FROM admins").await?;
  let ai_agents = count(state.db.pool(), "SELECT COUNT(*) FROM service_accounts").await?;
  let active_runner_tokens =
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM runner_tokens WHERE revoked_at IS NULL")
      .fetch_one(state.db.pool())
      .await?;
  let active_agent_tokens = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agent_tokens WHERE revoked_at IS NULL AND (expires_at IS NULL OR expires_at>?)").bind(chrono::Utc::now().to_rfc3339()).fetch_one(state.db.pool()).await?;
  let backups = std::fs::read_dir(state.config.data_dir.join("backups"))
    .map(|entries| {
      entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "dop"))
        .count() as i64
    })
    .unwrap_or(0);
  Ok(super::model::StatusResponse {
    version: env!("CARGO_PKG_VERSION"),
    uptime_seconds: state.started_at.elapsed().as_secs(),
    initialization_state: if human_users > 0 {
      "ready"
    } else {
      "setupRequired"
    },
    database_health: "healthy",
    key_availability: "available",
    projects,
    environments,
    secrets,
    human_users,
    ai_agents,
    active_runner_tokens,
    active_agent_tokens,
    backups,
    observed_at: chrono::Utc::now().to_rfc3339(),
  })
}

pub async fn preview(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<super::model::FactoryResetPreview, HttpError> {
  require_root_browser(identity)?;
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
    ai_agents: count(state.db.pool(), "service_accounts").await?,
    agent_tokens: count(state.db.pool(), "agent_tokens").await?,
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
  let (root_id, _) = require_root_browser(identity)?;
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
      "runner_tokens",
      "agent_tokens",
      "service_accounts",
      "secrets",
      "environment_env_layout",
      "environments",
      "environment_id_reservations",
      "projects",
      "sessions",
      "admins",
    ] {
      sqlx::query(&format!("DELETE FROM {table}"))
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query("UPDATE environment_id_sequence SET next_number=1000 WHERE id=1")
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
