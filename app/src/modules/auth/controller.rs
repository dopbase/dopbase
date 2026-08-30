use super::{error::AuthError, model::*, service};
use crate::{
  extractors::require_mutation,
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
/// also receive an HTTP-only session cookie and a CSRF token; CLI sessions
/// receive a bearer token. Failed attempts are rate limited per email.
#[utoipa::path(
  post,
  path = "/api/v1/auth/login",
  tag = "authentication",
  request_body = LoginRequest,
  responses(
    (status = 200, description = "Session started", body = inline(HttpResponseFormat<LoginResponse>)),
    (status = 401, description = "The email or password is incorrect", body = crate::http::ErrorBody),
    (status = 422, description = "Login input is invalid", body = crate::http::ErrorBody),
    (status = 429, description = "Too many login attempts; try again later", body = crate::http::ErrorBody),
  ),
)]
pub async fn login(
  State(state): State<AppState>,
  axum::Json(request): axum::Json<LoginRequest>,
) -> Result<HttpResponse<LoginResponse>, AuthError> {
  let result = service::login(&state, request).await?;
  let mut response = HttpResponse::ok(result.response, "LOGIN_SUCCEEDED");
  if let Some(raw) = result.cookie_token {
    let secure = if state.config.public_url.starts_with("https://") {
      "; Secure"
    } else {
      ""
    };
    let cookie =
      format!("dopbase_session={raw}; Path=/; HttpOnly; SameSite=Strict; Max-Age=86400{secure}");
    response = response.with_header(
      HeaderName::from_static("set-cookie"),
      HeaderValue::from_str(&cookie).map_err(|_| crate::http::HttpError::internal())?,
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
  path = "/api/v1/auth/logout",
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
  path = "/api/v1/auth/session",
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
  path = "/api/v1/auth/reauthenticate",
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
  path = "/api/v1/auth/change-password",
  tag = "authentication",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  request_body = ChangePasswordRequest,
  responses(
    (status = 200, description = "Password changed; all sessions were revoked", body = inline(HttpResponseFormat<serde_json::Value>)),
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
