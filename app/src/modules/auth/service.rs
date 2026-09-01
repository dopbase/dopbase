use super::{model::*, repository};
use crate::modules::common;
use crate::{
  constants::{
    errors::{AUTHENTICATION_INVALID, AUTHORIZATION_DENIED, RATE_LIMITED},
    limits::{
      BROWSER_SESSION_ABSOLUTE_HOURS, BROWSER_SESSION_IDLE_HOURS, CLI_SESSION_ABSOLUTE_DAYS,
      CLI_SESSION_IDLE_DAYS, RECENT_AUTHENTICATION_MINUTES,
    },
    tokens::{ADMIN_SESSION_PREFIX, CSRF_TOKEN_PREFIX, SESSION_ID_PREFIX},
  },
  extractors::require_admin,
  http::HttpError,
  models::{AuthIdentity, SessionKind},
  services::token,
  state::AppState,
};
use chrono::{Duration, Utc};
pub struct LoginResult {
  pub response: LoginResponse,
  pub cookie_token: Option<String>,
}
pub async fn login(
  state: &AppState,
  request: LoginRequest,
) -> Result<LoginResult, HttpError> {
  let email = common::validate_email(&request.email)?;
  let limit_key = format!("login:{email}");
  if !state.rate_limiter.check(&limit_key).await {
    return Err(HttpError::new(
      axum::http::StatusCode::TOO_MANY_REQUESTS,
      RATE_LIMITED,
      "Too many login attempts. Please try again later.",
    ));
  }
  let admin = repository::admin_by_email(state.db.pool(), &email).await?;
  let valid = if let Some((_, _, hash)) = &admin {
    common::verify_password_async(request.password.clone(), hash.clone()).await?
  } else {
    false
  };
  if !valid {
    state.rate_limiter.failure(&limit_key).await;
    let _ = common::audit(
      state.db.pool(),
      "anonymous",
      None,
      Some(&email),
      "login.failed",
      None,
      None,
      None,
      None,
      serde_json::json!({}),
    )
    .await;
    return Err(HttpError::unauthorized(
      AUTHENTICATION_INVALID,
      "The email or password is incorrect.",
    ));
  }
  state.rate_limiter.clear(&limit_key).await;
  let (admin_id, email, _) = admin.expect("validated");
  let raw = token::generate(ADMIN_SESSION_PREFIX).map_err(|_| HttpError::internal())?;
  let csrf = if request.session_kind == SessionKind::Browser {
    Some(token::generate(CSRF_TOKEN_PREFIX).map_err(|_| HttpError::internal())?)
  } else {
    None
  };
  let now = Utc::now();
  let (idle, absolute) = if request.session_kind == SessionKind::Browser {
    (
      now + Duration::hours(BROWSER_SESSION_IDLE_HOURS),
      now + Duration::hours(BROWSER_SESSION_ABSOLUTE_HOURS),
    )
  } else {
    (
      now + Duration::days(CLI_SESSION_IDLE_DAYS),
      now + Duration::days(CLI_SESSION_ABSOLUTE_DAYS),
    )
  };
  let session_id = token::public_id(SESSION_ID_PREFIX);
  let mut tx = state.db.pool().begin().await?;
  // Keep revoked/expired session history for 30 days for operational
  // diagnosis, then prune it during the naturally recurring login path.
  let retention_cutoff = (now - Duration::days(30)).to_rfc3339();
  sqlx::query("DELETE FROM sessions WHERE (revoked_at IS NOT NULL AND revoked_at < ?) OR (absolute_expires_at < ?) OR (idle_expires_at < ?)")
    .bind(&retention_cutoff)
    .bind(&retention_cutoff)
    .bind(&retention_cutoff)
    .execute(&mut *tx)
    .await?;
  sqlx::query("INSERT INTO sessions(id,admin_id,kind,token_hash,csrf_hash,created_at,last_used_at,recent_auth_at,idle_expires_at,absolute_expires_at) VALUES(?,?,?,?,?,?,?,?,?,?)").bind(&session_id).bind(&admin_id).bind(request.session_kind.as_str()).bind(token::hash(&raw)).bind(csrf.as_ref().map(|v|token::hash(v))).bind(now.to_rfc3339()).bind(now.to_rfc3339()).bind(now.to_rfc3339()).bind(idle.to_rfc3339()).bind(absolute.to_rfc3339()).execute(&mut *tx).await?;
  common::audit(
    &mut *tx,
    "admin",
    Some(&admin_id),
    Some(&email),
    "login.succeeded",
    None,
    None,
    Some("session"),
    Some(&session_id),
    serde_json::json!({"kind":request.session_kind.as_str()}),
  )
  .await?;
  tx.commit().await?;
  let response = LoginResponse {
    admin_id,
    email,
    session_kind: request.session_kind,
    token: (request.session_kind == SessionKind::Cli).then(|| raw.clone()),
    csrf_token: csrf,
  };
  Ok(LoginResult {
    response,
    cookie_token: (request.session_kind == SessionKind::Browser).then_some(raw),
  })
}
pub fn session(identity: &AuthIdentity) -> Result<SessionResponse, HttpError> {
  match identity {
    AuthIdentity::Admin {
      admin_id,
      email,
      kind,
      recent_auth_at,
      ..
    } => Ok(SessionResponse {
      admin_id: admin_id.clone(),
      email: email.clone(),
      session_kind: *kind,
      recent_authentication: Utc::now() - *recent_auth_at
        <= Duration::minutes(RECENT_AUTHENTICATION_MINUTES),
    }),
    AuthIdentity::Runner { .. } => Err(HttpError::forbidden(
      AUTHORIZATION_DENIED,
      "A runner token has no human session.",
    )),
  }
}
pub async fn logout(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<(), HttpError> {
  let (admin_id, email) = require_admin(identity)?;
  let session_id = match identity {
    AuthIdentity::Admin { session_id, .. } => session_id,
    _ => unreachable!(),
  };
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  sqlx::query("UPDATE sessions SET revoked_at=? WHERE id=?")
    .bind(&now)
    .bind(session_id)
    .execute(&mut *tx)
    .await?;
  common::audit(
    &mut *tx,
    "admin",
    Some(admin_id),
    Some(email),
    "logout.succeeded",
    None,
    None,
    Some("session"),
    Some(session_id),
    serde_json::json!({}),
  )
  .await?;
  tx.commit().await?;
  Ok(())
}
pub async fn reauthenticate(
  state: &AppState,
  identity: &AuthIdentity,
  password: &str,
) -> Result<(), HttpError> {
  let (admin_id, email) = require_admin(identity)?;
  let hash = repository::password_hash(state.db.pool(), admin_id)
    .await?
    .ok_or_else(|| HttpError::unauthorized(AUTHENTICATION_INVALID, "The session is invalid."))?;
  if !common::verify_password_async(password.to_owned(), hash).await? {
    return Err(HttpError::unauthorized(
      AUTHENTICATION_INVALID,
      "The password is incorrect.",
    ));
  }
  let session_id = match identity {
    AuthIdentity::Admin { session_id, .. } => session_id,
    _ => unreachable!(),
  };
  let mut tx = state.db.pool().begin().await?;
  sqlx::query("UPDATE sessions SET recent_auth_at=? WHERE id=?")
    .bind(Utc::now().to_rfc3339())
    .bind(session_id)
    .execute(&mut *tx)
    .await?;
  common::audit(
    &mut *tx,
    "admin",
    Some(admin_id),
    Some(email),
    "admin.reauthenticated",
    None,
    None,
    None,
    None,
    serde_json::json!({}),
  )
  .await?;
  tx.commit().await?;
  Ok(())
}
pub async fn change_password(
  state: &AppState,
  identity: &AuthIdentity,
  request: ChangePasswordRequest,
) -> Result<(), HttpError> {
  let (admin_id, email) = require_admin(identity)?;
  common::validate_password(&request.new_password)?;
  let old = repository::password_hash(state.db.pool(), admin_id)
    .await?
    .ok_or_else(|| HttpError::unauthorized(AUTHENTICATION_INVALID, "The session is invalid."))?;
  if !common::verify_password_async(request.current_password.clone(), old).await? {
    return Err(HttpError::unauthorized(
      AUTHENTICATION_INVALID,
      "The password is incorrect.",
    ));
  }
  let new_hash = common::hash_password_async(request.new_password.clone()).await?;
  let mut tx = state.db.pool().begin().await?;
  sqlx::query("UPDATE admins SET password_hash=?,updated_at=? WHERE id=?")
    .bind(new_hash)
    .bind(Utc::now().to_rfc3339())
    .bind(admin_id)
    .execute(&mut *tx)
    .await?;
  sqlx::query("UPDATE sessions SET revoked_at=? WHERE admin_id=? AND revoked_at IS NULL")
    .bind(Utc::now().to_rfc3339())
    .bind(admin_id)
    .execute(&mut *tx)
    .await?;
  common::audit(
    &mut *tx,
    "admin",
    Some(admin_id),
    Some(email),
    "admin.password_changed",
    None,
    None,
    Some("admin"),
    Some(admin_id),
    serde_json::json!({}),
  )
  .await?;
  tx.commit().await?;
  Ok(())
}
