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

/// List personal tokens
///
/// List an account's personal tokens. Use `me` for your own. Another
/// account's tokens need users.view, unless you own that AI agent.
#[utoipa::path(
  get,
  path = crate::constants::api::users::TOKENS,
  tag = "personal-tokens",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("id" = String, Path, description = "User ID with usr_ prefix, or me")),
  responses(
    (status = 200, description = "Tokens fetched", body = inline(HttpResponseFormat<Vec<PersonalTokenResponse>>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Tokens can't list tokens, or users.view is required", body = crate::http::ErrorBody),
    (status = 404, description = "The user was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn list(
  State(state): State<AppState>,
  identity: AuthIdentity,
  Path(id): Path<String>,
) -> Result<HttpResponse<Vec<PersonalTokenResponse>>, crate::http::HttpError> {
  Ok(HttpResponse::ok(
    service::list(&state, &identity, &id).await?,
    "PERSONAL_TOKENS_FETCHED",
  ))
}

/// Create a personal token
///
/// Create a token for yourself (`me`), which needs tokens.personal, or for
/// an AI agent account you own or manage. A token reaches at most what its
/// account reaches, within its own scope and level, and always expires.
/// With `replaceExisting`, an AI agent's other valid tokens are revoked in
/// the same step. Browser sessions need recent authentication and the CSRF
/// header.
#[utoipa::path(
  post,
  path = crate::constants::api::users::TOKENS,
  tag = "personal-tokens",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("id" = String, Path, description = "User ID with usr_ prefix, or me")),
  request_body = CreatePersonalTokenRequest,
  responses(
    (status = 201, description = "Token created", body = inline(HttpResponseFormat<CreatedPersonalTokenResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "The caller can't create tokens for this account", body = crate::http::ErrorBody),
    (status = 404, description = "The user, project, or environment was not found", body = crate::http::ErrorBody),
    (status = 409, description = "A token with this name already exists", body = crate::http::ErrorBody),
    (status = 422, description = "The name, scope, level, or expiry is invalid, or replaceExisting was asked for a person's tokens", body = crate::http::ErrorBody),
  ),
)]
pub async fn create(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path(id): Path<String>,
  axum::Json(request): axum::Json<CreatePersonalTokenRequest>,
) -> Result<HttpResponse<CreatedPersonalTokenResponse>, crate::http::HttpError> {
  Ok(HttpResponse::created(
    service::create(&state, &identity, &headers, &id, request).await?,
    "PERSONAL_TOKEN_CREATED",
  ))
}

/// Revoke a personal token
///
/// Revoke your own token, a token of an AI agent you own, or anyone's with
/// tokens.revoke_others when their access is within yours.
#[utoipa::path(
  post,
  path = crate::constants::api::users::TOKEN_REVOKE,
  tag = "personal-tokens",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(
    ("id" = String, Path, description = "User ID with usr_ prefix, or me"),
    ("token_id" = String, Path, description = "Token ID"),
  ),
  responses(
    (status = 200, description = "Token revoked", body = inline(HttpResponseFormat<PersonalTokenResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "The caller can't revoke this token", body = crate::http::ErrorBody),
    (status = 404, description = "The token was not found", body = crate::http::ErrorBody),
    (status = 409, description = "The token was already revoked", body = crate::http::ErrorBody),
  ),
)]
pub async fn revoke(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path((id, token_id)): Path<(String, String)>,
) -> Result<HttpResponse<PersonalTokenResponse>, crate::http::HttpError> {
  Ok(HttpResponse::ok(
    service::revoke(&state, &identity, &headers, &id, &token_id).await?,
    "PERSONAL_TOKEN_REVOKED",
  ))
}
