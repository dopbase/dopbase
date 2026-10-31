//! Browser-approved CLI login. The CLI asks for a device code, shows the
//! user a short user code, and polls until someone signed in to the Admin UI
//! approves that code. The CLI never sees a password or a browser session,
//! so this works the same whether the browser session came from a password
//! or from single sign-on.

use super::{
  model::*,
  repository::{self, NewDeviceRequest},
  service::issue_session,
};
use crate::{
  constants::{
    api::auth::CLI_APPROVE_PAGE,
    errors::{
      AUTHORIZATION_DENIED, DEVICE_ACCESS_DENIED, DEVICE_CODE_EXPIRED, DEVICE_CODE_INVALID,
      RATE_LIMITED,
    },
    limits::{
      CLI_DEVICE_CODE_MINUTES, CLI_DEVICE_POLL_INTERVAL_SECONDS, CLI_SESSION_ABSOLUTE_DAYS,
      MAX_PENDING_CLI_DEVICE_REQUESTS, MAX_PENDING_CLI_DEVICE_REQUESTS_PER_CLIENT,
    },
    tokens::{DEVICE_CODE_PREFIX, DEVICE_REQUEST_ID_PREFIX},
  },
  extractors::{require_person, require_recent_browser_auth},
  http::HttpError,
  models::{AdminRole, AuthIdentity, SessionKind},
  modules::common,
  services::{client_ip, token},
  state::AppState,
};
use axum::http::StatusCode;
use chrono::{DateTime, Duration, Utc};

/// Consonants only (RFC 8628 section 6.1): no vowels to spell words, and
/// nothing that reads as another character.
const USER_CODE_ALPHABET: &[u8] = b"BCDFGHJKLMNPQRSTVWXZ";
const USER_CODE_LENGTH: usize = 8;
const MAX_HOSTNAME_LENGTH: usize = 64;
const MAX_CLIENT_LENGTH: usize = 80;

fn generate_user_code() -> Result<String, HttpError> {
  let mut code = String::with_capacity(USER_CODE_LENGTH);
  // Rejection sampling keeps every letter equally likely: 240 is the
  // largest multiple of the alphabet size that fits in a byte.
  let limit = (256 / USER_CODE_ALPHABET.len() * USER_CODE_ALPHABET.len()) as u8;
  while code.len() < USER_CODE_LENGTH {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| HttpError::internal())?;
    for byte in bytes {
      if byte < limit && code.len() < USER_CODE_LENGTH {
        code.push(USER_CODE_ALPHABET[byte as usize % USER_CODE_ALPHABET.len()] as char);
      }
    }
  }
  Ok(code)
}

/// `WDJB-MJHT`, `wdjb mjht`, and `WDJBMJHT` are the same code.
fn normalize_user_code(value: &str) -> Option<String> {
  let code: String = value
    .chars()
    .filter(|character| !matches!(character, '-' | ' '))
    .map(|character| character.to_ascii_uppercase())
    .collect();
  (code.len() == USER_CODE_LENGTH && code.bytes().all(|byte| USER_CODE_ALPHABET.contains(&byte)))
    .then_some(code)
}

fn display_user_code(code: &str) -> String {
  format!("{}-{}", &code[..4], &code[4..])
}

/// The hostname is shown to the approver, so keep it short and plain.
fn sanitize_hostname(value: Option<&str>) -> Option<String> {
  let hostname: String = value?
    .trim()
    .chars()
    .filter(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '.' | '_'))
    .take(MAX_HOSTNAME_LENGTH)
    .collect();
  (!hostname.is_empty()).then_some(hostname)
}

/// The client name is shown to the approver too. Unlike a hostname it has
/// spaces, so keep printable ASCII and fold runs of whitespace into one.
fn sanitize_client(value: Option<&str>) -> Option<String> {
  let printable: String = value?
    .chars()
    .map(|character| {
      if character.is_whitespace() {
        ' '
      } else {
        character
      }
    })
    .filter(|character| character.is_ascii_graphic() || *character == ' ')
    .collect();
  let client: String = printable
    .split(' ')
    .filter(|word| !word.is_empty())
    .collect::<Vec<_>>()
    .join(" ")
    .chars()
    .take(MAX_CLIENT_LENGTH)
    .collect();
  let client = client.trim_end();
  (!client.is_empty()).then(|| client.to_owned())
}

