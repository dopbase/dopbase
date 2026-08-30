use super::{error::EnvironmentError, model::*, service};
use crate::{
  extractors::require_mutation,
  http::{ErrorBody, HttpResponse, HttpResponseFormat},
  models::AuthIdentity,
  state::AppState,
};
use axum::{
  extract::{Path, Query, State},
  http::HeaderMap,
};
#[utoipa::path(get,path="/api/v1/environments",tag="environments",security(("bearerAuth"=[]),("cookieAuth"=[])),params(("project"=Option<String>,Query)),responses((status=200,body=inline(HttpResponseFormat<Vec<EnvironmentResponse>>)),(status=401,body=ErrorBody)))]
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
#[utoipa::path(get,path="/api/v1/environments/resolve",tag="environments",security(("bearerAuth"=[])),params(("reference"=String,Query)),responses((status=200,body=inline(HttpResponseFormat<EnvironmentResponse>)),(status=403,body=ErrorBody),(status=404,body=ErrorBody)))]
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
#[utoipa::path(post,path="/api/v1/projects/{project_ref}/environments",tag="environments",security(("bearerAuth"=[]),("cookieAuth"=[])),params(("project_ref"=String,Path)),request_body=CreateEnvironmentRequest,responses((status=201,body=inline(HttpResponseFormat<EnvironmentResponse>)),(status=409,body=ErrorBody),(status=422,body=ErrorBody)))]
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
#[utoipa::path(get,path="/api/v1/environments/{environment_id}",tag="environments",security(("bearerAuth"=[]),("cookieAuth"=[])),params(("environment_id"=String,Path)),responses((status=200,body=inline(HttpResponseFormat<EnvironmentResponse>)),(status=404,body=ErrorBody)))]
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
#[utoipa::path(patch,path="/api/v1/environments/{environment_id}",tag="environments",security(("bearerAuth"=[]),("cookieAuth"=[])),params(("environment_id"=String,Path)),request_body=RenameEnvironmentRequest,responses((status=200,body=inline(HttpResponseFormat<EnvironmentResponse>)),(status=404,body=ErrorBody),(status=409,body=ErrorBody),(status=422,body=ErrorBody)))]
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
#[utoipa::path(delete,path="/api/v1/environments/{environment_id}",tag="environments",security(("bearerAuth"=[]),("cookieAuth"=[])),params(("environment_id"=String,Path)),responses((status=200,body=inline(HttpResponseFormat<DeleteEnvironmentResponse>)),(status=404,body=ErrorBody)))]
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
