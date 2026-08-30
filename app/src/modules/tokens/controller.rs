use super::{error::TokenError, model::*, service};
use crate::{
  extractors::require_mutation,
  http::{HttpResponse, HttpResponseFormat},
  models::AuthIdentity,
  state::AppState,
};
use axum::{
  extract::{Path, State},
  http::HeaderMap,
};

/// List runner tokens
///
/// Return the runner tokens of the environment, including revocation
/// state. Plaintext tokens are never included.
#[utoipa::path(
  get,
  path = "/api/v1/environments/{environment_id}/tokens",
  tag = "tokens",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("environment_id" = String, Path, description = "Environment id")),
  responses(
    (status = 200, description = "Tokens fetched", body = inline(HttpResponseFormat<Vec<TokenMetadata>>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Only administrators may list tokens", body = crate::http::ErrorBody),
    (status = 404, description = "The environment was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn list(
  State(state): State<AppState>,
  identity: AuthIdentity,
  Path(id): Path<String>,
) -> Result<HttpResponse<Vec<TokenMetadata>>, TokenError> {
  Ok(HttpResponse::ok(
    service::list(&state, &identity, &id).await?,
    "TOKENS_FETCHED",
  ))
}

/// Create a runner token
///
/// Mint a new runner token for the environment. The plaintext token is
/// returned once and cannot be retrieved again. Only the `runner` role is
/// supported. Requires the CSRF header for browser sessions.
#[utoipa::path(
  post,
  path = "/api/v1/environments/{environment_id}/tokens",
  tag = "tokens",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("environment_id" = String, Path, description = "Environment id")),
  request_body = CreateTokenRequest,
  responses(
    (status = 201, description = "Token created; the plaintext token is returned once", body = inline(HttpResponseFormat<CreatedTokenResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Administrator with a valid CSRF token is required", body = crate::http::ErrorBody),
    (status = 404, description = "The environment was not found", body = crate::http::ErrorBody),
    (status = 409, description = "A token with this name already exists in the environment", body = crate::http::ErrorBody),
    (status = 422, description = "Token role or name is invalid", body = crate::http::ErrorBody),
  ),
)]
pub async fn create(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path(id): Path<String>,
  axum::Json(request): axum::Json<CreateTokenRequest>,
) -> Result<HttpResponse<CreatedTokenResponse>, TokenError> {
  require_mutation(&identity, &headers)?;
  Ok(HttpResponse::created(
    service::create(&state, &identity, &id, request).await?,
    "TOKEN_CREATED",
  ))
}

/// Revoke a runner token
///
/// Permanently revoke a runner token; the change cannot be undone.
/// Requires the CSRF header for browser sessions.
#[utoipa::path(
  post,
  path = "/api/v1/tokens/{token_id}/revoke",
  tag = "tokens",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("token_id" = String, Path, description = "Runner token id")),
  responses(
    (status = 200, description = "Token revoked", body = inline(HttpResponseFormat<TokenMetadata>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Administrator with a valid CSRF token is required", body = crate::http::ErrorBody),
    (status = 404, description = "The token was not found", body = crate::http::ErrorBody),
    (status = 409, description = "The token has already been revoked", body = crate::http::ErrorBody),
  ),
)]
pub async fn revoke(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path(id): Path<String>,
) -> Result<HttpResponse<TokenMetadata>, TokenError> {
  require_mutation(&identity, &headers)?;
  Ok(HttpResponse::ok(
    service::revoke(&state, &identity, &id).await?,
    "TOKEN_REVOKED",
  ))
}
