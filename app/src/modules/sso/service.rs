//! Single sign-on through one OpenID Connect provider.
//!
//! Root configures the provider and proves it works with a test sign-in
//! that links root's own identity. After that, a sign-in finds its account
//! by the stored (issuer, subject) pair. The first sign-in of an admin or
//! member links by email, but only when the provider says the email is
//! verified. Root is never linked by email, and unknown emails are refused:
//! accounts are still created by administrators.

use super::{
  model::*,
  presets,
  repository::{self, FlowRow, NewFlow, NewIdentity, ProviderFields, ProviderRow},
};
use crate::{
  constants::{
    api::{
      auth::SSO_CALLBACK,
      sso::{ACCOUNT_PAGE, CONFIRMED_PAGE, HANDOFF_PAGE, LOGIN_PAGE, SETTINGS_PAGE},
    },
    errors::{
      AUTHORIZATION_DENIED, PASSWORD_NOT_SET, RATE_LIMITED, SSO_CLIENT_INVALID,
      SSO_DISCOVERY_FAILED, SSO_HANDOFF_INVALID, SSO_ISSUER_INVALID, SSO_NAME_INVALID,
      SSO_NOT_CONFIGURED, SSO_NOT_LINKED, SSO_NOT_VERIFIED, SSO_PRESET_INVALID,
      SSO_PUBLIC_URL_INSECURE,
    },
    limits::{
      MAX_PENDING_SSO_FLOWS, MAX_PENDING_SSO_FLOWS_PER_CLIENT, SSO_FLOW_MINUTES,
      SSO_HANDOFF_SECONDS,
    },
    tokens::{
      SSO_BINDING_PREFIX, SSO_FLOW_ID_PREFIX, SSO_HANDOFF_PREFIX, SSO_IDENTITY_ID_PREFIX,
      SSO_PROVIDER_ID_PREFIX,
    },
  },
  extractors::{require_capability_browser, require_person, require_recent_browser_auth},
  http::HttpError,
  models::{AdminRole, AuthIdentity, SessionKind},
  modules::{
    auth::{model::LoginResponse, service::issue_session},
    common,
  },
  services::{
    authx::{self, Capability, SignIn},
    client_ip,
    crypto::sso_client_secret_aad,
    oidc::{self, Freshness, ProviderSettings, VerifiedIdentity},
    token,
  },
  state::AppState,
};
use axum::http::StatusCode;
use chrono::{Duration, Utc};

const MAX_DISPLAY_NAME_LENGTH: usize = 64;
const MAX_CLIENT_FIELD_LENGTH: usize = 4096;
const MAX_REDIRECT_LENGTH: usize = 512;
/// Slack between this server's clock and the provider's for `auth_time`.
const REAUTH_CLOCK_SKEW_SECONDS: i64 = 60;

async fn provider_row(state: &AppState) -> Result<Option<ProviderRow>, sqlx::Error> {
  repository::provider(state.db.pool()).await
}

/// What single sign-on currently allows, for the parts of sign-in that do
/// not belong to this module.
#[derive(Default)]
pub struct SsoPolicy {
  /// The button label, when single sign-on is enabled.
  pub enabled_name: Option<String>,
  /// Password login is refused for every account except root.
  pub require_sso: bool,
  /// The login page shows only the single sign-on button.
  pub hide_password_form: bool,
}

pub async fn policy(state: &AppState) -> Result<SsoPolicy, sqlx::Error> {
  Ok(match repository::enabled_policy(state.db.pool()).await? {
    Some((name, require_sso, hide_password_form)) => SsoPolicy {
      enabled_name: Some(name),
      require_sso,
      hide_password_form,
    },
    None => SsoPolicy::default(),
  })
}

/// Whether this account can confirm itself through the provider: it has a
/// linked identity and single sign-on is enabled. Returns the button label.
pub async fn confirmation_name(
  state: &AppState,
  admin_id: &str,
) -> Result<Option<String>, sqlx::Error> {
  repository::confirmation_name(state.db.pool(), admin_id).await
}

fn provider_settings(
  state: &AppState,
  row: &ProviderRow,
) -> Result<ProviderSettings, HttpError> {
  let secret = state
    .crypto
    .decrypt_master(
      &row.client_secret_ciphertext,
      &row.client_secret_nonce,
      &sso_client_secret_aad(&row.id),
    )
    .map_err(|_| HttpError::internal())?;
  Ok(ProviderSettings {
    issuer: row.issuer.clone(),
    client_id: row.client_id.clone(),
    client_secret: String::from_utf8(secret).map_err(|_| HttpError::internal())?,
  })
}

pub fn redirect_uri(state: &AppState) -> String {
  format!(
    "{}{SSO_CALLBACK}",
    state.config.public_url.trim_end_matches('/')
  )
}

/// HTTPS, or plain HTTP to this machine for local development.
fn is_secure_url(value: &str) -> bool {
  let Ok(url) = url::Url::parse(value) else {
    return false;
  };
  match url.scheme() {
    "https" => true,
    "http" => {
      matches!(url.host(), Some(url::Host::Domain("localhost")))
        || matches!(url.host(), Some(url::Host::Ipv4(ip)) if ip.is_loopback())
        || matches!(url.host(), Some(url::Host::Ipv6(ip)) if ip.is_loopback())
    }
    _ => false,
  }
}

