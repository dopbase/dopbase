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

#[utoipa::path(patch, path=crate::constants::api::projects::LOCATION, tag="projects", request_body=ProjectLocationRequest, params(("project_ref"=String, Path)), responses((status=200, description="Project directory updated")), security(("bearerAuth"=[]), ("cookieAuth"=[])))]
pub async fn location(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path(reference): Path<String>,
  axum::Json(request): axum::Json<ProjectLocationRequest>,
) -> Result<HttpResponse<ProjectResponse>, ProjectError> {
  require_mutation(&identity, &headers)?;
  let (admin_id, email) = crate::extractors::require_project_manager(&identity)?;
  let project = service::show(&state, &reference).await?;
  let mut tx = state
    .db
    .pool()
    .begin_with("BEGIN IMMEDIATE")
    .await
    .map_err(crate::http::HttpError::from)?;
  if let Some(workspace_id) = &request.workspace_id {
    let path = request.relative_path.as_deref().unwrap_or(".");
    crate::modules::workspaces::validate_relative(path)?;
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workspaces WHERE id=?)")
      .bind(workspace_id)
      .fetch_one(&mut *tx)
      .await
      .map_err(crate::http::HttpError::from)?;
    if !exists {
      return Err(
        crate::http::HttpError::not_found("WORKSPACE_NOT_FOUND", "Workspace not found.").into(),
      );
    }
    if crate::modules::projects::repository::name_taken_in_workspace(
      state.db.pool(),
      workspace_id,
      &project.name,
      Some(&project.id),
    )
    .await
    .map_err(crate::http::HttpError::from)?
    {
      return Err(
        crate::http::HttpError::conflict(
          "PROJECT_ALREADY_EXISTS",
          "A project with this name already exists in the workspace.",
        )
        .into(),
      );
    }
    sqlx::query("INSERT INTO project_locations(project_id,workspace_id,relative_path) VALUES(?,?,?) ON CONFLICT(project_id) DO UPDATE SET workspace_id=excluded.workspace_id,relative_path=excluded.relative_path").bind(&project.id).bind(workspace_id).bind(path.replace('\\', "/")).execute(&mut *tx).await.map_err(|error| {
      if error.to_string().contains("UNIQUE") { crate::http::HttpError::conflict("PROJECT_PATH_IN_USE", "This workspace directory is already assigned to a project.") } else { error.into() }
    })?;
  } else {
    sqlx::query("DELETE FROM project_locations WHERE project_id=?")
      .bind(&project.id)
      .execute(&mut *tx)
      .await
      .map_err(crate::http::HttpError::from)?;
  }
  crate::modules::common::audit(
    &mut *tx,
    "admin",
    Some(admin_id),
    Some(email),
    "project.location_updated",
    Some(&project.id),
    None,
    Some("project"),
    Some(&project.id),
    serde_json::json!({"workspaceId":request.workspace_id,"relativePath":request.relative_path}),
  )
  .await?;
  tx.commit().await.map_err(crate::http::HttpError::from)?;
  Ok(HttpResponse::ok(
    service::show(&state, &project.id).await?,
    "PROJECT_LOCATION_UPDATED",
  ))
}

/// List projects
///
/// Return every project. Administrator authentication is required.
#[utoipa::path(
  get,
  path = crate::constants::api::projects::COLLECTION,
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
  crate::extractors::require_read_access(&identity)?;
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
  path = crate::constants::api::projects::COLLECTION,
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
  path = crate::constants::api::projects::INIT,
  tag = "projects",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  request_body = InitProjectRequest,
  responses(
    (status = 201, description = "Project, environment, and secrets created", body = inline(HttpResponseFormat<InitProjectResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Administrator with a valid CSRF token is required", body = crate::http::ErrorBody),
    (status = 409, description = "A project with this name already exists", body = crate::http::ErrorBody),
    (status = 422, description = "Names are invalid, the secret count or size exceeds the limit, or a secret key is duplicated", body = crate::http::ErrorBody),
    (status = 500, description = "The environment ID could not be generated", body = crate::http::ErrorBody),
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
  path = crate::constants::api::projects::ITEM,
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
  crate::extractors::require_read_access(&identity)?;
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
  path = crate::constants::api::projects::ITEM,
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
  path = crate::constants::api::projects::ITEM,
  tag = "projects",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("project_ref" = String, Path, description = "Project id or name")),
  responses(
    (status = 200, description = "Project deleted. Affected resource counts are returned", body = inline(HttpResponseFormat<DeleteProjectResponse>)),
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
