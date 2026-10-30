use std::convert::Infallible;

use axum::{
  extract::FromRequestParts,
  http::{HeaderMap, header, request::Parts},
};
use chrono::{Duration, Utc};

use crate::{
  constants::{
    errors::{AUTHENTICATION_INVALID, AUTHORIZATION_DENIED, CSRF_REQUIRED_MESSAGE},
    limits::{BROWSER_SESSION_IDLE_HOURS, CLI_SESSION_IDLE_DAYS, RECENT_AUTHENTICATION_MINUTES},
    tokens::CSRF_TOKEN_PREFIX,
  },
  http::HttpError,
  models::{AdminRole, AuthIdentity, SessionKind},
  services::{
    authx::{self, Capability},
    token,
  },
  state::AppState,
};

#[derive(sqlx::FromRow)]
struct SessionRow {
  session_id: String,
  admin_id: String,
  email: String,
  role: String,
  kind: String,
  csrf_hash: Option<Vec<u8>>,
  created_at: String,
  idle_expires_at: String,
  absolute_expires_at: String,
}

fn parse_role(value: &str) -> Result<AdminRole, HttpError> {
  match value {
    "root" => Ok(AdminRole::Root),
    "admin" => Ok(AdminRole::Admin),
    "member" => Ok(AdminRole::Member),
    "ai_agent" => Ok(AdminRole::AiAgent),
    _ => Err(HttpError::unauthorized(
      AUTHENTICATION_INVALID,
      "The account role is invalid.",
    )),
  }
}

impl FromRequestParts<AppState> for AuthIdentity {
  type Rejection = HttpError;

