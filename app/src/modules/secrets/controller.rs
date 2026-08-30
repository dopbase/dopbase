use super::{error::SecretError, model::*, service};
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

/// List secret metadata
///
/// Return the keys, versions, and timestamps of every secret in the
/// environment. Values are never included.
#[utoipa::path(
  get,
  path = "/api/v1/environments/{environment_id}/secrets",
  tag = "secrets",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("environment_id" = String, Path, description = "Environment id")),
  responses(
    (status = 200, description = "Secret metadata fetched", body = inline(HttpResponseFormat<Vec<SecretMetadata>>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Only administrators may list secrets", body = crate::http::ErrorBody),
    (status = 404, description = "The environment was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn list(
  State(state): State<AppState>,
  identity: AuthIdentity,
  Path(id): Path<String>,
) -> Result<HttpResponse<Vec<SecretMetadata>>, SecretError> {
  Ok(HttpResponse::ok(
    service::list(&state, &identity, &id).await?,
    "SECRETS_FETCHED",
  ))
}

/// Get secret metadata
///
/// Return the metadata of one secret. Values are never included.
#[utoipa::path(
  get,
  path = "/api/v1/environments/{environment_id}/secrets/{key}",
  tag = "secrets",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("environment_id" = String, Path, description = "Environment id"), ("key" = String, Path, description = "Secret key")),
  responses(
    (status = 200, description = "Secret metadata fetched", body = inline(HttpResponseFormat<SecretMetadata>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Only administrators may view secrets", body = crate::http::ErrorBody),
    (status = 404, description = "The secret or environment was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn get(
  State(state): State<AppState>,
  identity: AuthIdentity,
  Path((id, key)): Path<(String, String)>,
) -> Result<HttpResponse<SecretMetadata>, SecretError> {
  Ok(HttpResponse::ok(
    service::get(&state, &identity, &id, &key).await?,
    "SECRET_FETCHED",
  ))
}

/// Set a secret
///
/// Create the secret or store a new encrypted version of its value.
/// Requires the CSRF header for browser sessions.
#[utoipa::path(
  put,
  path = "/api/v1/environments/{environment_id}/secrets/{key}",
  tag = "secrets",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("environment_id" = String, Path, description = "Environment id"), ("key" = String, Path, description = "Secret key")),
  request_body = SetSecretRequest,
  responses(
    (status = 200, description = "Secret stored; the new metadata is returned", body = inline(HttpResponseFormat<SecretMetadata>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Administrator with a valid CSRF token is required", body = crate::http::ErrorBody),
    (status = 404, description = "The environment was not found", body = crate::http::ErrorBody),
    (status = 422, description = "The secret key or value is invalid", body = crate::http::ErrorBody),
  ),
)]
pub async fn set(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path((id, key)): Path<(String, String)>,
  axum::Json(request): axum::Json<SetSecretRequest>,
) -> Result<HttpResponse<SecretMetadata>, SecretError> {
  require_mutation(&identity, &headers)?;
  Ok(HttpResponse::ok(
    service::set(&state, &identity, &id, key, request).await?,
    "SECRET_SAVED",
  ))
}

/// Delete a secret
///
/// Remove the secret and all of its versions. Requires the CSRF header
/// for browser sessions.
#[utoipa::path(
  delete,
  path = "/api/v1/environments/{environment_id}/secrets/{key}",
  tag = "secrets",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("environment_id" = String, Path, description = "Environment id"), ("key" = String, Path, description = "Secret key")),
  responses(
    (status = 200, description = "Secret deleted", body = inline(HttpResponseFormat<serde_json::Value>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Administrator with a valid CSRF token is required", body = crate::http::ErrorBody),
    (status = 404, description = "The secret or environment was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn delete(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path((id, key)): Path<(String, String)>,
) -> Result<HttpResponse<serde_json::Value>, SecretError> {
  require_mutation(&identity, &headers)?;
  service::delete(&state, &identity, &id, &key).await?;
  Ok(HttpResponse::done("SECRET_DELETED"))
}

/// Reveal a secret value
///
/// Decrypt and return a secret value. Requires recent password
/// authentication and the CSRF header for browser sessions; the access is
/// recorded in the audit log.
#[utoipa::path(
  post,
  path = "/api/v1/environments/{environment_id}/secrets/{key}/reveal",
  tag = "secrets",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("environment_id" = String, Path, description = "Environment id"), ("key" = String, Path, description = "Secret key")),
  responses(
    (status = 200, description = "Secret value revealed", body = inline(HttpResponseFormat<RevealedSecret>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Recent password authentication and a valid CSRF token are required", body = crate::http::ErrorBody),
    (status = 404, description = "The secret or environment was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn reveal(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path((id, key)): Path<(String, String)>,
) -> Result<HttpResponse<RevealedSecret>, SecretError> {
  require_mutation(&identity, &headers)?;
  Ok(HttpResponse::ok(
    service::reveal(&state, &identity, &id, &key).await?,
    "SECRET_REVEALED",
  ))
}

/// Import secrets
///
/// Merge or replace a batch of secrets, optionally as a dry run that only
/// reports what would change. Requires the CSRF header for browser
/// sessions.
#[utoipa::path(
  post,
  path = "/api/v1/environments/{environment_id}/secrets/import",
  tag = "secrets",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("environment_id" = String, Path, description = "Environment id")),
  request_body = ImportSecretsRequest,
  responses(
    (status = 200, description = "Import applied (or previewed when dry run); added, updated, unchanged, and deleted keys are returned", body = inline(HttpResponseFormat<ImportSecretsResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Administrator with a valid CSRF token is required", body = crate::http::ErrorBody),
    (status = 404, description = "The environment was not found", body = crate::http::ErrorBody),
    (status = 422, description = "A secret key or value is invalid, a key is duplicated, or a limit is exceeded", body = crate::http::ErrorBody),
  ),
)]
pub async fn import(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path(id): Path<String>,
  axum::Json(request): axum::Json<ImportSecretsRequest>,
) -> Result<HttpResponse<ImportSecretsResponse>, SecretError> {
  require_mutation(&identity, &headers)?;
  Ok(HttpResponse::ok(
    service::import(&state, &identity, &id, request).await?,
    "SECRETS_IMPORTED",
  ))
}

/// Get the .env editor layout
///
/// Return the stored `.env` layout for the environment (comments,
/// ordering, blank `KEY=` slots). Contains no secret values, so no recent
/// authentication is required.
#[utoipa::path(
  get,
  path = "/api/v1/environments/{environment_id}/secrets/layout",
  tag = "secrets",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("environment_id" = String, Path, description = "Environment id")),
  responses(
    (status = 200, description = "Layout fetched", body = inline(HttpResponseFormat<EnvLayoutResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Only administrators may view the layout", body = crate::http::ErrorBody),
    (status = 404, description = "The environment was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn layout(
  State(state): State<AppState>,
  identity: AuthIdentity,
  Path(id): Path<String>,
) -> Result<HttpResponse<EnvLayoutResponse>, SecretError> {
  Ok(HttpResponse::ok(
    service::layout(&state, &identity, &id).await?,
    "SECRETS_LAYOUT_FETCHED",
  ))
}

/// Export all secrets
///
/// Decrypt and return every secret in the environment. Requires recent
/// password authentication and the CSRF header for browser sessions; the
/// export is recorded in the audit log.
#[utoipa::path(
  post,
  path = "/api/v1/environments/{environment_id}/secrets/export",
  tag = "secrets",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("environment_id" = String, Path, description = "Environment id")),
  responses(
    (status = 200, description = "Secrets exported", body = inline(HttpResponseFormat<ExportSecretsResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Recent password authentication and a valid CSRF token are required", body = crate::http::ErrorBody),
    (status = 404, description = "The environment was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn export(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path(id): Path<String>,
) -> Result<HttpResponse<ExportSecretsResponse>, SecretError> {
  require_mutation(&identity, &headers)?;
  Ok(HttpResponse::ok(
    service::export(&state, &identity, &id).await?,
    "SECRETS_EXPORTED",
  ))
}

/// Fetch runtime secrets
///
/// Return every decrypted secret of an environment for use by a runner.
/// Administrators may fetch any environment; runner tokens are restricted
/// to the environment their token belongs to. The access is recorded in
/// the audit log.
#[utoipa::path(
  get,
  path = "/api/v1/environments/{environment_id}/secrets/runtime",
  tag = "secrets",
  security(("bearerAuth" = [])),
  params(("environment_id" = String, Path, description = "Environment id")),
  responses(
    (status = 200, description = "Runtime secrets fetched", body = inline(HttpResponseFormat<RuntimeSecretsResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "The runner token cannot access this environment", body = crate::http::ErrorBody),
    (status = 404, description = "The environment was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn runtime(
  State(state): State<AppState>,
  identity: AuthIdentity,
  Path(id): Path<String>,
) -> Result<HttpResponse<RuntimeSecretsResponse>, SecretError> {
  Ok(HttpResponse::ok(
    service::runtime(&state, &identity, &id).await?,
    "RUNTIME_SECRETS_FETCHED",
  ))
}