fn parse_time(value: &str) -> Result<DateTime<Utc>, HttpError> {
  DateTime::parse_from_rfc3339(value)
    .map(|time| time.with_timezone(&Utc))
    .map_err(|_| HttpError::internal())
}

fn invalid_code() -> HttpError {
  HttpError::bad_request(
    DEVICE_CODE_INVALID,
    "This login request is invalid or was already used. Run `dopbase login` again.",
  )
}

fn unknown_user_code() -> HttpError {
  HttpError::not_found(
    DEVICE_CODE_INVALID,
    "This code is invalid or has expired. Check the code in your terminal.",
  )
}

fn rate_limited() -> HttpError {
  HttpError::new(
    StatusCode::TOO_MANY_REQUESTS,
    RATE_LIMITED,
    "Too many attempts. Please try again later.",
  )
}

/// Approving hands out a long-lived CLI session, so only a human signed in
/// to the Admin UI may do it, never another CLI session or a token.
fn require_browser_human(identity: &AuthIdentity) -> Result<(&str, &str, &str), HttpError> {
  let (admin_id, email) = require_person(identity)?;
  match identity {
    AuthIdentity::Admin {
      kind: SessionKind::Browser,
      session_id,
      ..
    } => Ok((admin_id, email, session_id)),
    _ => Err(HttpError::forbidden(
      AUTHORIZATION_DENIED,
      "Approve CLI logins from the Admin UI.",
    )),
  }
}

pub async fn methods(state: &AppState) -> Result<AuthMethodsResponse, HttpError> {
  let policy = crate::modules::sso::service::policy(state).await?;
  Ok(AuthMethodsResponse {
    // Root keeps password login when single sign-on is required; clients
    // still offer it, just not as the default.
    password: !policy.require_sso,
    password_form: !(policy.enabled_name.is_some() && policy.hide_password_form),
    sso: policy.enabled_name.map(|name| SsoMethod { name }),
    cli_browser: true,
  })
}

pub async fn start(
  state: &AppState,
  identity: Option<&AuthIdentity>,
  request: DeviceStartRequest,
) -> Result<DeviceStartResponse, HttpError> {
  // A confirmation is for the CLI session making the request, so it needs
  // one; a login needs nothing.
  let target_session = match request.purpose.unwrap_or_default() {
    DevicePurpose::Login => None,
    DevicePurpose::Reauth => match identity {
      Some(AuthIdentity::Admin {
        kind: SessionKind::Cli,
        session_id,
        ..
      }) => Some(session_id.as_str()),
      _ => {
        return Err(HttpError::forbidden(
          AUTHORIZATION_DENIED,
          "Only a signed-in CLI session can ask for browser confirmation.",
        ));
      }
    },
  };
  let purpose = if target_session.is_some() {
    "reauth"
  } else {
    "login"
  };
  let now = Utc::now();
  let expires_at = now + Duration::minutes(CLI_DEVICE_CODE_MINUTES);
  let hostname = sanitize_hostname(request.hostname.as_deref());
  let client = sanitize_client(request.client.as_deref());
  let ip_address = client_ip::current().map(|address| address.to_string());
  let device_code = token::generate(DEVICE_CODE_PREFIX).map_err(|_| HttpError::internal())?;

  let now_text = now.to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  // Expired rows stay for an hour so a late poll still reads "expired"
  // rather than "invalid", then this recurring path removes them.
  repository::delete_stale_device_requests(&mut tx, &(now - Duration::hours(1)).to_rfc3339())
    .await?;
  let (pending, pending_here) =
    repository::pending_device_requests(&mut tx, ip_address.as_deref(), &now_text).await?;
  if pending >= MAX_PENDING_CLI_DEVICE_REQUESTS
    || pending_here >= MAX_PENDING_CLI_DEVICE_REQUESTS_PER_CLIENT
  {
    return Err(rate_limited());
  }

  // A user code collision is unlikely (20^8 codes), but retry rather than
  // fail the login when one happens.
  let mut user_code = None;
  for _ in 0..3 {
    let candidate = generate_user_code()?;
    let inserted = repository::insert_device_request(
      &mut tx,
      NewDeviceRequest {
        id: &token::public_id(DEVICE_REQUEST_ID_PREFIX),
        device_code_hash: &token::hash(&device_code),
        user_code_hash: &token::hash(&candidate),
        purpose,
        target_session_id: target_session,
        client_hostname: hostname.as_deref(),
        client_name: client.as_deref(),
        client_ip: ip_address.as_deref(),
        now: &now_text,
        expires_at: &expires_at.to_rfc3339(),
      },
    )
    .await?;
    if inserted {
      user_code = Some(candidate);
      break;
    }
  }
  let user_code = user_code.ok_or_else(HttpError::internal)?;
  tx.commit().await?;

  let user_code = display_user_code(&user_code);
  let verification_uri = format!(
    "{}{CLI_APPROVE_PAGE}",
    state.config.public_url.trim_end_matches('/')
  );
  Ok(DeviceStartResponse {
    verification_uri_complete: format!("{verification_uri}?code={user_code}"),
    verification_uri,
    device_code,
    user_code,
    expires_in: CLI_DEVICE_CODE_MINUTES * 60,
    interval: CLI_DEVICE_POLL_INTERVAL_SECONDS,
  })
}