fn invalid(
  code: &str,
  message: &str,
) -> HttpError {
  HttpError::new(StatusCode::UNPROCESSABLE_ENTITY, code, message)
}

/// The provider sends codes back to the public URL, so over plain HTTP they
/// would cross the network readable by anyone on the path. Only loopback is
/// exempt, for trying it out on one machine.
fn require_secure_public_url(state: &AppState) -> Result<(), HttpError> {
  if is_secure_url(&state.config.public_url) {
    return Ok(());
  }
  Err(HttpError::conflict(
    SSO_PUBLIC_URL_INSECURE,
    &format!(
      "Single sign-on needs an HTTPS public URL, and this server's is {}. Set it with --public-url, DOPBASE_PUBLIC_URL, or public_url in server.toml.",
      state.config.public_url
    ),
  ))
}

/// The issuer as entered, or with its trailing slash toggled, whichever the
/// provider's discovery document confirms.
async fn discover_issuer(issuer: &str) -> Option<String> {
  let toggled = match issuer.strip_suffix('/') {
    Some(trimmed) => trimmed.to_owned(),
    None => format!("{issuer}/"),
  };
  for candidate in [issuer.to_owned(), toggled] {
    if oidc::discover(&candidate).await.is_ok() {
      return Some(candidate);
    }
  }
  None
}

/// Changing single sign-on decides who can reach the instance, so it takes
/// the root account in the Admin UI with a fresh password.
fn require_root_change(identity: &AuthIdentity) -> Result<(&str, &str), HttpError> {
  let result = require_capability_browser(identity, Capability::SsoManage)?;
  require_recent_browser_auth(identity)?;
  Ok(result)
}

fn view(
  row: ProviderRow,
  linked_accounts: i64,
  passwordless_accounts: i64,
) -> ProviderView {
  ProviderView {
    id: row.id,
    preset: row.preset,
    display_name: row.display_name,
    issuer: row.issuer,
    client_id: row.client_id,
    enabled: row.enabled,
    require_sso: row.require_sso,
    hide_password_form: row.hide_password_form,
    verified_at: row.verified_at,
    linked_accounts,
    passwordless_accounts,
    updated_at: row.updated_at,
  }
}

pub async fn get_provider(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<ProviderSettingsResponse, HttpError> {
  require_capability_browser(identity, Capability::SsoManage)?;
  let provider = match provider_row(state).await? {
    Some(row) => {
      let linked = repository::linked_count(state.db.pool(), &row.id).await?;
      let passwordless = repository::passwordless_count(state.db.pool()).await?;
      Some(view(row, linked, passwordless))
    }
    None => None,
  };
  Ok(ProviderSettingsResponse {
    provider,
    redirect_uri: redirect_uri(state),
    public_url: state.config.public_url.clone(),
    public_url_secure: is_secure_url(&state.config.public_url),
    presets: presets::all().iter().map(PresetView::from).collect(),
  })
}

pub async fn put_provider(
  state: &AppState,
  identity: &AuthIdentity,
  request: ProviderRequest,
) -> Result<ProviderSettingsResponse, HttpError> {
  let (admin_id, email) = require_root_change(identity)?;
  require_secure_public_url(state)?;
  let preset = presets::find(&request.preset)
    .ok_or_else(|| invalid(SSO_PRESET_INVALID, "Choose a provider from the list."))?;
  let display_name = request.display_name.trim();
  if display_name.is_empty() || display_name.chars().count() > MAX_DISPLAY_NAME_LENGTH {
    return Err(invalid(
      SSO_NAME_INVALID,
      "Enter a button label of up to 64 characters.",
    ));
  }
  let issuer = request.issuer.trim();
  if !is_secure_url(issuer) || issuer.contains(['?', '#']) {
    return Err(invalid(
      SSO_ISSUER_INVALID,
      "Enter the provider's HTTPS issuer URL.",
    ));
  }
  if preset.rejects_issuer(issuer) {
    return Err(invalid(
      SSO_ISSUER_INVALID,
      preset
        .reject_issuer_message
        .as_deref()
        .unwrap_or("This issuer URL cannot be used with this provider."),
    ));
  }
  let client_id = request.client_id.trim();
  if client_id.is_empty() || client_id.len() > MAX_CLIENT_FIELD_LENGTH {
    return Err(invalid(SSO_CLIENT_INVALID, "Enter the client ID."));
  }
  let client_secret = request
    .client_secret
    .as_deref()
    .map(str::trim)
    .filter(|secret| !secret.is_empty());
  if client_secret.is_some_and(|secret| secret.len() > MAX_CLIENT_FIELD_LENGTH) {
    return Err(invalid(
      SSO_CLIENT_INVALID,
      "The client secret is too long.",
    ));
  }

  let existing = provider_row(state).await?;
  if existing.is_none() && client_secret.is_none() {
    return Err(invalid(SSO_CLIENT_INVALID, "Enter the client secret."));
  }
  // Issuers must match exactly, and some end in a slash (Auth0) while most
  // do not. Accept the one whose discovery document agrees.
  let issuer = discover_issuer(issuer).await.ok_or_else(|| {
    invalid(
      SSO_DISCOVERY_FAILED,
      "Could not read the provider's discovery document. Check the issuer URL.",
    )
  })?;
  let issuer = issuer.as_str();

  let now = Utc::now().to_rfc3339();
  let provider_id = existing.as_ref().map_or_else(
    || token::public_id(SSO_PROVIDER_ID_PREFIX),
    |row| row.id.clone(),
  );
  let encrypted = client_secret
    .map(|secret| {
      state
        .crypto
        .encrypt_master(secret.as_bytes(), &sso_client_secret_aad(&provider_id))
        .map_err(|_| HttpError::internal())
    })
    .transpose()?;

  // New client settings have not been proven by a test sign-in yet, so an
  // existing provider goes back to disabled until root signs in again.
  let reverify = existing
    .as_ref()
    .is_some_and(|row| row.issuer != issuer || row.client_id != client_id || encrypted.is_some());
  let (ciphertext, nonce) = match (encrypted, &existing) {
    (Some(encrypted), _) => encrypted,
    (None, Some(row)) => (
      row.client_secret_ciphertext.clone(),
      row.client_secret_nonce.clone(),
    ),
    (None, None) => unreachable!("a new provider has a secret, checked above"),
  };
  let fields = ProviderFields {
    id: &provider_id,
    preset: &preset.id,
    display_name,
    issuer,
    client_id,
    client_secret_ciphertext: &ciphertext,
    client_secret_nonce: &nonce,
    now: &now,
  };

  let mut tx = state.db.pool().begin().await?;
  let mut identities_removed = 0;
  let action = if existing.is_none() {
    repository::insert_provider(&mut tx, fields).await?;
    "sso_provider.created"
  } else {
    repository::update_provider(&mut tx, fields, reverify).await?;
    // Identities from another issuer can never match a sign-in again.
    // People link again by verified email on their next sign-in.
    identities_removed =
      repository::delete_identities_from_other_issuers(&mut tx, &provider_id, issuer).await?;
    "sso_provider.updated"
  };
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(admin_id),
    Some(email),
    action,
    None,
    None,
    Some("sso_provider"),
    Some(&provider_id),
    serde_json::json!({"issuer":issuer,"preset":preset.id,"secretChanged":client_secret.is_some(),"identitiesRemoved":identities_removed}),
  )
  .await?;
  tx.commit().await?;
  state.oidc.clear().await;
  get_provider(state, identity).await
}

