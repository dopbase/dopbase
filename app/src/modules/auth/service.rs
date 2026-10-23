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
  extractors::require_project_manager,
  http::HttpError,
  models::{AdminRole, AuthIdentity, SessionKind},
  services::token,
  state::AppState,
};
use chrono::{DateTime, Duration, Utc};

pub struct LoginResult {
  pub response: LoginResponse,
  pub cookie_token: Option<String>,
}

/// A freshly generated session that has not been stored yet. Shared by password
/// login and OIDC sign-in so both constrain browser and CLI lifetimes alike.
pub(crate) struct NewSession {
  pub id: String,
  pub raw_token: String,
  pub csrf_token: Option<String>,
  pub created_at: DateTime<Utc>,
  pub idle_expires_at: DateTime<Utc>,
  pub absolute_expires_at: DateTime<Utc>,
}

pub(crate) fn prepare_session(kind: SessionKind) -> Result<NewSession, HttpError> {
  let raw_token = token::generate(ADMIN_SESSION_PREFIX).map_err(|_| HttpError::internal())?;
  let csrf_token = if kind == SessionKind::Browser {
    Some(token::generate(CSRF_TOKEN_PREFIX).map_err(|_| HttpError::internal())?)
  } else {
    None
  };
  let now = Utc::now();
  let (idle, absolute) = if kind == SessionKind::Browser {
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
  Ok(NewSession {
    id: token::public_id(SESSION_ID_PREFIX),
    raw_token,
    csrf_token,
    created_at: now,
    idle_expires_at: idle,
    absolute_expires_at: absolute,
  })
}

/// Prune expired session history and insert a session row inside the caller's
/// transaction, so the session and its audit event commit together.
pub(crate) async fn insert_session(
  tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
  session: &NewSession,
  admin_id: &str,
  kind: SessionKind,
) -> Result<(), HttpError> {
  // Keep revoked/expired session history for 30 days for operational
  // diagnosis, then prune it during the naturally recurring login path.
  let retention_cutoff = (session.created_at - Duration::days(30)).to_rfc3339();
  sqlx::query("DELETE FROM sessions WHERE (revoked_at IS NOT NULL AND revoked_at < ?) OR (absolute_expires_at < ?) OR (idle_expires_at < ?)")
    .bind(&retention_cutoff)
    .bind(&retention_cutoff)
    .bind(&retention_cutoff)
    .execute(&mut **tx)
    .await?;
  sqlx::query("INSERT INTO sessions(id,admin_id,kind,token_hash,csrf_hash,created_at,last_used_at,recent_auth_at,idle_expires_at,absolute_expires_at) VALUES(?,?,?,?,?,?,?,?,?,?)")
    .bind(&session.id)
    .bind(admin_id)
    .bind(kind.as_str())
    .bind(token::hash(&session.raw_token))
    .bind(session.csrf_token.as_ref().map(|value| token::hash(value)))
    .bind(session.created_at.to_rfc3339())
    .bind(session.created_at.to_rfc3339())
    .bind(session.created_at.to_rfc3339())
    .bind(session.idle_expires_at.to_rfc3339())
    .bind(session.absolute_expires_at.to_rfc3339())
    .execute(&mut **tx)
    .await?;
  Ok(())
}

/// Build the login response and the cookie value for a stored session.
pub(crate) fn session_grant(
  admin_id: &str,
  email: &str,
  role: AdminRole,
  kind: SessionKind,
  session: &NewSession,
) -> LoginResult {
  LoginResult {
    response: LoginResponse {
      admin_id: admin_id.to_owned(),
      email: email.to_owned(),
      role,
      session_kind: kind,
      token: (kind == SessionKind::Cli).then(|| session.raw_token.clone()),
      csrf_token: session.csrf_token.clone(),
      last_login_at: Some(session.created_at.to_rfc3339()),
    },
    cookie_token: (kind == SessionKind::Browser).then(|| session.raw_token.clone()),
  }
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
  let valid = if let Some((_, _, hash, _)) = &admin {
    common::verify_password_async(request.password.clone(), hash.clone()).await?
  } else {
    false
  };
  if !valid {
    state.rate_limiter.failure(&limit_key).await;
    let _ = common::audit_standalone(
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
  let (admin_id, email, _, role_value) = admin.expect("validated");
  let role = match role_value.as_str() {
    "root" => AdminRole::Root,
    "admin" => AdminRole::Admin,
    "member" => AdminRole::Member,
    _ => return Err(HttpError::internal()),
  };
  let session = prepare_session(request.session_kind)?;
  let mut tx = state.db.pool().begin().await?;
  insert_session(&mut tx, &session, &admin_id, request.session_kind).await?;
  common::audit(
    &mut tx,
    role.audit_actor_type(),
    Some(&admin_id),
    Some(&email),
    "login.succeeded",
    None,
    None,
    Some("session"),
    Some(&session.id),
    serde_json::json!({"kind":request.session_kind.as_str()}),
  )
  .await?;
  tx.commit().await?;
  Ok(session_grant(
    &admin_id,
    &email,
    role,
    request.session_kind,
    &session,
  ))
}
pub fn session(identity: &AuthIdentity) -> Result<SessionResponse, HttpError> {
  match identity {
    AuthIdentity::Admin {
      admin_id,
      email,
      kind,
      role,
      recent_auth_at,
      created_at,
      ..
    } => Ok(SessionResponse {
      admin_id: admin_id.clone(),
      email: email.clone(),
      role: *role,
      session_kind: *kind,
      recent_authentication: Utc::now() - *recent_auth_at
        <= Duration::minutes(RECENT_AUTHENTICATION_MINUTES),
      last_login_at: Some(created_at.to_rfc3339()),
    }),
    AuthIdentity::Runner { .. } => Err(HttpError::forbidden(
      AUTHORIZATION_DENIED,
      "A runner token has no human session.",
    )),
    AuthIdentity::ServiceAccount { .. } => Err(HttpError::forbidden(
      AUTHORIZATION_DENIED,
      "An AI service account has no human session.",
    )),
  }
}
pub async fn logout(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<(), HttpError> {
  let (admin_id, email) = require_project_manager(identity)?;
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
    &mut tx,
    identity.audit_actor_type(),
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
  let (admin_id, email) = require_project_manager(identity)?;
  // Password re-verification is brute-forceable through a hijacked session,
  // so it shares the login limiter's budget for this account.
  let limit_key = format!("password_verify:{admin_id}");
  if !state.rate_limiter.check(&limit_key).await {
    return Err(HttpError::new(
      axum::http::StatusCode::TOO_MANY_REQUESTS,
      RATE_LIMITED,
      "Too many attempts. Please try again later.",
    ));
  }
  let hash = repository::password_hash(state.db.pool(), admin_id)
    .await?
    .ok_or_else(|| HttpError::unauthorized(AUTHENTICATION_INVALID, "The session is invalid."))?;
  if !common::verify_password_async(password.to_owned(), hash).await? {
    state.rate_limiter.failure(&limit_key).await;
    return Err(HttpError::unauthorized(
      AUTHENTICATION_INVALID,
      "The password is incorrect.",
    ));
  }
  state.rate_limiter.clear(&limit_key).await;
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
    &mut tx,
    identity.audit_actor_type(),
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
  let (admin_id, email) = require_project_manager(identity)?;
  // Verifying the current password is the same brute-force surface as
  // reauthentication, so it shares that limiter budget.
  let limit_key = format!("password_verify:{admin_id}");
  if !state.rate_limiter.check(&limit_key).await {
    return Err(HttpError::new(
      axum::http::StatusCode::TOO_MANY_REQUESTS,
      RATE_LIMITED,
      "Too many attempts. Please try again later.",
    ));
  }
  common::validate_password(&request.new_password)?;
  let old = repository::password_hash(state.db.pool(), admin_id)
    .await?
    .ok_or_else(|| HttpError::unauthorized(AUTHENTICATION_INVALID, "The session is invalid."))?;
  if !common::verify_password_async(request.current_password.clone(), old).await? {
    state.rate_limiter.failure(&limit_key).await;
    return Err(HttpError::unauthorized(
      AUTHENTICATION_INVALID,
      "The password is incorrect.",
    ));
  }
  state.rate_limiter.clear(&limit_key).await;
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
    &mut tx,
    identity.audit_actor_type(),
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
