use super::{device, error::AuthError, model::*, service};
use crate::{
  extractors::{OptionalIdentity, require_mutation},
  http::{HttpResponse, HttpResponseFormat},
  models::AuthIdentity,
  state::AppState,
};
use axum::{
  extract::State,
  http::{HeaderMap, HeaderName, HeaderValue},
};

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
    (status = 401, description = "The email or password is wrong. Unknown emails get the same reply. With 1 to 3 tries left, the message says how many remain before a pause", body = crate::http::ErrorBody),
    (status = 422, description = "Login input is invalid", body = crate::http::ErrorBody),
    (status = 429, description = "Too many tries. The message says how many minutes to wait", body = crate::http::ErrorBody),
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

/// The browser session cookie. Every way of signing in to the Admin UI
/// sets it the same way.
pub(crate) fn session_cookie(
  state: &AppState,
  raw: &str,
) -> Result<HeaderValue, crate::http::HttpError> {
  let secure = if state.config.public_url.starts_with("https://") {
    "; Secure"
  } else {
    ""
  };
  HeaderValue::from_str(&format!(
    "dopbase_session={raw}; Path=/; HttpOnly; SameSite=Strict; Max-Age=86400{secure}"
  ))
  .map_err(|_| crate::http::HttpError::internal())
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
pub async fn session(
  State(state): State<AppState>,
  identity: AuthIdentity,
) -> Result<HttpResponse<SessionResponse>, AuthError> {
  Ok(HttpResponse::ok(
    service::session(&state, &identity).await?,
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

/// List sign-in methods
///
/// Report which ways of signing in this server accepts. The login page and
/// `dopbase login` use it to choose a default. No authentication required.
#[utoipa::path(
  get,
  path = crate::constants::api::auth::METHODS,
  tag = "authentication",
  responses(
    (status = 200, description = "Sign-in methods listed", body = inline(HttpResponseFormat<AuthMethodsResponse>)),
  ),
)]
pub async fn methods(
  State(state): State<AppState>
) -> Result<HttpResponse<AuthMethodsResponse>, AuthError> {
  Ok(HttpResponse::ok(
    device::methods(&state).await?,
    "AUTH_METHODS_FETCHED",
  ))
}

/// Start a browser CLI login
///
/// Create a short-lived login request. The CLI keeps the device code and
/// polls with it; the user approves the user code in the Admin UI. Codes
/// expire after ten minutes. No authentication required.
#[utoipa::path(
  post,
  path = crate::constants::api::auth::CLI_DEVICE,
  tag = "authentication",
  request_body = DeviceStartRequest,
  responses(
    (status = 201, description = "Login request created", body = inline(HttpResponseFormat<DeviceStartResponse>)),
    (status = 429, description = "Too many pending login requests", body = crate::http::ErrorBody),
  ),
)]
pub async fn cli_device(
  State(state): State<AppState>,
  OptionalIdentity(identity): OptionalIdentity,
  axum::Json(request): axum::Json<DeviceStartRequest>,
) -> Result<HttpResponse<DeviceStartResponse>, AuthError> {
  Ok(HttpResponse::created(
    device::start(&state, identity.as_ref(), request).await?,
    "CLI_LOGIN_STARTED",
  ))
}

/// Poll a browser CLI login
///
/// Return `pending` until the request is approved, then a CLI session
/// exactly once. Polling faster than the returned interval yields
/// `slowDown`. No authentication required; the device code is the secret.
#[utoipa::path(
  post,
  path = crate::constants::api::auth::CLI_TOKEN,
  tag = "authentication",
  request_body = DeviceCodeRequest,
  responses(
    (status = 200, description = "Request state, with a session once approved", body = inline(HttpResponseFormat<DeviceTokenResponse>)),
    (status = 400, description = "The device code is unknown, cancelled, or already used", body = crate::http::ErrorBody),
    (status = 403, description = "The request was denied in the browser", body = crate::http::ErrorBody),
    (status = 410, description = "The request expired before approval", body = crate::http::ErrorBody),
  ),
)]
pub async fn cli_token(
  State(state): State<AppState>,
  axum::Json(request): axum::Json<DeviceCodeRequest>,
) -> Result<HttpResponse<DeviceTokenResponse>, AuthError> {
  Ok(HttpResponse::ok(
    device::poll(&state, &request.device_code).await?,
    "CLI_LOGIN_POLLED",
  ))
}

