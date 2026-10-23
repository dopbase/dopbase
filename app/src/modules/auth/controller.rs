use super::{error::AuthError, model::*, oidc, oidc::FLOW_TTL, service};
use crate::{
  constants::limits::BROWSER_SESSION_ABSOLUTE_HOURS,
  extractors::require_mutation,
  http::{HttpResponse, HttpResponseFormat},
  models::AuthIdentity,
  state::AppState,
};
use axum::{
  extract::{Query, State},
  http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header},
  response::{IntoResponse, Response},
};
use chrono::Duration;

/// The pending sign-in is bound to the browser that started it with this
/// cookie, which the callback must present alongside the provider's `state`.
const SSO_BINDING_COOKIE: &str = "dopbase_sso_state";
/// Attributes shared by both forms of the binding cookie. `SameSite=Lax` is
/// deliberate: the provider's callback is a cross-site top-level navigation,
/// which a `Strict` cookie would not be sent on, while `Lax` still withholds
/// the cookie from cross-site subrequests.
const SSO_BINDING_ATTRIBUTES: &str = "; Path=/api/v1/auth; HttpOnly; SameSite=Lax";

/// `; Secure` when the instance is served over HTTPS.
fn secure_attribute(state: &AppState) -> &'static str {
  if state.config.public_url.starts_with("https://") {
    "; Secure"
  } else {
    ""
  }
}

/// Build the browser session cookie for a raw session token.
fn session_cookie(
  state: &AppState,
  raw: &str,
) -> Result<HeaderValue, AuthError> {
  let max_age = Duration::hours(BROWSER_SESSION_ABSOLUTE_HOURS).num_seconds();
  let cookie = format!(
    "dopbase_session={raw}; Path=/; HttpOnly; SameSite=Strict; Max-Age={max_age}{}",
    secure_attribute(state)
  );
  HeaderValue::from_str(&cookie).map_err(|_| crate::http::HttpError::internal())
}

/// Bind a pending sign-in to the browser that started it.
fn sso_binding_cookie(
  state: &AppState,
  value: &str,
) -> Result<HeaderValue, AuthError> {
  let cookie = format!(
    "{SSO_BINDING_COOKIE}={value}; Max-Age={}{}{}",
    FLOW_TTL.num_seconds(),
    SSO_BINDING_ATTRIBUTES,
    secure_attribute(state)
  );
  HeaderValue::from_str(&cookie).map_err(|_| crate::http::HttpError::internal())
}

/// Clear the sign-in binding cookie. Every callback response carries it, so a
/// completed or failed attempt leaves no value behind in the browser.
fn clear_sso_binding_cookie() -> HeaderValue {
  HeaderValue::from_str(&format!(
    "{SSO_BINDING_COOKIE}=; Max-Age=0{SSO_BINDING_ATTRIBUTES}"
  ))
  .expect("static cookie attributes")
}

/// Read a cookie from the request headers.
fn request_cookie(
  headers: &HeaderMap,
  name: &str,
) -> Option<String> {
  headers
    .get(header::COOKIE)?
    .to_str()
    .ok()?
    .split(';')
    .map(str::trim)
    .find_map(|cookie| {
      cookie
        .strip_prefix(name)
        .and_then(|value| value.strip_prefix('='))
    })
    .map(|value| value.to_owned())
}

/// Build a redirect response, optionally setting cookies.
fn redirect_response(
  location: &str,
  cookies: Vec<HeaderValue>,
) -> Response {
  let mut response = StatusCode::SEE_OTHER.into_response();
  if let Ok(value) = HeaderValue::from_str(location) {
    response.headers_mut().insert(header::LOCATION, value);
  }
  for cookie in cookies {
    response.headers_mut().append(header::SET_COOKIE, cookie);
  }
  response
}