pub async fn set_enabled(
  state: &AppState,
  identity: &AuthIdentity,
  enabled: bool,
) -> Result<ProviderSettingsResponse, HttpError> {
  let (admin_id, email) = require_root_change(identity)?;
  let row = provider_row(state)
    .await?
    .ok_or_else(|| HttpError::not_found(SSO_NOT_CONFIGURED, "Single sign-on is not configured."))?;
  if enabled {
    if row.verified_at.is_none() {
      return Err(HttpError::conflict(
        SSO_NOT_VERIFIED,
        "Complete a test sign-in with these settings before enabling single sign-on.",
      ));
    }
    require_secure_public_url(state)?;
  }
  let mut tx = state.db.pool().begin().await?;
  // Requiring single sign-on only makes sense while it is on.
  repository::set_enabled(&mut tx, &row.id, enabled, &Utc::now().to_rfc3339()).await?;
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(admin_id),
    Some(email),
    if enabled {
      "sso_provider.enabled"
    } else {
      "sso_provider.disabled"
    },
    None,
    None,
    Some("sso_provider"),
    Some(&row.id),
    serde_json::json!({}),
  )
  .await?;
  tx.commit().await?;
  get_provider(state, identity).await
}

pub async fn set_required(
  state: &AppState,
  identity: &AuthIdentity,
  required: bool,
) -> Result<ProviderSettingsResponse, HttpError> {
  let (admin_id, email) = require_root_change(identity)?;
  let row = provider_row(state)
    .await?
    .filter(|row| row.enabled)
    .ok_or_else(|| {
      HttpError::conflict(
        SSO_NOT_CONFIGURED,
        "Turn on single sign-on before requiring it.",
      )
    })?;
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  repository::set_required(&mut tx, &row.id, required, &now).await?;
  // Sessions started with a password would otherwise outlive the policy,
  // CLI ones by up to 90 days. Everyone but root signs in again.
  let revoked = if required && !row.require_sso {
    repository::revoke_non_root_sessions(&mut tx, &now).await?
  } else {
    0
  };
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(admin_id),
    Some(email),
    "sso_provider.policy_changed",
    None,
    None,
    Some("sso_provider"),
    Some(&row.id),
    serde_json::json!({"requireSso":required,"revokedSessions":revoked}),
  )
  .await?;
  tx.commit().await?;
  get_provider(state, identity).await
}