pub async fn poll(
  state: &AppState,
  device_code: &str,
) -> Result<DeviceTokenResponse, HttpError> {
  let now = Utc::now();
  let row = repository::device_request(state.db.pool(), &token::hash(device_code))
    .await?
    .ok_or_else(invalid_code)?;
  match row.status.as_str() {
    "consumed" | "cancelled" => return Err(invalid_code()),
    "denied" => {
      return Err(HttpError::forbidden(
        DEVICE_ACCESS_DENIED,
        "The login request was denied in the browser.",
      ));
    }
    _ => {}
  }
  if now >= parse_time(&row.expires_at)? {
    return Err(HttpError::new(
      StatusCode::GONE,
      DEVICE_CODE_EXPIRED,
      "The login request expired before it was approved. Run `dopbase login` again.",
    ));
  }

  if row.status == "pending" {
    let too_fast = match &row.last_polled_at {
      // Half a second of slack absorbs timer jitter on the client.
      Some(last) => {
        now - parse_time(last)?
          < Duration::seconds(CLI_DEVICE_POLL_INTERVAL_SECONDS) - Duration::milliseconds(500)
      }
      None => false,
    };
    repository::touch_device_request(state.db.pool(), &row.id, &now.to_rfc3339()).await?;
    return Ok(DeviceTokenResponse {
      status: if too_fast {
        DevicePollStatus::SlowDown
      } else {
        DevicePollStatus::Pending
      },
      session: None,
    });
  }

  let mut tx = state.db.pool().begin().await?;
  // The status check and the flip happen in one statement, so two polls
  // racing on the same approved code cannot both receive a session.
  let approved = repository::consume_device_request(&mut tx, &row.id).await?;
  let (approved_by, purpose, target_session) = approved.ok_or_else(invalid_code)?;
  let admin_id = approved_by.ok_or_else(invalid_code)?;
  let (email, role_value) = repository::account(&mut tx, &admin_id)
    .await?
    .ok_or_else(invalid_code)?;
  let role = AdminRole::parse(&role_value).ok_or_else(HttpError::internal)?;
  if purpose == "reauth" {
    let confirmed = repository::confirm_session_of(
      &mut tx,
      target_session.as_deref(),
      &admin_id,
      &now.to_rfc3339(),
    )
    .await?;
    if !confirmed {
      return Err(invalid_code());
    }
    common::audit(
      &mut tx,
      role.audit_actor_type(),
      Some(&admin_id),
      Some(&email),
      "admin.reauthenticated",
      None,
      None,
      Some("session"),
      target_session.as_deref(),
      serde_json::json!({"method":"browser","requestId":row.id}),
    )
    .await?;
    tx.commit().await?;
    return Ok(DeviceTokenResponse {
      status: DevicePollStatus::Approved,
      session: None,
    });
  }
  let session = issue_session(&mut tx, &admin_id, SessionKind::Cli).await?;
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
    serde_json::json!({"kind":"cli","method":"browser","requestId":row.id}),
  )
  .await?;
  tx.commit().await?;
  Ok(DeviceTokenResponse {
    status: DevicePollStatus::Approved,
    session: Some(LoginResponse {
      admin_id,
      email,
      role,
      session_kind: SessionKind::Cli,
      token: Some(session.token),
      csrf_token: None,
      last_login_at: Some(session.created_at.to_rfc3339()),
    }),
  })
}