/// Log in
///
/// Verify the email and password and start a session. Browser sessions
/// also receive an HTTP-only session cookie and a CSRF token. CLI sessions
/// receive a bearer token. Failed attempts are rate limited per email.
#[utoipa::path(
  post,
  path = crate::constants::api::auth::LOGIN,
  tag = "authentication",
  request_body = LoginRequest,
  responses(
    (status = 200, description = "Session started", body = inline(HttpResponseFormat<LoginResponse>)),
    (status = 401, description = "The email or password is incorrect", body = crate::http::ErrorBody),
    (status = 422, description = "Login input is invalid", body = crate::http::ErrorBody),
    (status = 429, description = "Too many login attempts. Try again later", body = crate::http::ErrorBody),
  ),
)]
pub async fn login(
  State(state): State<AppState>,
  axum::Json(request): axum::Json<LoginRequest>,
) -> Result<HttpResponse<LoginResponse>, AuthError> {
  let result = service::login(&state, request).await?;
  let mut response = HttpResponse::ok(result.response, "LOGIN_SUCCEEDED");
  if let Some(raw) = result.cookie_token {
    response = response.with_header(
      HeaderName::from_static("set-cookie"),
      session_cookie(&state, &raw)?,
    );
  }
  Ok(response)
}

/// Log out
///
/// Revoke the current session and clear the browser session cookie.
/// Requires the CSRF header for browser sessions.
#[utoipa::path(
  post,
  path = crate::constants::api::auth::LOGOUT,
  tag = "authentication",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  responses(
    (status = 200, description = "Session revoked", body = inline(HttpResponseFormat<serde_json::Value>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "The request is missing or fails the CSRF check", body = crate::http::ErrorBody),
  ),
)]
pub async fn logout(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
) -> Result<HttpResponse<serde_json::Value>, AuthError> {
  require_mutation(&identity, &headers)?;
  service::logout(&state, &identity).await?;
  Ok(HttpResponse::done("LOGOUT_SUCCEEDED").with_header(
    HeaderName::from_static("set-cookie"),
    HeaderValue::from_static("dopbase_session=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0"),
  ))
}

/// Show the current session
///
/// Describe the authenticated admin session: email, session kind (browser
/// or CLI), and whether the authentication is still considered recent.
#[utoipa::path(
  get,
  path = crate::constants::api::auth::SESSION,
  tag = "authentication",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  responses(
    (status = 200, description = "Session details fetched", body = inline(HttpResponseFormat<SessionResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "A runner token has no human session", body = crate::http::ErrorBody),
  ),
)]
pub async fn session(identity: AuthIdentity) -> Result<HttpResponse<SessionResponse>, AuthError> {
  Ok(HttpResponse::ok(
    service::session(&identity)?,
    "SESSION_FETCHED",
  ))
}

/// Re-authenticate
///
/// Verify the password again to refresh the session's recent-authentication
/// timestamp, which sensitive operations such as revealing secrets require.
/// Requires the CSRF header for browser sessions.
#[utoipa::path(
  post,
  path = crate::constants::api::auth::REAUTHENTICATE,
  tag = "authentication",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  request_body = ReauthenticateRequest,
  responses(
    (status = 200, description = "Recent authentication refreshed", body = inline(HttpResponseFormat<serde_json::Value>)),
    (status = 401, description = "The password is incorrect", body = crate::http::ErrorBody),
    (status = 403, description = "The request is missing or fails the CSRF check", body = crate::http::ErrorBody),
  ),
)]
pub async fn reauthenticate(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  axum::Json(request): axum::Json<ReauthenticateRequest>,
) -> Result<HttpResponse<serde_json::Value>, AuthError> {
  require_mutation(&identity, &headers)?;
  service::reauthenticate(&state, &identity, &request.password).await?;
  Ok(HttpResponse::done("REAUTHENTICATION_SUCCEEDED"))
}