/// Cancel a browser CLI login
///
/// Invalidate a pending or approved request that has not issued a session
/// yet. Unknown codes succeed silently. No authentication required.
#[utoipa::path(
  post,
  path = crate::constants::api::auth::CLI_CANCEL,
  tag = "authentication",
  request_body = DeviceCodeRequest,
  responses(
    (status = 200, description = "Request cancelled", body = inline(HttpResponseFormat<serde_json::Value>)),
  ),
)]
pub async fn cli_cancel(
  State(state): State<AppState>,
  axum::Json(request): axum::Json<DeviceCodeRequest>,
) -> Result<HttpResponse<serde_json::Value>, AuthError> {
  device::cancel(&state, &request.device_code).await?;
  Ok(HttpResponse::done("CLI_LOGIN_CANCELLED"))
}

/// Look up a browser CLI login
///
/// Show where a pending request came from before approving it. Requires a
/// browser session and the CSRF header. Wrong codes are rate limited.
#[utoipa::path(
  post,
  path = crate::constants::api::auth::CLI_INSPECT,
  tag = "authentication",
  security(("cookieAuth" = [])),
  request_body = UserCodeRequest,
  responses(
    (status = 200, description = "Pending request found", body = inline(HttpResponseFormat<DeviceRequestDetails>)),
    (status = 403, description = "Not a browser session, or the CSRF check failed", body = crate::http::ErrorBody),
    (status = 404, description = "The code is invalid or has expired", body = crate::http::ErrorBody),
    (status = 429, description = "Too many wrong codes", body = crate::http::ErrorBody),
  ),
)]
pub async fn cli_inspect(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  axum::Json(request): axum::Json<UserCodeRequest>,
) -> Result<HttpResponse<DeviceRequestDetails>, AuthError> {
  require_mutation(&identity, &headers)?;
  Ok(HttpResponse::ok(
    device::inspect(&state, &identity, &request.user_code).await?,
    "CLI_LOGIN_FOUND",
  ))
}

/// Approve a browser CLI login
///
/// Let the CLI that shows this code sign in as the current user. Requires
/// a browser session, the CSRF header, and recent authentication.
#[utoipa::path(
  post,
  path = crate::constants::api::auth::CLI_APPROVE,
  tag = "authentication",
  security(("cookieAuth" = [])),
  request_body = UserCodeRequest,
  responses(
    (status = 200, description = "Request approved", body = inline(HttpResponseFormat<serde_json::Value>)),
    (status = 403, description = "Not a browser session, the CSRF check failed, or recent authentication is required", body = crate::http::ErrorBody),
    (status = 404, description = "The code is invalid or has expired", body = crate::http::ErrorBody),
    (status = 429, description = "Too many wrong codes", body = crate::http::ErrorBody),
  ),
)]
pub async fn cli_approve(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  axum::Json(request): axum::Json<UserCodeRequest>,
) -> Result<HttpResponse<serde_json::Value>, AuthError> {
  require_mutation(&identity, &headers)?;
  device::decide(&state, &identity, &request.user_code, true).await?;
  Ok(HttpResponse::done("CLI_LOGIN_APPROVED"))
}

/// Deny a browser CLI login
///
/// Refuse the request so the waiting CLI stops. Requires a browser session
/// and the CSRF header.
#[utoipa::path(
  post,
  path = crate::constants::api::auth::CLI_DENY,
  tag = "authentication",
  security(("cookieAuth" = [])),
  request_body = UserCodeRequest,
  responses(
    (status = 200, description = "Request denied", body = inline(HttpResponseFormat<serde_json::Value>)),
    (status = 403, description = "Not a browser session, or the CSRF check failed", body = crate::http::ErrorBody),
    (status = 404, description = "The code is invalid or has expired", body = crate::http::ErrorBody),
    (status = 429, description = "Too many wrong codes", body = crate::http::ErrorBody),
  ),
)]
pub async fn cli_deny(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  axum::Json(request): axum::Json<UserCodeRequest>,
) -> Result<HttpResponse<serde_json::Value>, AuthError> {
  require_mutation(&identity, &headers)?;
  device::decide(&state, &identity, &request.user_code, false).await?;
  Ok(HttpResponse::done("CLI_LOGIN_DENIED"))
}
