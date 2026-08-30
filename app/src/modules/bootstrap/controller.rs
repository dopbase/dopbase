use super::{error::BootstrapError, model::*, service};
use crate::{
  http::{ErrorBody, HttpResponse, HttpResponseFormat},
  state::AppState,
};
use axum::{
  extract::State,
  http::{HeaderName, HeaderValue},
};
#[utoipa::path(get,path="/api/v1/bootstrap/status",tag="bootstrap",responses((status=200,body=inline(HttpResponseFormat<BootstrapStatus>)),(status=500,body=ErrorBody)))]
pub async fn status(
  State(state): State<AppState>
) -> Result<HttpResponse<BootstrapStatus>, BootstrapError> {
  Ok(HttpResponse::ok(
    service::status(&state).await?,
    "BOOTSTRAP_STATUS",
  ))
}
#[utoipa::path(post,path="/api/v1/bootstrap/admin",tag="bootstrap",request_body=BootstrapAdminRequest,responses((status=201,body=inline(HttpResponseFormat<BootstrapAdminResponse>)),(status=401,body=ErrorBody),(status=409,body=ErrorBody),(status=422,body=ErrorBody),(status=429,body=ErrorBody)))]
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
