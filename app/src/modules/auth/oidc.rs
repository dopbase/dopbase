//! Browser single sign-on over OpenID Connect (authorization code + PKCE).
//!
//! The flow reuses the shared OIDC verifier for discovery, the code exchange,
//! and ID-token verification. It adds the browser-facing pieces: a single-use
//! `state` that is also bound to the browser that started the sign-in, a PKCE
//! verifier, the account link, and the role mapping.
//!
//! Accounts are provisioned by an administrator. The first sign-in links the
//! provider identity to an existing account matched by verified email; an email
//! that matches no account is refused rather than provisioned. The group claim
//! decides the account role, and the account is never left without one.

use super::{model::*, repository, service};
use crate::{
  http::HttpError,
  models::{AdminRole, SessionKind},
  modules::common,
  services::token,
  state::AppState,
};
use axum::http::StatusCode;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Duration, Utc};
use serde_json::json;
use url::Url;

/// How long a browser sign-in may stay pending before the `state` expires.
/// The controller reuses it for the lifetime of the binding cookie.
pub(crate) const FLOW_TTL: Duration = Duration::minutes(10);

/// Notice shown on the login screen when sign-in fails before authorization.
const NOTICE_FAILED: &str = "/login?notice=oidc-failed";
/// Notice shown when the provider identity is not authorized for this instance.
const NOTICE_DENIED: &str = "/login?notice=oidc-denied";

/// Where the browser should go to authorize the sign-in, and the `state` it
/// must present again on the callback. The controller sets the `state` as a
/// short-lived cookie so the callback can be bound to this browser.
pub struct StartRedirect {
  pub location: String,
  pub state: String,
}

/// Where the browser should go next after the callback, and the cookie to set.
pub struct LoginRedirect {
  pub location: String,
  pub cookie_token: Option<String>,
}

impl LoginRedirect {
  fn failed() -> Self {
    Self {
      location: NOTICE_FAILED.to_owned(),
      cookie_token: None,
    }
  }

  fn denied() -> Self {
    Self {
      location: NOTICE_DENIED.to_owned(),
      cookie_token: None,
    }
  }
}

/// Report whether single sign-on is available, for the login screen.
pub fn config(state: &AppState) -> OidcConfigResponse {
  match (state.oidc.as_ref(), state.oidc_login.as_ref()) {
    (Some(_), Some(login)) => OidcConfigResponse {
      enabled: true,
      issuer: Some(login.issuer.clone()),
    },
    _ => OidcConfigResponse {
      enabled: false,
      issuer: None,
    },
  }
}

/// Begin a sign-in: record the pending flow and return the authorization URL.
pub async fn start(
  state: &AppState,
  redirect: Option<&str>,
) -> Result<StartRedirect, HttpError> {
  let login = state.oidc_login.as_ref().ok_or_else(oidc_disabled)?;
  let verifier = state.oidc.as_ref().ok_or_else(HttpError::internal)?;
  let metadata = verifier
    .metadata(&login.issuer)
    .await
    .map_err(|_| provider_unavailable())?;
  let authorization_endpoint = metadata
    .authorization_endpoint
    .ok_or_else(HttpError::internal)?;

  let state_value = token::generate("").map_err(|_| HttpError::internal())?;
  let nonce = token::generate("").map_err(|_| HttpError::internal())?;
  let code_verifier = token::generate("").map_err(|_| HttpError::internal())?;
  let challenge = URL_SAFE_NO_PAD.encode(token::hash(&code_verifier));
  let flow = LoginFlow {
    state: state_value.clone(),
    nonce: nonce.clone(),
    code_verifier,
    redirect_to: safe_redirect(redirect),
  };

  let now = Utc::now();
  let mut tx = state.db.pool().begin().await?;
  repository::prune_login_flows(&mut tx, &(now - FLOW_TTL).to_rfc3339()).await?;
  repository::create_login_flow(&mut tx, &flow, &now.to_rfc3339()).await?;
  tx.commit().await?;

  let mut url = Url::parse(&authorization_endpoint).map_err(|_| HttpError::internal())?;
  url
    .query_pairs_mut()
    .append_pair("response_type", "code")
    .append_pair("client_id", &login.client_id)
    .append_pair(
      "redirect_uri",
      &login.callback_url(&state.config.public_url),
    )
    .append_pair("scope", &login.scopes.join(" "))
    .append_pair("state", &state_value)
    .append_pair("nonce", &nonce)
    .append_pair("code_challenge", &challenge)
    .append_pair("code_challenge_method", "S256");
  Ok(StartRedirect {
    location: url.to_string(),
    state: state_value,
  })
}