/// Cancelling an unknown or finished request is not an error, so the
/// endpoint says nothing about which device codes exist.
pub async fn cancel(
  state: &AppState,
  device_code: &str,
) -> Result<(), HttpError> {
  repository::cancel_device_request(
    state.db.pool(),
    &token::hash(device_code),
    &Utc::now().to_rfc3339(),
  )
  .await?;
  Ok(())
}

/// Count a wrong user code against the approving session. User codes are
/// short, so guessing has to stay expensive.
async fn user_code_hash(
  state: &AppState,
  session_id: &str,
  user_code: &str,
) -> Result<(String, Vec<u8>), HttpError> {
  let limit_key = format!("cli_user_code:{session_id}");
  if !state.rate_limiter.check(&limit_key).await {
    return Err(rate_limited());
  }
  match normalize_user_code(user_code) {
    Some(code) => Ok((limit_key, token::hash(&code))),
    None => {
      state.rate_limiter.failure(&limit_key).await;
      Err(unknown_user_code())
    }
  }
}

pub async fn inspect(
  state: &AppState,
  identity: &AuthIdentity,
  user_code: &str,
) -> Result<DeviceRequestDetails, HttpError> {
  let (admin_id, _, session_id) = require_browser_human(identity)?;
  let (limit_key, hash) = user_code_hash(state, session_id, user_code).await?;
  let details =
    repository::pending_device_request(state.db.pool(), &hash, admin_id, &Utc::now().to_rfc3339())
      .await?;
  let Some(row) = details else {
    state.rate_limiter.failure(&limit_key).await;
    return Err(unknown_user_code());
  };
  Ok(DeviceRequestDetails {
    hostname: row.client_hostname,
    client: row.client_name,
    ip_address: row.client_ip,
    created_at: row.created_at,
    expires_at: row.expires_at,
    purpose: if row.purpose == "reauth" {
      DevicePurpose::Reauth
    } else {
      DevicePurpose::Login
    },
    // The same limit `issue_session` gives a CLI session.
    session_days: u32::try_from(CLI_SESSION_ABSOLUTE_DAYS).map_err(|_| HttpError::internal())?,
  })
}

pub async fn decide(
  state: &AppState,
  identity: &AuthIdentity,
  user_code: &str,
  approve: bool,
) -> Result<(), HttpError> {
  let (admin_id, email, session_id) = require_browser_human(identity)?;
  if approve {
    require_recent_browser_auth(identity)?;
  }
  let (limit_key, hash) = user_code_hash(state, session_id, user_code).await?;
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  let decided = repository::decide_device_request(&mut tx, &hash, admin_id, approve, &now).await?;
  let Some((request_id, hostname)) = decided else {
    drop(tx);
    state.rate_limiter.failure(&limit_key).await;
    return Err(unknown_user_code());
  };
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(admin_id),
    Some(email),
    if approve {
      "cli_login.approved"
    } else {
      "cli_login.denied"
    },
    None,
    None,
    Some("cli_device_request"),
    Some(&request_id),
    serde_json::json!({"hostname":hostname}),
  )
  .await?;
  tx.commit().await?;
  state.rate_limiter.clear(&limit_key).await;
  Ok(())
}