  async fn from_request_parts(
    parts: &mut Parts,
    state: &AppState,
  ) -> Result<Self, Self::Rejection> {
    let (raw_token, cookie_auth) = credential(&parts.headers).ok_or_else(|| {
      HttpError::unauthorized("AUTHENTICATION_REQUIRED", "Please log in to continue.")
    })?;
    let hash = token::hash(&raw_token);
    let now = Utc::now();

    let session: Option<SessionRow> = sqlx::query_as(
            "SELECT s.id AS session_id, a.id AS admin_id, a.email, a.role, s.kind, s.csrf_hash, s.created_at, s.idle_expires_at, s.absolute_expires_at FROM sessions s JOIN admins a ON a.id = s.admin_id WHERE s.token_hash = ? AND s.revoked_at IS NULL",
        ).bind(&hash).fetch_optional(state.db.pool()).await.map_err(HttpError::from)?;
    if let Some(session) = session {
      let created_at = chrono::DateTime::parse_from_rfc3339(&session.created_at)
        .map_err(|_| HttpError::internal())?
        .with_timezone(&Utc);
      let idle = chrono::DateTime::parse_from_rfc3339(&session.idle_expires_at)
        .map_err(|_| HttpError::internal())?
        .with_timezone(&Utc);
      let absolute = chrono::DateTime::parse_from_rfc3339(&session.absolute_expires_at)
        .map_err(|_| HttpError::internal())?
        .with_timezone(&Utc);
      if now >= idle || now >= absolute {
        return Err(HttpError::unauthorized(
          AUTHENTICATION_INVALID,
          "Your session has expired.",
        ));
      }
      let recent: String = sqlx::query_scalar("SELECT recent_auth_at FROM sessions WHERE id = ?")
        .bind(&session.session_id)
        .fetch_one(state.db.pool())
        .await
        .map_err(HttpError::from)?;
      let recent_auth_at = chrono::DateTime::parse_from_rfc3339(&recent)
        .map_err(|_| HttpError::internal())?
        .with_timezone(&Utc);
      let session_kind = if session.kind == "browser" {
        SessionKind::Browser
      } else {
        SessionKind::Cli
      };
      if cookie_auth && session_kind != SessionKind::Browser {
        return Err(HttpError::unauthorized(
          AUTHENTICATION_INVALID,
          "The session type is invalid.",
        ));
      }
      let next_idle = now
        + if session_kind == SessionKind::Browser {
          Duration::hours(BROWSER_SESSION_IDLE_HOURS)
        } else {
          Duration::days(CLI_SESSION_IDLE_DAYS)
        };
      sqlx::query("UPDATE sessions SET last_used_at = ?, idle_expires_at = MIN(?, absolute_expires_at) WHERE id = ?")
                .bind(now.to_rfc3339()).bind(next_idle.to_rfc3339()).bind(&session.session_id)
                .execute(state.db.pool()).await.map_err(HttpError::from)?;
      let mut conn = state.db.pool().acquire().await.map_err(HttpError::from)?;
      let access = authx::load_access(&mut conn, &session.admin_id)
        .await
        .map_err(HttpError::from)?
        .ok_or_else(|| {
          HttpError::unauthorized(AUTHENTICATION_INVALID, "The account is disabled.")
        })?;
      return Ok(AuthIdentity::Admin {
        access,
        admin_id: session.admin_id,
        email: session.email,
        role: parse_role(&session.role)?,
        session_id: session.session_id,
        kind: session_kind,
        recent_auth_at,
        created_at,
        csrf_hash: session.csrf_hash,
      });
    }

    let agent: Option<(String, String, String)> = sqlx::query_as(
      "SELECT t.id, a.id, a.name FROM agent_tokens t JOIN service_accounts a ON a.id=t.service_account_id WHERE t.token_hash=? AND t.revoked_at IS NULL AND (t.expires_at IS NULL OR t.expires_at>?)",
    )
    .bind(&hash)
    .bind(now.to_rfc3339())
    .fetch_optional(state.db.pool())
    .await
    .map_err(HttpError::from)?;
    if let Some((token_id, service_account_id, name)) = agent {
      sqlx::query("UPDATE agent_tokens SET last_used_at=? WHERE id=?")
        .bind(now.to_rfc3339())
        .bind(&token_id)
        .execute(state.db.pool())
        .await
        .map_err(HttpError::from)?;
      return Ok(AuthIdentity::ServiceAccount {
        service_account_id,
        name,
        token_id,
      });
    }

    let runner: Option<(String, String)> = sqlx::query_as(
      "SELECT id, environment_id FROM runner_tokens WHERE token_hash = ? AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at > ?)",
    )
    .bind(&hash)
    .bind(now.to_rfc3339())
    .fetch_optional(state.db.pool())
    .await
    .map_err(HttpError::from)?;
    if let Some((token_id, environment_id)) = runner {
      sqlx::query("UPDATE runner_tokens SET last_used_at = ? WHERE id = ?")
        .bind(now.to_rfc3339())
        .bind(&token_id)
        .execute(state.db.pool())
        .await
        .map_err(HttpError::from)?;
      return Ok(AuthIdentity::Runner {
        token_id,
        environment_id,
      });
    }
    Err(HttpError::unauthorized(
      AUTHENTICATION_INVALID,
      "The provided credential is invalid.",
    ))
  }
}

fn credential(headers: &HeaderMap) -> Option<(String, bool)> {
  if let Some(value) = headers
    .get(header::AUTHORIZATION)
    .and_then(|value| value.to_str().ok())
    && let Some(value) = value.strip_prefix("Bearer ")
  {
    return Some((value.to_owned(), false));
  }
  let cookies = headers.get(header::COOKIE)?.to_str().ok()?;
  cookies.split(';').map(str::trim).find_map(|cookie| {
    cookie
      .strip_prefix("dopbase_session=")
      .map(|value| (value.to_owned(), true))
  })
}

/// Any person signed in with a session, whatever their role. Tokens are
/// not people.
pub fn require_person(identity: &AuthIdentity) -> Result<(&str, &str), HttpError> {
  match identity {
    AuthIdentity::Admin {
      admin_id, email, ..
    } => Ok((admin_id, email)),
    _ => Err(HttpError::forbidden(
      AUTHORIZATION_DENIED,
      "This operation requires a signed-in account.",
    )),
  }
}

