use super::{error::AuthError, model::*, service};
use crate::{
  extractors::require_mutation,
  http::{ErrorBody, HttpResponse, HttpResponseFormat},
  models::AuthIdentity,
  state::AppState,
};
use axum::{
  extract::State,
  http::{HeaderMap, HeaderName, HeaderValue},
};
#[utoipa::path(post,path="/api/v1/auth/login",tag="authentication",request_body=LoginRequest,responses((status=200,body=inline(HttpResponseFormat<LoginResponse>)),(status=401,body=ErrorBody),(status=422,body=ErrorBody),(status=429,body=ErrorBody)))]
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
#[utoipa::path(post,path="/api/v1/auth/logout",tag="authentication",security(("bearerAuth"=[]),("cookieAuth"=[])),responses((status=200,body=inline(HttpResponseFormat<serde_json::Value>)),(status=401,body=ErrorBody),(status=403,body=ErrorBody)))]
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
#[utoipa::path(get,path="/api/v1/auth/session",tag="authentication",security(("bearerAuth"=[]),("cookieAuth"=[])),responses((status=200,body=inline(HttpResponseFormat<SessionResponse>)),(status=401,body=ErrorBody)))]
pub async fn session(identity: AuthIdentity) -> Result<HttpResponse<SessionResponse>, AuthError> {
  Ok(HttpResponse::ok(
    service::session(&identity)?,
    "SESSION_FETCHED",
  ))
}
#[utoipa::path(post,path="/api/v1/auth/reauthenticate",tag="authentication",security(("bearerAuth"=[]),("cookieAuth"=[])),request_body=ReauthenticateRequest,responses((status=200,body=inline(HttpResponseFormat<serde_json::Value>)),(status=401,body=ErrorBody),(status=403,body=ErrorBody)))]
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
#[utoipa::path(post,path="/api/v1/auth/change-password",tag="authentication",security(("bearerAuth"=[]),("cookieAuth"=[])),request_body=ChangePasswordRequest,responses((status=200,body=inline(HttpResponseFormat<serde_json::Value>)),(status=401,body=ErrorBody),(status=403,body=ErrorBody),(status=422,body=ErrorBody)))]
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