/// Change the account password
///
/// Verify the current password, set a new one, and revoke every active
/// session of the account, including the current one. Requires the CSRF
/// header for browser sessions.
#[utoipa::path(
  post,
  path = crate::constants::api::auth::CHANGE_PASSWORD,
  tag = "authentication",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  request_body = ChangePasswordRequest,
  responses(
    (status = 200, description = "Password changed. All sessions were revoked", body = inline(HttpResponseFormat<serde_json::Value>)),
    (status = 401, description = "The current password is incorrect", body = crate::http::ErrorBody),
    (status = 403, description = "The request is missing or fails the CSRF check", body = crate::http::ErrorBody),
    (status = 422, description = "The new password does not meet the policy", body = crate::http::ErrorBody),
  ),
)]
pub async fn change_password(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  axum::Json(request): axum::Json<ChangePasswordRequest>,
) -> Result<HttpResponse<serde_json::Value>, AuthError> {
  require_mutation(&identity, &headers)?;
  service::change_password(&state, &identity, request).await?;
  Ok(HttpResponse::done("PASSWORD_CHANGED").with_header(
    HeaderName::from_static("set-cookie"),
    HeaderValue::from_static("dopbase_session=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0"),
  ))
}

/// Report single sign-on availability
///
/// Tell the login screen whether an operator configured OIDC sign-in. This
/// endpoint is public so the unauthenticated login page can read it.
#[utoipa::path(
  get,
  path = crate::constants::api::auth::OIDC_CONFIG,
  tag = "authentication",
  responses(
    (status = 200, description = "Single sign-on availability", body = inline(HttpResponseFormat<OidcConfigResponse>)),
  ),
)]
pub async fn oidc_config(State(state): State<AppState>) -> HttpResponse<OidcConfigResponse> {
  HttpResponse::ok(oidc::config(&state), "OIDC_CONFIG_FETCHED")
}

/// Start single sign-on
///
/// Redirect the browser to the configured identity provider with a single-use
/// `state` and a PKCE challenge. The optional `redirect` query parameter names
/// the in-app path to return to; only relative paths are honored.
#[utoipa::path(
  get,
  path = crate::constants::api::auth::OIDC_START,
  tag = "authentication",
  params(("redirect" = Option<String>, Query, description = "In-app path to return to after sign-in")),
  responses(
    (status = 303, description = "Redirects to the identity provider"),
    (status = 404, description = "Single sign-on is not configured", body = crate::http::ErrorBody),
    (status = 502, description = "The identity provider could not be reached", body = crate::http::ErrorBody),
  ),
)]
pub async fn oidc_start(
  State(state): State<AppState>,
  Query(query): Query<OidcStartQuery>,
) -> Result<Response, AuthError> {
  let start = oidc::start(&state, query.redirect.as_deref()).await?;
  let cookie = sso_binding_cookie(&state, &start.state)?;
  Ok(redirect_response(&start.location, vec![cookie]))
}

/// Complete single sign-on
///
/// Verify the provider's response against the browser that started the
/// sign-in, link the identity to an existing account on first sign-in, apply
/// the mapped role, and start a browser session. Every outcome is a redirect:
/// success returns to the app, failure returns to the login screen with a
/// generic notice.
#[utoipa::path(
  get,
  path = crate::constants::api::auth::OIDC_CALLBACK,
  tag = "authentication",
  params(
    ("code" = Option<String>, Query, description = "Authorization code from the provider"),
    ("state" = Option<String>, Query, description = "Single-use state value"),
    ("error" = Option<String>, Query, description = "Provider error code, when sign-in failed"),
  ),
  responses(
    (status = 303, description = "Redirects back into the app or to the login screen"),
  ),
)]
pub async fn oidc_callback(
  State(state): State<AppState>,
  headers: HeaderMap,
  Query(query): Query<OidcCallbackQuery>,
) -> Result<Response, AuthError> {
  let binding = request_cookie(&headers, SSO_BINDING_COOKIE);
  let mut cookies = vec![clear_sso_binding_cookie()];
  let redirect = match oidc::callback(&state, query, binding.as_deref()).await {
    Ok(redirect) => redirect,
    Err(error) => {
      tracing::warn!(?error, "OIDC callback failed");
      return Ok(redirect_response("/login?notice=oidc-failed", cookies));
    }
  };
  if let Some(raw) = redirect.cookie_token {
    cookies.push(session_cookie(&state, &raw)?);
  }
  Ok(redirect_response(&redirect.location, cookies))
}