/// A signed-in person whose role holds `capability`. Root holds them all.
pub fn require_capability(
  identity: &AuthIdentity,
  capability: Capability,
) -> Result<(&str, &str), HttpError> {
  match identity {
    AuthIdentity::Admin {
      admin_id,
      email,
      access,
      ..
    } if access.has(capability) => Ok((admin_id, email)),
    _ if Capability::ROOT_ONLY.contains(&capability) => Err(HttpError::forbidden(
      AUTHORIZATION_DENIED,
      "This operation requires the root administrator.",
    )),
    _ => Err(HttpError::forbidden(
      AUTHORIZATION_DENIED,
      "Your role does not allow this operation.",
    )),
  }
}

/// A person signed in to the Admin UI. Changes to who can reach what are
/// made there only, behind CSRF and recent sign-in checks.
pub fn require_browser(identity: &AuthIdentity) -> Result<(&str, &str), HttpError> {
  let result = require_person(identity)?;
  if !matches!(
    identity,
    AuthIdentity::Admin {
      kind: SessionKind::Browser,
      ..
    }
  ) {
    return Err(HttpError::forbidden(
      AUTHORIZATION_DENIED,
      "This operation is available from the Admin UI only.",
    ));
  }
  Ok(result)
}

/// [`require_capability`] from an Admin UI session only.
pub fn require_capability_browser(
  identity: &AuthIdentity,
  capability: Capability,
) -> Result<(&str, &str), HttpError> {
  let result = require_capability(identity, capability)?;
  require_browser(identity)?;
  Ok(result)
}

pub fn require_read_access(identity: &AuthIdentity) -> Result<(), HttpError> {
  match identity {
    AuthIdentity::Admin { .. } | AuthIdentity::ServiceAccount { .. } => Ok(()),
    AuthIdentity::Runner { .. } => Err(HttpError::forbidden(
      AUTHORIZATION_DENIED,
      "This operation is not available to runner tokens.",
    )),
  }
}

pub fn require_metadata_access(identity: &AuthIdentity) -> Result<(), HttpError> {
  match identity {
    AuthIdentity::Admin { .. } | AuthIdentity::ServiceAccount { .. } => Ok(()),
    _ => Err(HttpError::forbidden(
      AUTHORIZATION_DENIED,
      "This operation requires an authenticated account.",
    )),
  }
}

pub fn require_mutation(
  identity: &AuthIdentity,
  headers: &HeaderMap,
) -> Result<(), HttpError> {
  require_person(identity)?;
  if let AuthIdentity::Admin {
    kind: SessionKind::Browser,
    csrf_hash,
    ..
  } = identity
  {
    let provided = headers
      .get("x-dopbase-csrf")
      .and_then(|value| value.to_str().ok())
      .ok_or_else(|| HttpError::forbidden(AUTHORIZATION_DENIED, CSRF_REQUIRED_MESSAGE))?;
    let expected = csrf_hash
      .as_ref()
      .ok_or_else(|| HttpError::forbidden(AUTHORIZATION_DENIED, CSRF_REQUIRED_MESSAGE))?;
    let tab = token::tab_csrf(CSRF_TOKEN_PREFIX, expected);
    if token::hash(provided) != *expected && !token::constant_time_eq(provided, &tab) {
      return Err(HttpError::forbidden(
        AUTHORIZATION_DENIED,
        CSRF_REQUIRED_MESSAGE,
      ));
    }
  }
  Ok(())
}

pub fn require_recent_browser_auth(identity: &AuthIdentity) -> Result<(), HttpError> {
  if let AuthIdentity::Admin {
    kind: SessionKind::Browser,
    recent_auth_at,
    ..
  } = identity
    && Utc::now() - *recent_auth_at > Duration::minutes(RECENT_AUTHENTICATION_MINUTES)
  {
    return Err(HttpError::forbidden(
      "RECENT_AUTHENTICATION_REQUIRED",
      "Please confirm your password before continuing.",
    ));
  }
  require_person(identity).map(|_| ())
}

pub struct OptionalIdentity(pub Option<AuthIdentity>);

impl FromRequestParts<AppState> for OptionalIdentity {
  type Rejection = Infallible;
  async fn from_request_parts(
    parts: &mut Parts,
    state: &AppState,
  ) -> Result<Self, Self::Rejection> {
    Ok(Self(
      AuthIdentity::from_request_parts(parts, state).await.ok(),
    ))
  }
}