/// Hide or show the password form on the login page. Only the page changes:
/// the API and the CLI keep accepting passwords, which `require_sso` controls.
pub async fn set_password_form_hidden(
  state: &AppState,
  identity: &AuthIdentity,
  hidden: bool,
) -> Result<ProviderSettingsResponse, HttpError> {
  let (admin_id, email) = require_root_change(identity)?;
  let row = provider_row(state)
    .await?
    .ok_or_else(|| HttpError::not_found(SSO_NOT_CONFIGURED, "Single sign-on is not configured."))?;
  let mut tx = state.db.pool().begin().await?;
  repository::set_password_form_hidden(&mut tx, &row.id, hidden, &Utc::now().to_rfc3339()).await?;
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(admin_id),
    Some(email),
    "sso_provider.policy_changed",
    None,
    None,
    Some("sso_provider"),
    Some(&row.id),
    serde_json::json!({"hidePasswordForm":hidden}),
  )
  .await?;
  tx.commit().await?;
  get_provider(state, identity).await
}

pub async fn delete_provider(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<(), HttpError> {
  let (admin_id, email) = require_root_change(identity)?;
  let row = provider_row(state)
    .await?
    .ok_or_else(|| HttpError::not_found(SSO_NOT_CONFIGURED, "Single sign-on is not configured."))?;
  let mut tx = state.db.pool().begin().await?;
  // Linked identities and pending sign-ins go with it.
  repository::delete_provider(&mut tx, &row.id).await?;
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(admin_id),
    Some(email),
    "sso_provider.deleted",
    None,
    None,
    Some("sso_provider"),
    Some(&row.id),
    serde_json::json!({"issuer":row.issuer}),
  )
  .await?;
  tx.commit().await?;
  state.oidc.clear().await;
  Ok(())
}

/// Only Admin UI paths, so the callback cannot send the browser elsewhere.
/// Control characters are refused because browsers strip tabs and newlines
/// from URLs, which can turn `/\t/evil.example` into `//evil.example`.
fn safe_redirect(value: Option<String>) -> Option<String> {
  value.filter(|path| {
    path.len() <= MAX_REDIRECT_LENGTH
      && path.starts_with('/')
      && !path.starts_with("//")
      && !path.contains("://")
      && !path.contains('\\')
      && !path.chars().any(char::is_control)
  })
}

pub struct StartedFlow {
  pub authorize_url: String,
  pub binding: String,
}

async fn create_flow(
  state: &AppState,
  row: &ProviderRow,
  purpose: &str,
  session: Option<(&str, &str)>,
  redirect_to: Option<String>,
) -> Result<StartedFlow, HttpError> {
  // Also checked here: the public URL may have changed since setup.
  require_secure_public_url(state)?;
  let settings = provider_settings(state, row)?;
  let freshness = if purpose == "reauth" {
    // Some providers (Google) reject `prompt=login`; `max_age=0` alone asks
    // them to sign in again.
    Freshness::Required {
      prompt_login: presets::resolve(&row.preset).prompt_login,
    }
  } else {
    Freshness::Any
  };
  let request =
    oidc::authorization_request(&state.oidc, &settings, &redirect_uri(state), freshness)
      .await
      .map_err(|_| {
        HttpError::new(
          StatusCode::BAD_GATEWAY,
          SSO_DISCOVERY_FAILED,
          "The identity provider could not be reached. Try again later.",
        )
      })?;
  let binding = token::generate(SSO_BINDING_PREFIX).map_err(|_| HttpError::internal())?;
  let now = Utc::now();
  let now_text = now.to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  repository::delete_expired_flows(&mut tx, &now_text).await?;
  let client = client_ip::current().map(|address| address.to_string());
  let (pending, pending_here) = repository::pending_flows(&mut tx, client.as_deref()).await?;
  if pending >= MAX_PENDING_SSO_FLOWS || pending_here >= MAX_PENDING_SSO_FLOWS_PER_CLIENT {
    return Err(HttpError::new(
      StatusCode::TOO_MANY_REQUESTS,
      RATE_LIMITED,
      "Too many sign-ins in progress. Try again later.",
    ));
  }
  repository::insert_flow(
    &mut tx,
    NewFlow {
      id: &token::public_id(SSO_FLOW_ID_PREFIX),
      state_hash: &token::hash(&request.state),
      provider_id: &row.id,
      purpose,
      nonce: &request.nonce,
      pkce_verifier: &request.pkce_verifier,
      binding_hash: &token::hash(&binding),
      session_id: session.map(|(session_id, _)| session_id),
      admin_id: session.map(|(_, admin_id)| admin_id),
      redirect_to: redirect_to.as_deref(),
      client_ip: client.as_deref(),
      now: &now_text,
      expires_at: &(now + Duration::minutes(SSO_FLOW_MINUTES)).to_rfc3339(),
    },
  )
  .await?;
  tx.commit().await?;
  Ok(StartedFlow {
    authorize_url: request.url,
    binding,
  })
}

/// Start a sign-in. Errors become a login page location, because this
/// endpoint is a browser navigation, not an API call.
pub async fn start_login(
  state: &AppState,
  redirect: Option<String>,
) -> Result<StartedFlow, String> {
  let row = match provider_row(state).await {
    Ok(Some(row)) if row.enabled => row,
    Ok(_) => return Err(error_location(Target::Login, "disabled")),
    Err(_) => return Err(error_location(Target::Login, "failed")),
  };
  create_flow(state, &row, "login", None, safe_redirect(redirect))
    .await
    .map_err(|_| error_location(Target::Login, "unavailable"))
}