/// Complete a sign-in from the provider's callback.
///
/// `browser_state` is the `state` value the browser presents as the binding
/// cookie set when the sign-in started; it must equal the callback's `state`,
/// so a callback URL replayed into another browser is refused.
///
/// Every outcome is a redirect: the browser never sees a JSON error from this
/// endpoint. Failures are audited with a stable reason and then shown as a
/// generic notice, so a denial cannot be used to probe which accounts exist.
pub async fn callback(
  state: &AppState,
  query: OidcCallbackQuery,
  browser_state: Option<&str>,
) -> Result<LoginRedirect, HttpError> {
  let Some(login) = state.oidc_login.as_ref() else {
    return Ok(LoginRedirect::failed());
  };

  let Some(state_value) = query.state.as_deref() else {
    audit_failure(state, login, "state_missing", None).await;
    return Ok(LoginRedirect::failed());
  };
  // Check the browser binding before touching the database: an unbound
  // callback can neither consume the flow nor probe whether its state exists.
  if browser_state != Some(state_value) {
    audit_failure(state, login, "state_unbound", None).await;
    return Ok(LoginRedirect::failed());
  }
  let Some((flow, created_at)) =
    repository::consume_login_flow(state.db.pool(), state_value).await?
  else {
    audit_failure(state, login, "state_unknown", None).await;
    return Ok(LoginRedirect::failed());
  };
  if flow_expired(&created_at) {
    audit_failure(state, login, "state_expired", None).await;
    return Ok(LoginRedirect::failed());
  }
  if query.error.is_some() {
    audit_failure(state, login, "provider_error", None).await;
    return Ok(LoginRedirect::failed());
  }
  let Some(code) = query.code.as_deref() else {
    audit_failure(state, login, "code_missing", None).await;
    return Ok(LoginRedirect::failed());
  };
  let Some(verifier) = state.oidc.as_ref() else {
    return Ok(LoginRedirect::failed());
  };

  // Exchange the code, then verify the ID token exactly as any other assertion.
  let tokens = match verifier
    .exchange_code(
      login,
      code,
      &login.callback_url(&state.config.public_url),
      &flow.code_verifier,
    )
    .await
  {
    Ok(tokens) => tokens,
    Err(error) => {
      audit_failure(state, login, error.code(), None).await;
      return Ok(LoginRedirect::failed());
    }
  };
  let Some(id_token) = tokens.id_token.as_deref() else {
    audit_failure(state, login, "token_missing", None).await;
    return Ok(LoginRedirect::failed());
  };
  let claims = match verifier.verify(id_token).await {
    Ok(claims) => claims,
    Err(error) => {
      audit_failure(state, login, error.code(), None).await;
      return Ok(LoginRedirect::failed());
    }
  };
  if claims.string("nonce") != Some(flow.nonce.as_str()) {
    audit_failure(state, login, "nonce_mismatch", None).await;
    return Ok(LoginRedirect::failed());
  }
  let Some(subject) = claims.subject.clone() else {
    audit_failure(state, login, "subject_missing", None).await;
    return Ok(LoginRedirect::failed());
  };
  let email = match claims.string(&login.email_claim).and_then(normalize_email) {
    Some(email) => email,
    None => {
      audit_failure(state, login, "email_missing", None).await;
      return Ok(LoginRedirect::denied());
    }
  };
  if login.require_email_verified && claims.boolean("email_verified") != Some(true) {
    audit_failure(state, login, "email_unverified", Some(&email)).await;
    return Ok(LoginRedirect::denied());
  }
  let Some(mapped_role) = map_role(login, &claims) else {
    audit_failure(state, login, "role_unmapped", Some(&email)).await;
    return Ok(LoginRedirect::denied());
  };

  // Resolve the account: an existing link first, then a first-time match by
  // verified email. Anything else is refused.
  let linked = repository::oidc_identity(state.db.pool(), &login.issuer, &subject).await?;
  let (admin_id, account_email, account_role, is_new_link) = match linked {
    Some(row) => (row.0, row.1, row.2, false),
    None => match repository::admin_by_email(state.db.pool(), &email).await? {
      Some((admin_id, account_email, _, role)) => (admin_id, account_email, role, true),
      None => {
        audit_failure(state, login, "account_unknown", Some(&email)).await;
        return Ok(LoginRedirect::denied());
      }
    },
  };
  let account_role = match parse_role(&account_role) {
    Some(role) => role,
    None => {
      audit_failure(state, login, "account_unknown", Some(&email)).await;
      return Ok(LoginRedirect::denied());
    }
  };
  // AI agent accounts are service accounts and never sign in as a human.
  if account_role == AdminRole::AiAgent {
    audit_failure(state, login, "account_not_human", Some(&email)).await;
    return Ok(LoginRedirect::denied());
  }
  // The setup account keeps its elevated role: SSO must not grant or demote root.
  let final_role = if account_role == AdminRole::Root {
    AdminRole::Root
  } else {
    mapped_role
  };
  let role_changed = final_role != account_role;

  let session = service::prepare_session(SessionKind::Browser)?;
  let now = session.created_at.to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  if is_new_link {
    repository::link_identity(&mut tx, &admin_id, &login.issuer, &subject, &email, &now).await?;
    common::audit(
      &mut tx,
      final_role.audit_actor_type(),
      Some(&admin_id),
      Some(&account_email),
      "oidc.identity.linked",
      None,
      None,
      Some("oidc_provider"),
      Some(&login.issuer),
      json!({"issuer": login.issuer}),
    )
    .await?;
  } else {
    repository::touch_identity(&mut tx, &login.issuer, &subject, &email, &now).await?;
  }
  if role_changed {
    repository::set_role(&mut tx, &admin_id, final_role.as_str(), &now).await?;
  }
  service::insert_session(&mut tx, &session, &admin_id, SessionKind::Browser).await?;
  common::audit(
    &mut tx,
    final_role.audit_actor_type(),
    Some(&admin_id),
    Some(&account_email),
    "oidc.login.succeeded",
    None,
    None,
    Some("session"),
    Some(&session.id),
    json!({
      "issuer": login.issuer,
      "role": final_role.as_str(),
      "roleChanged": role_changed,
    }),
  )
  .await?;
  tx.commit().await?;

  Ok(LoginRedirect {
    location: flow.redirect_to.unwrap_or_else(|| "/".to_owned()),
    cookie_token: Some(session.raw_token),
  })
}

