use super::{error::ProjectError, model::*, service};
use crate::{
  extractors::require_mutation,
  http::{ErrorBody, HttpResponse, HttpResponseFormat},
  models::AuthIdentity,
  state::AppState,
};
use axum::{
  extract::{Path, State},
  http::HeaderMap,
};
#[utoipa::path(get,path="/api/v1/projects",tag="projects",security(("bearerAuth"=[]),("cookieAuth"=[])),responses((status=200,body=inline(HttpResponseFormat<Vec<ProjectResponse>>)),(status=401,body=ErrorBody)))]
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
#[utoipa::path(post,path="/api/v1/projects",tag="projects",security(("bearerAuth"=[]),("cookieAuth"=[])),request_body=CreateProjectRequest,responses((status=201,body=inline(HttpResponseFormat<ProjectResponse>)),(status=409,body=ErrorBody),(status=422,body=ErrorBody)))]
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
#[utoipa::path(post,path="/api/v1/projects/init",tag="projects",security(("bearerAuth"=[]),("cookieAuth"=[])),request_body=InitProjectRequest,responses((status=201,body=inline(HttpResponseFormat<InitProjectResponse>)),(status=409,body=ErrorBody),(status=422,body=ErrorBody)))]
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
#[utoipa::path(get,path="/api/v1/projects/{project_ref}",tag="projects",security(("bearerAuth"=[]),("cookieAuth"=[])),params(("project_ref"=String,Path)),responses((status=200,body=inline(HttpResponseFormat<ProjectResponse>)),(status=404,body=ErrorBody)))]
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
#[utoipa::path(patch,path="/api/v1/projects/{project_ref}",tag="projects",security(("bearerAuth"=[]),("cookieAuth"=[])),params(("project_ref"=String,Path)),request_body=RenameProjectRequest,responses((status=200,body=inline(HttpResponseFormat<ProjectResponse>)),(status=404,body=ErrorBody),(status=409,body=ErrorBody),(status=422,body=ErrorBody)))]
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
#[utoipa::path(delete,path="/api/v1/projects/{project_ref}",tag="projects",security(("bearerAuth"=[]),("cookieAuth"=[])),params(("project_ref"=String,Path)),responses((status=200,body=inline(HttpResponseFormat<DeleteProjectResponse>)),(status=404,body=ErrorBody)))]
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