/// Start linking the current account. Root may do this before the provider
/// is enabled: that is the test sign-in.
pub async fn start_link(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<StartedFlow, HttpError> {
  let (admin_id, _) = require_person(identity)?;
  let AuthIdentity::Admin {
    kind: SessionKind::Browser,
    session_id,
    role,
    ..
  } = identity
  else {
    return Err(HttpError::forbidden(
      AUTHORIZATION_DENIED,
      "Link single sign-on from the Admin UI.",
    ));
  };
  let row = provider_row(state)
    .await?
    .filter(|row| row.enabled || *role == AdminRole::Root)
    .ok_or_else(|| HttpError::not_found(SSO_NOT_CONFIGURED, "Single sign-on is not enabled."))?;
  create_flow(state, &row, "link", Some((session_id, admin_id)), None).await
}

/// Confirm the current user again at the provider, for accounts that have
/// no password to confirm with. The browser opens the returned URL in a
/// popup; the session counts as recently authenticated once it returns.
pub async fn start_reauth(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<StartedFlow, HttpError> {
  let (admin_id, _) = require_person(identity)?;
  let AuthIdentity::Admin {
    kind: SessionKind::Browser,
    session_id,
    ..
  } = identity
  else {
    return Err(HttpError::forbidden(
      AUTHORIZATION_DENIED,
      "Confirm with single sign-on from the Admin UI.",
    ));
  };
  if confirmation_name(state, admin_id).await?.is_none() {
    return Err(HttpError::conflict(
      SSO_NOT_LINKED,
      "This account has no single sign-on link to confirm with.",
    ));
  }
  let row = provider_row(state)
    .await?
    .ok_or_else(|| HttpError::not_found(SSO_NOT_CONFIGURED, "Single sign-on is not enabled."))?;
  create_flow(state, &row, "reauth", Some((session_id, admin_id)), None).await
}

#[derive(Clone, Copy)]
enum Target {
  Login,
  Settings,
  Account,
  Confirmation,
}

fn error_location(
  target: Target,
  reason: &str,
) -> String {
  let page = match target {
    Target::Login => LOGIN_PAGE,
    Target::Settings => SETTINGS_PAGE,
    Target::Account => ACCOUNT_PAGE,
    Target::Confirmation => CONFIRMED_PAGE,
  };
  format!("{page}?sso_error={reason}")
}

/// Why a callback failed. `reason` goes into the redirect and the audit
/// record, never provider output.
struct Failure {
  target: Target,
  reason: &'static str,
  email: Option<String>,
}

impl Failure {
  fn new(
    target: Target,
    reason: &'static str,
  ) -> Self {
    Self {
      target,
      reason,
      email: None,
    }
  }
}

impl From<sqlx::Error> for Failure {
  fn from(_: sqlx::Error) -> Self {
    Self::new(Target::Login, "failed")
  }
}

impl From<HttpError> for Failure {
  fn from(_: HttpError) -> Self {
    Self::new(Target::Login, "failed")
  }
}

/// Handle the provider's redirect back. Always returns a location in the
/// Admin UI: the handoff page on a successful login, or a page with an
/// `sso_error` reason.
pub async fn callback(
  state: &AppState,
  binding: Option<&str>,
  query: CallbackQuery,
) -> String {
  let limit_key = format!(
    "sso_callback:{}",
    client_ip::current().map_or_else(|| "unknown".to_owned(), |ip| ip.to_string())
  );
  if !state.rate_limiter.check(&limit_key).await {
    return error_location(Target::Login, "rate_limited");
  }
  // Without the binding cookie the callback cannot be tied to the browser
  // that started it. Refuse before touching the database, so a replayed
  // callback URL can neither use nor burn someone else's sign-in.
  let (Some(binding), Some(state_param)) = (binding, query.state.as_deref()) else {
    state.rate_limiter.failure(&limit_key).await;
    return error_location(Target::Login, "expired");
  };
  let now = Utc::now().to_rfc3339();
  let flow: Result<Option<FlowRow>, sqlx::Error> = repository::consume_flow(
    state.db.pool(),
    &token::hash(state_param),
    &token::hash(binding),
    &now,
  )
  .await;
  // Early returns stay in let-else. Inside a match that binds `flow`, CodeQL
  // reads the returned error page as the flow itself and reports it reaching
  // the nonce check.
  let Ok(found) = flow else {
    return error_location(Target::Login, "failed");
  };
  let Some(flow) = found else {
    state.rate_limiter.failure(&limit_key).await;
    return error_location(Target::Login, "expired");
  };

  match finish_callback(state, &flow, &query).await {
    Ok(location) => location,
    Err(failure) => {
      state.rate_limiter.failure(&limit_key).await;
      if let Ok(mut conn) = state.db.pool().acquire().await {
        let _ = repository::delete_flow(&mut conn, &flow.id).await;
      }
      let _ = common::audit_standalone(
        state.db.pool(),
        "anonymous",
        None,
        failure.email.as_deref(),
        "sso_login.failed",
        None,
        None,
        None,
        None,
        serde_json::json!({"reason":failure.reason,"purpose":flow.purpose}),
      )
      .await;
      error_location(failure.target, failure.reason)
    }
  }
}

async fn finish_callback(
  state: &AppState,
  flow: &FlowRow,
  query: &CallbackQuery,
) -> Result<String, Failure> {
  let target = match flow.purpose.as_str() {
    "link" => {
      let mut conn = state.db.pool().acquire().await?;
      let access = match flow.admin_id.as_deref() {
        Some(admin_id) => authx::load_access(&mut conn, admin_id).await?,
        None => None,
      };
      // Whoever manages single sign-on tested it, so send them back there.
      if access.is_some_and(|access| access.has(Capability::SsoManage)) {
        Target::Settings
      } else {
        Target::Account
      }
    }
    "reauth" => Target::Confirmation,
    _ => Target::Login,
  };
  if query.error.is_some() {
    return Err(Failure::new(target, "cancelled"));
  }
  let code = query
    .code
    .as_deref()
    .ok_or_else(|| Failure::new(target, "failed"))?;
  let row = repository::provider_by_id(state.db.pool(), &flow.provider_id)
    .await?
    .ok_or_else(|| Failure::new(target, "disabled"))?;
  if flow.purpose != "link" && !row.enabled {
    return Err(Failure::new(target, "disabled"));
  }
  let settings = provider_settings(state, &row).map_err(|_| Failure::new(target, "failed"))?;
  let verified = oidc::exchange(
    &state.oidc,
    &settings,
    &redirect_uri(state),
    code,
    &flow.pkce_verifier,
    &flow.nonce,
  )
  .await
  .map_err(|_| Failure::new(target, "failed"))?;

  match flow.purpose.as_str() {
    "link" => link_identity(state, flow, &row, &verified, target).await,
    "reauth" => confirm_identity(state, flow, &verified).await,
    _ => login_identity(state, flow, &row, &verified).await,
  }
}

/// Finish a reauthentication: the provider must have signed in the same
/// person again just now, and the session that asked must still be live.
async fn confirm_identity(
  state: &AppState,
  flow: &FlowRow,
  verified: &VerifiedIdentity,
) -> Result<String, Failure> {
  let target = Target::Confirmation;
  let admin_id = flow
    .admin_id
    .as_deref()
    .ok_or_else(|| Failure::new(target, "failed"))?;
  let mut conn = state.db.pool().acquire().await?;
  let owner = repository::identity_owner(&mut conn, &verified.issuer, &verified.subject).await?;
  if owner.as_deref() != Some(admin_id) {
    // Name the account whose confirmation was attempted, so the record
    // shows who someone tried to act as.
    let email = repository::account(&mut conn, admin_id)
      .await?
      .map(|(email, _)| email);
    return Err(Failure {
      target,
      reason: "identity_mismatch",
      email,
    });
  }
  drop(conn);
  // `auth_time` before the request started means the provider reused an
  // older sign-in instead of asking again.
  let started = chrono::DateTime::parse_from_rfc3339(&flow.created_at)
    .map_err(|_| Failure::new(target, "failed"))?
    .with_timezone(&Utc);
  let fresh = verified
    .authenticated_at
    .is_some_and(|at| at >= started - Duration::seconds(REAUTH_CLOCK_SKEW_SECONDS));
  if !fresh {
    return Err(Failure::new(target, "reauth_not_fresh"));
  }
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  let account =
    repository::live_session_account(&mut tx, flow.session_id.as_deref(), admin_id, &now).await?;
  let (email, role) = account.ok_or_else(|| Failure::new(target, "expired"))?;
  repository::mark_recent_auth(&mut tx, flow.session_id.as_deref(), &now).await?;
  common::audit(
    &mut tx,
    audit_actor(&role),
    Some(admin_id),
    Some(&email),
    "admin.reauthenticated",
    None,
    None,
    None,
    None,
    serde_json::json!({"method":"sso"}),
  )
  .await?;
  repository::delete_flow(&mut tx, &flow.id).await?;
  tx.commit().await?;
  Ok(CONFIRMED_PAGE.to_owned())
}

/// The audit actor type for a stored role.
fn audit_actor(role: &str) -> &'static str {
  AdminRole::parse(role).map_or("admin", AdminRole::audit_actor_type)
}

/// Whether an account may hold a session: it is active and its role signs
/// in, unlike AI agents.
async fn signs_in(
  conn: &mut sqlx::SqliteConnection,
  admin_id: &str,
) -> Result<bool, sqlx::Error> {
  Ok(
    authx::load_access(conn, admin_id)
      .await?
      .is_some_and(|access| access.sign_in == SignIn::Interactive),
  )
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
  error
    .as_database_error()
    .is_some_and(|error| error.is_unique_violation())
}

async fn login_identity(
  state: &AppState,
  flow: &FlowRow,
  row: &ProviderRow,
  verified: &VerifiedIdentity,
) -> Result<String, Failure> {
  let now = Utc::now();
  let now_text = now.to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  let linked = repository::linked_account(&mut tx, &verified.issuer, &verified.subject).await?;

  let admin_id = match linked {
    Some((admin_id, _)) => {
      // A disabled or token-only account has no session, even with a stale
      // link.
      if !signs_in(&mut tx, &admin_id).await? {
        return Err(Failure::new(Target::Login, "account_not_found"));
      }
      repository::touch_identity(&mut tx, &verified.issuer, &verified.subject, &now_text).await?;
      admin_id
    }
    None => {
      let email = verified
        .email
        .as_deref()
        .map(|email| email.trim().to_lowercase())
        .ok_or_else(|| Failure::new(Target::Login, "email_unverified"))?;
      let failure = |reason| Failure {
        target: Target::Login,
        reason,
        email: Some(email.clone()),
      };
      // The preset names the claim that proves the email belongs to the
      // signer. Entra, for one, has no `email_verified` and uses `xms_edov`.
      if !verified.claim_is_true(&presets::resolve(&row.preset).email_verified_claim) {
        return Err(failure("email_unverified"));
      }
      let account = repository::account_by_email(&mut tx, &email).await?;
      let (admin_id, _) = account.ok_or_else(|| failure("account_not_found"))?;
      // Root links by hand, so the provider can never take over the
      // break-glass account. Accounts that never sign in don't link.
      let access = authx::load_access(&mut tx, &admin_id).await?;
      if access.as_ref().is_some_and(|access| access.is_root) {
        return Err(failure("root_not_linked"));
      }
      if !signs_in(&mut tx, &admin_id).await? {
        return Err(failure("account_not_found"));
      }
      let inserted = repository::insert_identity(
        &mut tx,
        NewIdentity {
          id: &token::public_id(SSO_IDENTITY_ID_PREFIX),
          admin_id: &admin_id,
          provider_id: &row.id,
          issuer: &verified.issuer,
          subject: &verified.subject,
          email_at_link: Some(&email),
          linked_via: "auto",
          now: &now_text,
          last_login_at: Some(&now_text),
        },
      )
      .await;
      match inserted {
        Ok(_) => {}
        // The account already has another identity from this provider.
        Err(error) if is_unique_violation(&error) => return Err(failure("already_linked")),
        Err(error) => return Err(error.into()),
      }
      common::audit(
        &mut tx,
        "admin",
        Some(&admin_id),
        Some(&email),
        "sso_identity.linked",
        None,
        None,
        Some("admin"),
        Some(&admin_id),
        serde_json::json!({"via":"auto","issuer":verified.issuer}),
      )
      .await?;
      admin_id
    }
  };

  let handoff =
    token::generate(SSO_HANDOFF_PREFIX).map_err(|_| Failure::new(Target::Login, "failed"))?;
  repository::set_handoff(
    &mut tx,
    &flow.id,
    &token::hash(&handoff),
    &(now + Duration::seconds(SSO_HANDOFF_SECONDS)).to_rfc3339(),
    &admin_id,
  )
  .await?;
  tx.commit().await?;
  // The fragment never reaches a server log; the Admin UI posts it back.
  Ok(format!("{HANDOFF_PAGE}#code={handoff}"))
}

async fn link_identity(
  state: &AppState,
  flow: &FlowRow,
  row: &ProviderRow,
  verified: &VerifiedIdentity,
  target: Target,
) -> Result<String, Failure> {
  let admin_id = flow
    .admin_id
    .as_deref()
    .ok_or_else(|| Failure::new(target, "failed"))?;
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  // The session that asked for the link must still be live: a sign-out in
  // the meantime cancels it.
  let account =
    repository::live_session_account(&mut tx, flow.session_id.as_deref(), admin_id, &now).await?;
  let (email, role) = account.ok_or_else(|| Failure::new(target, "expired"))?;

  let owner = repository::identity_owner(&mut tx, &verified.issuer, &verified.subject).await?;
  match owner.as_deref() {
    Some(owner) if owner != admin_id => {
      return Err(Failure {
        target,
        reason: "identity_taken",
        email: Some(email),
      });
    }
    Some(_) => {}
    None => {
      let inserted = repository::insert_identity(
        &mut tx,
        NewIdentity {
          id: &token::public_id(SSO_IDENTITY_ID_PREFIX),
          admin_id,
          provider_id: &row.id,
          issuer: &verified.issuer,
          subject: &verified.subject,
          email_at_link: verified.email.as_deref(),
          linked_via: "manual",
          now: &now,
          last_login_at: None,
        },
      )
      .await;
      match inserted {
        Ok(_) => {}
        Err(error) if is_unique_violation(&error) => {
          return Err(Failure {
            target,
            reason: "already_linked",
            email: Some(email),
          });
        }
        Err(error) => return Err(error.into()),
      }
      common::audit(
        &mut tx,
        audit_actor(&role),
        Some(admin_id),
        Some(&email),
        "sso_identity.linked",
        None,
        None,
        Some("admin"),
        Some(admin_id),
        serde_json::json!({"via":"manual","issuer":verified.issuer}),
      )
      .await?;
    }
  }
  // A link by whoever manages single sign-on is the test sign-in that
  // verifies the provider.
  let manages_sso = authx::load_access(&mut tx, admin_id)
    .await?
    .is_some_and(|access| access.has(Capability::SsoManage));
  if manages_sso && repository::mark_verified(&mut tx, &row.id, &now).await? {
    common::audit(
      &mut tx,
      "root",
      Some(admin_id),
      Some(&email),
      "sso_provider.verified",
      None,
      None,
      Some("sso_provider"),
      Some(&row.id),
      serde_json::json!({}),
    )
    .await?;
  }
  repository::delete_flow(&mut tx, &flow.id).await?;
  tx.commit().await?;
  Ok(match target {
    Target::Settings => format!("{SETTINGS_PAGE}?linked=1"),
    _ => format!("{ACCOUNT_PAGE}?sso=linked"),
  })
}

pub struct ExchangeResult {
  pub response: ExchangeResponse,
  pub cookie_token: String,
}

/// Trade the handoff code for a browser session. Same-origin and bound to
/// the browser that started the sign-in, so it works with the strict
/// session cookie and returns the CSRF token exactly like password login.
pub async fn exchange(
  state: &AppState,
  binding: Option<&str>,
  code: &str,
) -> Result<ExchangeResult, HttpError> {
  let invalid = || {
    HttpError::unauthorized(
      SSO_HANDOFF_INVALID,
      "This sign-in expired or was opened in another browser. Try again.",
    )
  };
  let binding = binding.ok_or_else(invalid)?;
  let now = Utc::now();
  let mut tx = state.db.pool().begin().await?;
  let flow = repository::take_handoff(
    &mut tx,
    &token::hash(code),
    &token::hash(binding),
    &now.to_rfc3339(),
  )
  .await?;
  let (admin_id, redirect_to) = flow.ok_or_else(invalid)?;
  let admin_id = admin_id.ok_or_else(invalid)?;
  let (email, role_value) = repository::account(&mut tx, &admin_id)
    .await?
    .ok_or_else(invalid)?;
  let role = AdminRole::parse(&role_value).ok_or_else(invalid)?;
  let session = issue_session(&mut tx, &admin_id, SessionKind::Browser).await?;
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
    serde_json::json!({"kind":"browser","method":"sso"}),
  )
  .await?;
  tx.commit().await?;
  Ok(ExchangeResult {
    response: ExchangeResponse {
      session: LoginResponse {
        admin_id,
        email,
        role,
        session_kind: SessionKind::Browser,
        token: None,
        csrf_token: session.csrf,
        last_login_at: Some(session.created_at.to_rfc3339()),
      },
      redirect_to: safe_redirect(redirect_to),
    },
    cookie_token: session.token,
  })
}