/// Map the groups claim to the highest role the mapping grants.
fn map_role(
  login: &crate::services::oidc::OidcLogin,
  claims: &crate::services::oidc::VerifiedClaims,
) -> Option<AdminRole> {
  let mut mapped = None;
  for group in claims.groups(&login.groups_claim) {
    match login.roles.get(group) {
      Some(AdminRole::Admin) => return Some(AdminRole::Admin),
      Some(AdminRole::Member) => mapped = Some(AdminRole::Member),
      _ => {}
    }
  }
  mapped
}

/// Normalize and validate an email claim. Invalid values count as absent.
fn normalize_email(value: &str) -> Option<String> {
  common::validate_email(value).ok()
}

fn parse_role(value: &str) -> Option<AdminRole> {
  match value {
    "root" => Some(AdminRole::Root),
    "admin" => Some(AdminRole::Admin),
    "member" => Some(AdminRole::Member),
    "ai_agent" => Some(AdminRole::AiAgent),
    _ => None,
  }
}

fn flow_expired(created_at: &str) -> bool {
  match chrono::DateTime::parse_from_rfc3339(created_at) {
    Ok(created) => Utc::now() - created.with_timezone(&Utc) > FLOW_TTL,
    Err(_) => true,
  }
}

/// Accept only in-app absolute paths, so the callback cannot be turned into an
/// open redirect.
fn safe_redirect(value: Option<&str>) -> Option<String> {
  let value = value?;
  if value.is_empty() || value.len() > 512 {
    return None;
  }
  if !value.starts_with('/') || value.starts_with("//") || value.contains('\\') {
    return None;
  }
  if value.bytes().any(|byte| byte < 0x20) {
    return None;
  }
  Some(value.to_owned())
}

async fn audit_failure(
  state: &AppState,
  login: &crate::services::oidc::OidcLogin,
  reason: &str,
  email: Option<&str>,
) {
  let _ = common::audit_standalone(
    state.db.pool(),
    "anonymous",
    None,
    email,
    "oidc.login.failed",
    None,
    None,
    Some("oidc_provider"),
    Some(&login.issuer),
    json!({"reason": reason, "issuer": login.issuer}),
  )
  .await;
}

fn oidc_disabled() -> HttpError {
  HttpError::not_found(
    "OIDC_DISABLED",
    "Single sign-on is not configured on this instance.",
  )
}

fn provider_unavailable() -> HttpError {
  HttpError::new(
    StatusCode::BAD_GATEWAY,
    "OIDC_PROVIDER_UNAVAILABLE",
    "The identity provider could not be reached.",
  )
}
