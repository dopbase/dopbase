use super::{model::*, repository};
use crate::modules::common;
use crate::{
  constants::{
    errors::{
      AUTHENTICATION_INVALID, AUTHORIZATION_DENIED, PASSWORD_LOGIN_DISABLED, PASSWORD_NOT_SET,
      RATE_LIMITED,
    },
    limits::{
      BROWSER_SESSION_ABSOLUTE_HOURS, BROWSER_SESSION_IDLE_HOURS, CLI_SESSION_ABSOLUTE_DAYS,
      CLI_SESSION_IDLE_DAYS, RECENT_AUTHENTICATION_MINUTES,
    },
    tokens::{ADMIN_SESSION_PREFIX, CSRF_TOKEN_PREFIX, SESSION_ID_PREFIX},
  },
  extractors::{require_person, require_recent_browser_auth},
  http::HttpError,
  models::{AdminRole, AuthIdentity, SessionKind},
  services::{
    authx::{self, Capability, SignIn},
    token,
  },
  state::AppState,
};
use chrono::{DateTime, Duration, Utc};
use sqlx::{Sqlite, Transaction};
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
  let valid = common::verify_password_or_dummy(
    request.password.clone(),
    admin.as_ref().map(|(_, _, hash, _)| hash.clone()),
  )
  .await?;
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
  let role = AdminRole::parse(&role_value).ok_or_else(HttpError::internal)?;
  // The password was right, so saying why it is refused leaks nothing.
  let mut conn = state.db.pool().acquire().await?;
  let access = authx::load_access(&mut conn, &admin_id)
    .await?
    .ok_or_else(|| HttpError::unauthorized(AUTHENTICATION_INVALID, "The account is disabled."))?;
  drop(conn);
  if access.sign_in == SignIn::TokenOnly {
    return Err(HttpError::unauthorized(
      AUTHENTICATION_INVALID,
      "This account signs in with tokens only.",
    ));
  }
  let actor_type = if access.is_root { "root" } else { "admin" };
  // Only a break-glass account, root, keeps password sign-in while single
  // sign-on is required, so there is always a way back in.
  if !access.has(Capability::PasswordBreakglass)
    && crate::modules::sso::service::policy(state)
      .await?
      .require_sso
  {
    let _ = common::audit_standalone(
      state.db.pool(),
      actor_type,
      Some(&admin_id),
      Some(&email),
      "login.failed",
      None,
      None,
      None,
      None,
      serde_json::json!({"reason":"password_login_disabled"}),
    )
    .await;
    return Err(HttpError::forbidden(
      PASSWORD_LOGIN_DISABLED,
      "Password sign-in is turned off for this account. Use single sign-on.",
    ));
  }
  let mut tx = state.db.pool().begin().await?;
  let session = issue_session(&mut tx, &admin_id, request.session_kind).await?;
  common::audit(
    &mut tx,
    actor_type,
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
  let response = LoginResponse {
    admin_id,
    email,
    role,
    session_kind: request.session_kind,
    token: (request.session_kind == SessionKind::Cli).then(|| session.token.clone()),
    csrf_token: session.csrf,
    last_login_at: Some(session.created_at.to_rfc3339()),
  };
  Ok(LoginResult {
    response,
    cookie_token: (request.session_kind == SessionKind::Browser).then_some(session.token),
  })
}

pub(crate) struct IssuedSession {
  pub id: String,
  pub token: String,
  pub csrf: Option<String>,
  pub created_at: DateTime<Utc>,
}