pub async fn identity_status(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<IdentityResponse, HttpError> {
  let (admin_id, _) = require_person(identity)?;
  let provider = provider_row(state).await?;
  let linked = match &provider {
    Some(row) => repository::identity_of(state.db.pool(), admin_id, &row.id).await?,
    None => None,
  };
  Ok(IdentityResponse {
    available: provider.as_ref().is_some_and(|row| row.enabled),
    provider_name: provider.map(|row| row.display_name),
    linked: linked.is_some(),
    email_at_link: linked.as_ref().and_then(|(email, _)| email.clone()),
    linked_at: linked.map(|(_, created_at)| created_at),
  })
}

/// Remove the current account's link. Only for accounts with a password, so
/// nobody unlinks the only way they can sign in.
pub async fn unlink_self(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<(), HttpError> {
  let (admin_id, email) = require_person(identity)?;
  if !matches!(
    identity,
    AuthIdentity::Admin {
      kind: SessionKind::Browser,
      ..
    }
  ) {
    return Err(HttpError::forbidden(
      AUTHORIZATION_DENIED,
      "Unlink single sign-on from the Admin UI.",
    ));
  }
  require_recent_browser_auth(identity)?;
  let hash = repository::password_hash(state.db.pool(), admin_id)
    .await?
    .unwrap_or_default();
  if !common::has_password(&hash) {
    return Err(HttpError::conflict(
      PASSWORD_NOT_SET,
      "Set a password before unlinking single sign-on.",
    ));
  }
  unlink(state, identity, admin_id, email).await
}

/// Remove every provider identity linked to `admin_id` and audit it.
pub async fn unlink(
  state: &AppState,
  identity: &AuthIdentity,
  admin_id: &str,
  target_email: &str,
) -> Result<(), HttpError> {
  let (actor_id, actor_email) = require_person(identity)?;
  let mut tx = state.db.pool().begin().await?;
  let removed = repository::delete_identities_of(&mut tx, admin_id).await?;
  if removed == 0 {
    return Err(HttpError::not_found(
      SSO_NOT_LINKED,
      "This account has no single sign-on link.",
    ));
  }
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(actor_id),
    Some(actor_email),
    "sso_identity.unlinked",
    None,
    None,
    Some("admin"),
    Some(admin_id),
    serde_json::json!({"email":target_email}),
  )
  .await?;
  tx.commit().await?;
  Ok(())
}
