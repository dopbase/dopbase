use super::{model::*, repository};
use crate::modules::common;
use crate::{
  constants::{
    errors::RATE_LIMITED,
    limits::{BROWSER_SESSION_ABSOLUTE_HOURS, BROWSER_SESSION_IDLE_HOURS},
    tokens::{ADMIN_ID_PREFIX, ADMIN_SESSION_PREFIX, CSRF_TOKEN_PREFIX, SESSION_ID_PREFIX},
  },
  http::HttpError,
  services::token,
  state::AppState,
};
use chrono::{Duration, Utc};
pub async fn status(state: &AppState) -> Result<BootstrapStatus, HttpError> {
  let count = repository::admin_count(state.db.pool()).await?;
  Ok(BootstrapStatus {
    state: if count == 0 { "setupRequired" } else { "ready" },
  })
}
pub struct CreatedAdmin {
  pub response: BootstrapAdminResponse,
  pub session_token: String,
}
pub async fn create(
  state: &AppState,
  request: BootstrapAdminRequest,
) -> Result<CreatedAdmin, HttpError> {
  if !state.rate_limiter.check("bootstrap").await {
    return Err(HttpError::new(
      axum::http::StatusCode::TOO_MANY_REQUESTS,
      RATE_LIMITED,
      "Too many setup attempts. Please try again later.",
    ));
  }
  let email = common::validate_email(&request.email)?;
  common::validate_password(&request.password)?;
  let expected = state.setup.read().await.token.clone().ok_or_else(|| {
    HttpError::conflict(
      "BOOTSTRAP_CLOSED",
      "This instance has already been initialized.",
    )
  })?;
  if !token::constant_time_eq(&request.setup_token, &expected) {
    state.rate_limiter.failure("bootstrap").await;
    return Err(HttpError::unauthorized(
      "SETUP_TOKEN_INVALID",
      "The setup token is invalid.",
    ));
  }
  let hash = common::hash_password(&request.password)?;
  let admin_id = token::public_id(ADMIN_ID_PREFIX);
  let session_id = token::public_id(SESSION_ID_PREFIX);
  let session_token = token::generate(ADMIN_SESSION_PREFIX).map_err(|_| HttpError::internal())?;
  let csrf = token::generate(CSRF_TOKEN_PREFIX).map_err(|_| HttpError::internal())?;
  let now = Utc::now();
  let mut tx = state.db.pool().begin().await?;
  let admin_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admins")
    .fetch_one(&mut *tx)
    .await?;
  if admin_count > 0 {
    return Err(HttpError::conflict(
      "BOOTSTRAP_CLOSED",
      "This instance has already been initialized.",
    ));
  }
  repository::insert_admin(&mut tx, &admin_id, &email, &hash, &now.to_rfc3339()).await?;
  repository::insert_session(
    &mut tx,
    &session_id,
    &admin_id,
    &token::hash(&session_token),
    &token::hash(&csrf),
    &now.to_rfc3339(),
    &(now + Duration::hours(BROWSER_SESSION_IDLE_HOURS)).to_rfc3339(),
    &(now + Duration::hours(BROWSER_SESSION_ABSOLUTE_HOURS)).to_rfc3339(),
  )
  .await?;
  common::audit(
    &mut *tx,
    "admin",
    Some(&admin_id),
    Some(&email),
    "admin.bootstrapped",
    None,
    None,
    Some("admin"),
    Some(&admin_id),
    serde_json::json!({}),
  )
  .await?;
  tx.commit().await?;
  state.setup.write().await.token = None;
  state.rate_limiter.clear("bootstrap").await;
  Ok(CreatedAdmin {
    response: BootstrapAdminResponse {
      admin_id,
      email,
      csrf_token: csrf,
    },
    session_token,
  })
}
