use super::{error::EnvironmentError, model::*, service};
use crate::{
  extractors::require_mutation,
  http::{HttpResponse, HttpResponseFormat},
  models::AuthIdentity,
  state::AppState,
};
use axum::{
  extract::{Path, Query, State},
  http::HeaderMap,
};

/// List environments
///
/// Return every environment, optionally filtered to one project via the
/// `project` query parameter. Administrator authentication is required.
#[utoipa::path(
  get,
  path = "/api/v1/environments",
  tag = "environments",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("project" = Option<String>, Query, description = "Filter environments by project reference")),
  responses(
    (status = 200, description = "Environments fetched", body = inline(HttpResponseFormat<Vec<EnvironmentResponse>>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Only administrators may list environments", body = crate::http::ErrorBody),
  ),
)]
pub async fn list(
  State(state): State<AppState>,
  identity: AuthIdentity,
  Query(query): Query<EnvironmentListQuery>,
) -> Result<HttpResponse<Vec<EnvironmentResponse>>, EnvironmentError> {
  Ok(HttpResponse::ok(
    service::list(&state, &identity, query.project.as_deref()).await?,
    "ENVIRONMENTS_FETCHED",
  ))
}

/// Resolve an environment reference
///
/// Look up an environment by id or name and return it. Runner tokens are
/// restricted to the environment they belong to.
#[utoipa::path(
  get,
  path = "/api/v1/environments/resolve",
  tag = "environments",
  security(("bearerAuth" = [])),
  params(("reference" = String, Query, description = "Environment id or project/name reference")),
  responses(
    (status = 200, description = "Environment resolved", body = inline(HttpResponseFormat<EnvironmentResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "The runner token cannot access this environment", body = crate::http::ErrorBody),
    (status = 404, description = "No environment matches the reference", body = crate::http::ErrorBody),
  ),
)]
pub async fn resolve(
  State(state): State<AppState>,
  identity: AuthIdentity,
  Query(query): Query<ResolveEnvironmentQuery>,
) -> Result<HttpResponse<EnvironmentResponse>, EnvironmentError> {
  Ok(HttpResponse::ok(
    service::resolve(&state, &identity, &query.reference).await?,
    "ENVIRONMENT_RESOLVED",
  ))
}

/// Create an environment
///
/// Add a new environment to a project. Names must be lowercase slugs.
/// Requires the CSRF header for browser sessions.
#[utoipa::path(
  post,
  path = "/api/v1/projects/{project_ref}/environments",
  tag = "environments",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("project_ref" = String, Path, description = "Project id or name")),
  request_body = CreateEnvironmentRequest,
  responses(
    (status = 201, description = "Environment created", body = inline(HttpResponseFormat<EnvironmentResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Administrator with a valid CSRF token is required", body = crate::http::ErrorBody),
    (status = 404, description = "The project was not found", body = crate::http::ErrorBody),
    (status = 409, description = "An environment with this name already exists in the project", body = crate::http::ErrorBody),
    (status = 422, description = "The environment name is invalid", body = crate::http::ErrorBody),
  ),
)]
pub async fn create(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path(project_ref): Path<String>,
  axum::Json(request): axum::Json<CreateEnvironmentRequest>,
) -> Result<HttpResponse<EnvironmentResponse>, EnvironmentError> {
  require_mutation(&identity, &headers)?;
  Ok(HttpResponse::created(
    service::create(&state, &identity, &project_ref, request).await?,
    "ENVIRONMENT_CREATED",
  ))
}

/// Show an environment
///
/// Fetch one environment by id. Administrator authentication is required.
#[utoipa::path(
  get,
  path = "/api/v1/environments/{environment_id}",
  tag = "environments",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("environment_id" = String, Path, description = "Environment id")),
  responses(
    (status = 200, description = "Environment fetched", body = inline(HttpResponseFormat<EnvironmentResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Only administrators may view environments", body = crate::http::ErrorBody),
    (status = 404, description = "The environment was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn show(
  State(state): State<AppState>,
  identity: AuthIdentity,
  Path(id): Path<String>,
) -> Result<HttpResponse<EnvironmentResponse>, EnvironmentError> {
  crate::extractors::require_admin(&identity)?;
  Ok(HttpResponse::ok(
    service::show(&state, &id).await?,
    "ENVIRONMENT_FETCHED",
  ))
}

/// Rename an environment
///
/// Change the name of an environment. Names must be lowercase slugs and
/// unique within the project. Requires the CSRF header for browser
/// sessions.
#[utoipa::path(
  patch,
  path = "/api/v1/environments/{environment_id}",
  tag = "environments",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("environment_id" = String, Path, description = "Environment id")),
  request_body = RenameEnvironmentRequest,
  responses(
    (status = 200, description = "Environment renamed", body = inline(HttpResponseFormat<EnvironmentResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Administrator with a valid CSRF token is required", body = crate::http::ErrorBody),
    (status = 404, description = "The environment was not found", body = crate::http::ErrorBody),
    (status = 409, description = "An environment with this name already exists in the project", body = crate::http::ErrorBody),
    (status = 422, description = "The environment name is invalid", body = crate::http::ErrorBody),
  ),
)]
pub async fn rename(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path(id): Path<String>,
  axum::Json(request): axum::Json<RenameEnvironmentRequest>,
) -> Result<HttpResponse<EnvironmentResponse>, EnvironmentError> {
  require_mutation(&identity, &headers)?;
  Ok(HttpResponse::ok(
    service::rename(&state, &identity, &id, request).await?,
    "ENVIRONMENT_RENAMED",
  ))
}

/// Delete an environment
///
/// Remove an environment together with all of its secrets and runner
/// tokens. The response reports how many resources were affected.
/// Requires the CSRF header for browser sessions.
#[utoipa::path(
  delete,
  path = "/api/v1/environments/{environment_id}",
  tag = "environments",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("environment_id" = String, Path, description = "Environment id")),
  responses(
    (status = 200, description = "Environment deleted; affected resource counts are returned", body = inline(HttpResponseFormat<DeleteEnvironmentResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Administrator with a valid CSRF token is required", body = crate::http::ErrorBody),
    (status = 404, description = "The environment was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn delete(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path(id): Path<String>,
) -> Result<HttpResponse<DeleteEnvironmentResponse>, EnvironmentError> {
  require_mutation(&identity, &headers)?;
  Ok(HttpResponse::ok(
    service::delete(&state, &identity, &id).await?,
    "ENVIRONMENT_DELETED",
  ))
}
