use super::{model::*, service};
use crate::{
  http::{HttpResponse, HttpResponseFormat},
  models::AuthIdentity,
  state::AppState,
};
use axum::{
  extract::{Path, State},
  http::HeaderMap,
};

/// List service accounts
///
/// Return every service account (AI agent). Requires an authenticated
/// administrator browser session.
#[utoipa::path(
  get,
  path = crate::constants::api::service_accounts::COLLECTION,
  tag = "service-accounts",
  security(("cookieAuth" = [])),
  responses(
    (status = 200, description = "Service accounts fetched", body = inline(HttpResponseFormat<Vec<ServiceAccountResponse>>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Administrator browser session is required", body = crate::http::ErrorBody),
  ),
)]
pub async fn list(
  State(s): State<AppState>,
  i: AuthIdentity,
) -> Result<HttpResponse<Vec<ServiceAccountResponse>>, crate::http::HttpError> {
  Ok(HttpResponse::ok(
    service::list(&s, &i).await?,
    "SERVICE_ACCOUNTS_FETCHED",
  ))
}

/// Create a service account
///
/// Add a new service account for AI agent integrations. Names must be
/// between 2 and 100 characters and unique. Requires an authenticated
/// administrator browser session with recent re-authentication and the CSRF header.
#[utoipa::path(
  post,
  path = crate::constants::api::service_accounts::COLLECTION,
  tag = "service-accounts",
  security(("cookieAuth" = [])),
  request_body = CreateServiceAccountRequest,
  responses(
    (status = 201, description = "Service account created", body = inline(HttpResponseFormat<ServiceAccountResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Administrator browser session with recent authentication and CSRF token is required", body = crate::http::ErrorBody),
    (status = 409, description = "A service account with this name already exists", body = crate::http::ErrorBody),
    (status = 422, description = "The service account name is invalid", body = crate::http::ErrorBody),
  ),
)]
pub async fn create(
  State(s): State<AppState>,
  h: HeaderMap,
  i: AuthIdentity,
  axum::Json(r): axum::Json<CreateServiceAccountRequest>,
) -> Result<HttpResponse<ServiceAccountResponse>, crate::http::HttpError> {
  Ok(HttpResponse::created(
    service::create(&s, &i, &h, r).await?,
    "SERVICE_ACCOUNT_CREATED",
  ))
}

