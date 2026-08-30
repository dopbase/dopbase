use super::{error::ProjectError, model::*, service};
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

/// List projects
///
/// Return every project. Administrator authentication is required.
#[utoipa::path(
  get,
  path = "/api/v1/projects",
  tag = "projects",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  responses(
    (status = 200, description = "Projects fetched", body = inline(HttpResponseFormat<Vec<ProjectResponse>>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Only administrators may list projects", body = crate::http::ErrorBody),
  ),
)]
pub async fn list(
  State(state): State<AppState>,
  identity: AuthIdentity,
) -> Result<HttpResponse<Vec<ProjectResponse>>, ProjectError> {
  crate::extractors::require_admin(&identity)?;
  Ok(HttpResponse::ok(
    service::list(&state).await?,
    "PROJECTS_FETCHED",
  ))
}

/// Create a project
///
/// Add a new project. Names must be lowercase slugs and unique.
/// Requires the CSRF header for browser sessions.
#[utoipa::path(
  post,
  path = "/api/v1/projects",
  tag = "projects",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  request_body = CreateProjectRequest,
  responses(
    (status = 201, description = "Project created", body = inline(HttpResponseFormat<ProjectResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Administrator with a valid CSRF token is required", body = crate::http::ErrorBody),
    (status = 409, description = "A project with this name already exists", body = crate::http::ErrorBody),
    (status = 422, description = "The project name is invalid", body = crate::http::ErrorBody),
  ),
)]
pub async fn create(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  axum::Json(request): axum::Json<CreateProjectRequest>,
) -> Result<HttpResponse<ProjectResponse>, ProjectError> {
  require_mutation(&identity, &headers)?;
  Ok(HttpResponse::created(
    service::create(&state, &identity, request).await?,
    "PROJECT_CREATED",
  ))
}

/// Initialize a project
///
/// Create a project, its first environment, and a batch of initial
/// secrets in one atomic step — useful for importing a `.env` file into a
/// fresh instance. Requires the CSRF header for browser sessions.
#[utoipa::path(
  post,
  path = "/api/v1/projects/init",
  tag = "projects",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  request_body = InitProjectRequest,
  responses(
    (status = 201, description = "Project, environment, and secrets created", body = inline(HttpResponseFormat<InitProjectResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Administrator with a valid CSRF token is required", body = crate::http::ErrorBody),
    (status = 409, description = "A project with this name already exists", body = crate::http::ErrorBody),
    (status = 422, description = "Names are invalid, the secret count or size exceeds the limit, or a secret key is duplicated", body = crate::http::ErrorBody),
  ),
)]
pub async fn init(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  axum::Json(request): axum::Json<InitProjectRequest>,
) -> Result<HttpResponse<InitProjectResponse>, ProjectError> {
  require_mutation(&identity, &headers)?;
  Ok(HttpResponse::created(
    service::init(&state, &identity, request).await?,
    "PROJECT_INITIALIZED",
  ))
}

/// Show a project
///
/// Fetch one project by id or name. Administrator authentication is
/// required.
#[utoipa::path(
  get,
  path = "/api/v1/projects/{project_ref}",
  tag = "projects",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("project_ref" = String, Path, description = "Project id or name")),
  responses(
    (status = 200, description = "Project fetched", body = inline(HttpResponseFormat<ProjectResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Only administrators may view projects", body = crate::http::ErrorBody),
    (status = 404, description = "The project was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn show(
  State(state): State<AppState>,
  identity: AuthIdentity,
  Path(reference): Path<String>,
) -> Result<HttpResponse<ProjectResponse>, ProjectError> {
  crate::extractors::require_admin(&identity)?;
  Ok(HttpResponse::ok(
    service::show(&state, &reference).await?,
    "PROJECT_FETCHED",
  ))
}

/// Rename a project
///
/// Change the name of a project. Names must be lowercase slugs and unique.
/// Requires the CSRF header for browser sessions.
#[utoipa::path(
  patch,
  path = "/api/v1/projects/{project_ref}",
  tag = "projects",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("project_ref" = String, Path, description = "Project id or name")),
  request_body = RenameProjectRequest,
  responses(
    (status = 200, description = "Project renamed", body = inline(HttpResponseFormat<ProjectResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Administrator with a valid CSRF token is required", body = crate::http::ErrorBody),
    (status = 404, description = "The project was not found", body = crate::http::ErrorBody),
    (status = 409, description = "A project with this name already exists", body = crate::http::ErrorBody),
    (status = 422, description = "The project name is invalid", body = crate::http::ErrorBody),
  ),
)]
pub async fn rename(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path(reference): Path<String>,
  axum::Json(request): axum::Json<RenameProjectRequest>,
) -> Result<HttpResponse<ProjectResponse>, ProjectError> {
  require_mutation(&identity, &headers)?;
  Ok(HttpResponse::ok(
    service::rename(&state, &identity, &reference, request).await?,
    "PROJECT_RENAMED",
  ))
}

/// Delete a project
///
/// Remove a project together with all of its environments, secrets, and
/// runner tokens. The response reports how many resources were affected.
/// Requires the CSRF header for browser sessions.
#[utoipa::path(
  delete,
  path = "/api/v1/projects/{project_ref}",
  tag = "projects",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("project_ref" = String, Path, description = "Project id or name")),
  responses(
    (status = 200, description = "Project deleted; affected resource counts are returned", body = inline(HttpResponseFormat<DeleteProjectResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Administrator with a valid CSRF token is required", body = crate::http::ErrorBody),
    (status = 404, description = "The project was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn delete(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path(reference): Path<String>,
) -> Result<HttpResponse<DeleteProjectResponse>, ProjectError> {
  require_mutation(&identity, &headers)?;
  Ok(HttpResponse::ok(
    service::delete(&state, &identity, &reference).await?,
    "PROJECT_DELETED",
  ))
}