/// Insert a new session for an already authenticated account. Every way of
/// signing in ends here, so lifetimes and retention stay identical across
/// them. The caller records the audit event and commits.
pub(crate) async fn issue_session(
  tx: &mut Transaction<'_, Sqlite>,
  admin_id: &str,
  kind: SessionKind,
) -> Result<IssuedSession, HttpError> {
  let raw = token::generate(ADMIN_SESSION_PREFIX).map_err(|_| HttpError::internal())?;
  let csrf = if kind == SessionKind::Browser {
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
  let session_id = token::public_id(SESSION_ID_PREFIX);
  // Keep revoked/expired session history for 30 days for operational
  // diagnosis, then prune it during the naturally recurring login path.
  repository::prune_sessions(tx, &(now - Duration::days(30)).to_rfc3339()).await?;
  repository::insert_session(
    tx,
    repository::NewSession {
      id: &session_id,
      admin_id,
      kind: kind.as_str(),
      token_hash: &token::hash(&raw),
      csrf_hash: csrf.as_deref().map(token::hash).as_deref(),
      now: &now.to_rfc3339(),
      idle_expires_at: &idle.to_rfc3339(),
      absolute_expires_at: &absolute.to_rfc3339(),
    },
  )
  .await?;
  // Only a new session counts as signing in; confirming one does not.
  repository::mark_signed_in(tx, admin_id, &now.to_rfc3339()).await?;
  Ok(IssuedSession {
    id: session_id,
    token: raw,
    csrf,
    created_at: now,
  })
}
/// Whether this account may confirm itself with its password: it has one,
/// and single sign-on does not require otherwise. Break-glass accounts keep
/// their password either way.
async fn password_confirmation(
  state: &AppState,
  admin_id: &str,
  breakglass: bool,
) -> Result<bool, HttpError> {
  let hash = repository::password_hash(state.db.pool(), admin_id)
    .await?
    .unwrap_or_default();
  Ok(
    common::has_password(&hash)
      && (breakglass
        || !crate::modules::sso::service::policy(state)
          .await?
          .require_sso),
  )
}

pub async fn session(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<SessionResponse, HttpError> {
  if let AuthIdentity::Admin {
    admin_id, access, ..
  } = identity
  {
    let mut response = session_summary(identity)?;
    response.has_password = common::has_password(
      &repository::password_hash(state.db.pool(), admin_id)
        .await?
        .unwrap_or_default(),
    );
    response.password_confirmation =
      password_confirmation(state, admin_id, access.has(Capability::PasswordBreakglass)).await?;
    response.sso_confirmation =
      crate::modules::sso::service::confirmation_name(state, admin_id).await?;
    if let AuthIdentity::Admin { access, .. } = identity
      && let Some(role_id) = &access.role_id
    {
      response.token_scope = repository::token_scope(state.db.pool(), role_id)
        .await?
        .unwrap_or_else(|| "none".into());
    }
    return Ok(response);
  }
  session_summary(identity)
}

fn session_summary(identity: &AuthIdentity) -> Result<SessionResponse, HttpError> {
  match identity {
    AuthIdentity::Admin {
      admin_id,
      email,
      kind,
      role,
      recent_auth_at,
      created_at,
      csrf_hash,
      access,
      ..
    } => Ok(SessionResponse {
      admin_id: admin_id.clone(),
      email: email.clone(),
      role: *role,
      role_name: access.role_name.clone(),
      capabilities: access.capabilities().collect(),
      max_level: access.max_level,
      token_scope: "none".into(),
      session_kind: *kind,
      recent_authentication: Utc::now() - *recent_auth_at
        <= Duration::minutes(RECENT_AUTHENTICATION_MINUTES),
      last_login_at: Some(created_at.to_rfc3339()),
      has_password: false,
      password_confirmation: false,
      sso_confirmation: None,
      csrf_token: csrf_hash
        .as_deref()
        .map(|hash| token::tab_csrf(CSRF_TOKEN_PREFIX, hash)),
    }),
    AuthIdentity::Runner { .. } => Err(HttpError::forbidden(
      AUTHORIZATION_DENIED,
      "A runner token has no human session.",
    )),
    AuthIdentity::Token { .. } => Err(HttpError::forbidden(
      AUTHORIZATION_DENIED,
      "A personal token has no session.",
    )),
  }
}
pub async fn logout(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<(), HttpError> {
  let (admin_id, email) = require_person(identity)?;
  let session_id = match identity {
    AuthIdentity::Admin { session_id, .. } => session_id,
    _ => unreachable!(),
  };
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  repository::revoke_session(&mut tx, session_id, &now).await?;
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
  let (admin_id, email) = require_person(identity)?;
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
  let breakglass = match identity {
    AuthIdentity::Admin { access, .. } => access.has(Capability::PasswordBreakglass),
    _ => unreachable!(),
  };
  if !common::has_password(&hash) {
    return Err(HttpError::conflict(
      PASSWORD_NOT_SET,
      "This account has no password. Confirm with single sign-on instead.",
    ));
  }
  if !password_confirmation(state, admin_id, breakglass).await? {
    return Err(HttpError::forbidden(
      PASSWORD_LOGIN_DISABLED,
      "Passwords are turned off for this account. Confirm with single sign-on instead.",
    ));
  }
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
  repository::mark_recent_auth(&mut tx, session_id, &Utc::now().to_rfc3339()).await?;
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
  let (admin_id, email) = require_person(identity)?;
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
  if common::has_password(&old) {
    if !common::verify_password_async(request.current_password.clone(), old).await? {
      state.rate_limiter.failure(&limit_key).await;
      return Err(HttpError::unauthorized(
        AUTHENTICATION_INVALID,
        "The password is incorrect.",
      ));
    }
  } else {
    // An account without a password proves itself with a fresh single
    // sign-on confirmation in the Admin UI instead.
    if !matches!(
      identity,
      AuthIdentity::Admin {
        kind: SessionKind::Browser,
        ..
      }
    ) {
      return Err(HttpError::forbidden(
        AUTHORIZATION_DENIED,
        "Set a password from the Admin UI.",
      ));
    }
    require_recent_browser_auth(identity)?;
  }
  state.rate_limiter.clear(&limit_key).await;
  let new_hash = common::hash_password_async(request.new_password.clone()).await?;
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  repository::set_password_hash(&mut tx, admin_id, &new_hash, &now).await?;
  repository::revoke_sessions_of(&mut tx, admin_id, &now).await?;
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