/// Show a service account
///
/// Fetch a single service account by its ID. Requires an authenticated
/// administrator browser session.
#[utoipa::path(
  get,
  path = crate::constants::api::service_accounts::ITEM,
  tag = "service-accounts",
  security(("cookieAuth" = [])),
  params(("id" = String, Path, description = "Service account ID with aia_ prefix")),
  responses(
    (status = 200, description = "Service account fetched", body = inline(HttpResponseFormat<ServiceAccountResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Administrator browser session is required", body = crate::http::ErrorBody),
    (status = 404, description = "The service account was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn get(
  State(s): State<AppState>,
  i: AuthIdentity,
  Path(id): Path<String>,
) -> Result<HttpResponse<ServiceAccountResponse>, crate::http::HttpError> {
  Ok(HttpResponse::ok(
    service::get(&s, &i, &id).await?,
    "SERVICE_ACCOUNT_FETCHED",
  ))
}

/// Delete a service account
///
/// Permanently delete a service account and revoke its tokens. Requires
/// an authenticated administrator browser session with recent re-authentication
/// and the CSRF header.
#[utoipa::path(
  delete,
  path = crate::constants::api::service_accounts::ITEM,
  tag = "service-accounts",
  security(("cookieAuth" = [])),
  params(("id" = String, Path, description = "Service account ID with aia_ prefix")),
  responses(
    (status = 200, description = "Service account deleted", body = inline(HttpResponseFormat<serde_json::Value>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Administrator browser session with recent authentication and CSRF token is required", body = crate::http::ErrorBody),
    (status = 404, description = "The service account was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn delete(
  State(s): State<AppState>,
  h: HeaderMap,
  i: AuthIdentity,
  Path(id): Path<String>,
) -> Result<HttpResponse<serde_json::Value>, crate::http::HttpError> {
  service::delete(&s, &i, &h, &id).await?;
  Ok(HttpResponse::done("SERVICE_ACCOUNT_DELETED"))
}

/// List service account tokens
///
/// Return metadata for all active and revoked agent tokens belonging to
/// the service account. Requires an authenticated administrator browser session.
#[utoipa::path(
  get,
  path = crate::constants::api::service_accounts::TOKENS,
  tag = "service-accounts",
  security(("cookieAuth" = [])),
  params(("id" = String, Path, description = "Service account ID with aia_ prefix")),
  responses(
    (status = 200, description = "Agent tokens fetched", body = inline(HttpResponseFormat<Vec<AgentTokenResponse>>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Administrator browser session is required", body = crate::http::ErrorBody),
    (status = 404, description = "The service account was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn tokens(
  State(s): State<AppState>,
  i: AuthIdentity,
  Path(id): Path<String>,
) -> Result<HttpResponse<Vec<AgentTokenResponse>>, crate::http::HttpError> {
  Ok(HttpResponse::ok(
    service::tokens(&s, &i, &id).await?,
    "AGENT_TOKENS_FETCHED",
  ))
}

/// Create a service account token
///
/// Generate a new agent bearer token for the service account. The plaintext
/// token is returned only once in this response and cannot be retrieved again.
/// Requires an authenticated administrator browser session with recent
/// re-authentication and the CSRF header.
#[utoipa::path(
  post,
  path = crate::constants::api::service_accounts::TOKENS,
  tag = "service-accounts",
  security(("cookieAuth" = [])),
  params(("id" = String, Path, description = "Service account ID with aia_ prefix")),
  request_body = CreateAgentTokenRequest,
  responses(
    (status = 201, description = "Agent token created", body = inline(HttpResponseFormat<CreatedAgentTokenResponse>)),
    (status = 400, description = "Invalid expiration timestamp. It must be in the future and within 90 days", body = crate::http::ErrorBody),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Administrator browser session with recent authentication and CSRF token is required", body = crate::http::ErrorBody),
    (status = 404, description = "The service account was not found", body = crate::http::ErrorBody),
    (status = 409, description = "A token with this name already exists for this service account", body = crate::http::ErrorBody),
    (status = 422, description = "The token name is invalid", body = crate::http::ErrorBody),
  ),
)]
pub async fn create_token(
  State(s): State<AppState>,
  h: HeaderMap,
  i: AuthIdentity,
  Path(id): Path<String>,
  axum::Json(r): axum::Json<CreateAgentTokenRequest>,
) -> Result<HttpResponse<CreatedAgentTokenResponse>, crate::http::HttpError> {
  Ok(HttpResponse::created(
    service::create_token(&s, &i, &h, &id, r).await?,
    "AGENT_TOKEN_CREATED",
  ))
}

/// Revoke a service account token
///
/// Immediately revoke an active agent token. Revoked tokens cannot authenticate
/// future requests. Requires an authenticated administrator browser session with
/// recent re-authentication and the CSRF header.
#[utoipa::path(
  post,
  path = crate::constants::api::service_accounts::REVOKE_TOKEN,
  tag = "service-accounts",
  security(("cookieAuth" = [])),
  params(
    ("id" = String, Path, description = "Service account ID with aia_ prefix"),
    ("token_id" = String, Path, description = "Agent token ID with ait_ prefix")
  ),
  responses(
    (status = 200, description = "Agent token revoked", body = inline(HttpResponseFormat<AgentTokenResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Administrator browser session with recent authentication and CSRF token is required", body = crate::http::ErrorBody),
    (status = 404, description = "The active agent token or service account was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn revoke_token(
  State(s): State<AppState>,
  h: HeaderMap,
  i: AuthIdentity,
  Path((id, token_id)): Path<(String, String)>,
) -> Result<HttpResponse<AgentTokenResponse>, crate::http::HttpError> {
  Ok(HttpResponse::ok(
    service::revoke_token(&s, &i, &h, &id, &token_id).await?,
    "AGENT_TOKEN_REVOKED",
  ))
}
