use super::{error::BootstrapError, model::*, service};
use crate::{
  http::{HttpResponse, HttpResponseFormat},
  state::AppState,
};
use axum::{
  extract::State,
  http::{HeaderName, HeaderValue},
};

/// Show bootstrap status
///
/// Report whether the instance still needs its first administrator
/// (`setupRequired`) or is already ready.
#[utoipa::path(
  get,
  path = "/api/v1/bootstrap/status",
  tag = "bootstrap",
  responses(
    (status = 200, description = "Bootstrap state fetched", body = inline(HttpResponseFormat<BootstrapStatus>)),
    (status = 500, description = "An internal error occurred", body = crate::http::ErrorBody),
  ),
)]
pub async fn status(
  State(state): State<AppState>
) -> Result<HttpResponse<BootstrapStatus>, BootstrapError> {
  Ok(HttpResponse::ok(
    service::status(&state).await?,
    "BOOTSTRAP_STATUS",
  ))
}

/// Create the first administrator
///
/// Complete instance setup by creating the initial admin account. The
/// request must carry the setup token printed at first startup. On success
/// the setup window closes permanently, the response sets the session
/// cookie, and a CSRF token is returned for subsequent mutations.
#[utoipa::path(
  post,
  path = "/api/v1/bootstrap/admin",
  tag = "bootstrap",
  request_body = BootstrapAdminRequest,
  responses(
    (status = 201, description = "Administrator created and session started", body = inline(HttpResponseFormat<BootstrapAdminResponse>)),
    (status = 401, description = "The setup token is invalid", body = crate::http::ErrorBody),
    (status = 409, description = "This instance has already been initialized", body = crate::http::ErrorBody),
    (status = 422, description = "Email or password is invalid", body = crate::http::ErrorBody),
    (status = 429, description = "Too many setup attempts; try again later", body = crate::http::ErrorBody),
  ),
)]
pub async fn create_admin(
  State(state): State<AppState>,
  axum::Json(request): axum::Json<BootstrapAdminRequest>,
) -> Result<HttpResponse<BootstrapAdminResponse>, BootstrapError> {
  let created = service::create(&state, request).await?;
  let secure = if state.config.public_url.starts_with("https://") {
    "; Secure"
  } else {
    ""
  };
  let cookie = format!(
    "dopbase_session={}; Path=/; HttpOnly; SameSite=Strict; Max-Age=86400{}",
    created.session_token, secure
  );
  Ok(
    HttpResponse::created(created.response, "ADMIN_BOOTSTRAPPED").with_header(
      HeaderName::from_static("set-cookie"),
      HeaderValue::from_str(&cookie).map_err(|_| crate::http::HttpError::internal())?,
    ),
  )
}
