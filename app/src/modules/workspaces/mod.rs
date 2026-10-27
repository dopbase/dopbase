use crate::{
  extractors::{require_mutation, require_project_manager, require_read_access},
  http::{HttpError, HttpResponse},
  models::AuthIdentity,
  modules::common,
  state::AppState,
};
use axum::{
  Json, Router,
  extract::{Path, State},
  http::HeaderMap,
  routing::get,
};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};

#[derive(Clone, Debug, Serialize, sqlx::FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
  pub id: String,
  pub name: String,
  pub root_path: String,
  pub created_at: String,
  pub updated_at: String,
}

pub const DEFAULT_WORKSPACE_ROOT_PATH: &str = "/workspace";

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceInput {
  pub name: String,
  pub root_path: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWorkspaceInput {
  pub name: String,
  pub root_path: String,
}

pub fn validate_root(value: &str) -> Result<(), HttpError> {
  let path = value.replace('\\', "/");
  let absolute = path.starts_with('/')
    || (path.as_bytes().get(1) == Some(&b':')
      && path.as_bytes().get(2) == Some(&b'/')
      && path.as_bytes()[0].is_ascii_alphabetic());
  if !absolute
    || path.len() > 4096
    || path.chars().any(char::is_control)
    || path.split('/').any(|part| part == ".." || part == ".")
  {
    return Err(HttpError::bad_request(
      "WORKSPACE_PATH_INVALID",
      "Use an absolute root path without '.' or '..' components.",
    ));
  }
  Ok(())
}

pub fn validate_relative(value: &str) -> Result<(), HttpError> {
  if value == "." {
    return Ok(());
  }
  let path = value.replace('\\', "/");
  if path.is_empty()
    || path.len() > 4096
    || path.starts_with('/')
    || path.contains(':')
    || path.chars().any(char::is_control)
    || path
      .split('/')
      .any(|part| part.is_empty() || part == ".." || part == ".")
  {
    return Err(HttpError::bad_request(
      "PROJECT_PATH_INVALID",
      "Use a relative project directory inside the workspace, or '.' for its root.",
    ));
  }
  Ok(())
}

fn database_error(error: sqlx::Error) -> HttpError {
  if error.to_string().contains("UNIQUE") {
    HttpError::conflict(
      "WORKSPACE_ALREADY_EXISTS",
      "A workspace with this name already exists.",
    )
  } else {
    error.into()
  }
}

#[utoipa::path(get, path=crate::constants::api::workspaces::COLLECTION, tag="workspaces", responses((status=200, description="Workspaces fetched")), security(("bearerAuth"=[]), ("cookieAuth"=[])))]
pub async fn list(
  State(state): State<AppState>,
  identity: AuthIdentity,
) -> Result<HttpResponse<Vec<Workspace>>, HttpError> {
  require_read_access(&identity)?;
  let rows = sqlx::query_as("SELECT * FROM workspaces ORDER BY name")
    .fetch_all(state.db.pool())
    .await?;
  Ok(HttpResponse::ok(rows, "WORKSPACES_FETCHED"))
}

#[utoipa::path(post, path=crate::constants::api::workspaces::COLLECTION, tag="workspaces", request_body=CreateWorkspaceInput, responses((status=201, description="Workspace created")), security(("bearerAuth"=[]), ("cookieAuth"=[])))]
pub async fn create(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Json(input): Json<CreateWorkspaceInput>,
) -> Result<HttpResponse<Workspace>, HttpError> {
  require_mutation(&identity, &headers)?;
  let (admin_id, email) = require_project_manager(&identity)?;
  common::validate_slug(&input.name, "WORKSPACE_NAME_INVALID", "Workspace name")?;
  let root_path = input
    .root_path
    .as_deref()
    .unwrap_or(DEFAULT_WORKSPACE_ROOT_PATH);
  validate_root(root_path)?;
  let id = crate::services::token::public_id("wsp_");
  let now = chrono::Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  let row = sqlx::query_as(
    "INSERT INTO workspaces(id,name,root_path,created_at,updated_at) VALUES(?,?,?,?,?) RETURNING *",
  )
  .bind(&id)
  .bind(&input.name)
  .bind(root_path)
  .bind(&now)
  .bind(&now)
  .fetch_one(&mut *tx)
  .await
  .map_err(database_error)?;
  common::audit(
    &mut *tx,
    "admin",
    Some(admin_id),
    Some(email),
    "workspace.created",
    None,
    None,
    Some("workspace"),
    Some(&id),
    serde_json::json!({"name":input.name}),
  )
  .await?;
  tx.commit().await?;
  Ok(HttpResponse::created(row, "WORKSPACE_CREATED"))
}

#[utoipa::path(patch, path=crate::constants::api::workspaces::ITEM, tag="workspaces", request_body=UpdateWorkspaceInput, params(("id"=String, Path)), responses((status=200, description="Workspace updated")), security(("bearerAuth"=[]), ("cookieAuth"=[])))]
pub async fn update(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path(id): Path<String>,
  Json(input): Json<UpdateWorkspaceInput>,
) -> Result<HttpResponse<Workspace>, HttpError> {
  require_mutation(&identity, &headers)?;
  let (admin_id, email) = require_project_manager(&identity)?;
  common::validate_slug(&input.name, "WORKSPACE_NAME_INVALID", "Workspace name")?;
  validate_root(&input.root_path)?;
  let mut tx = state.db.pool().begin().await?;
  let row =
    sqlx::query_as("UPDATE workspaces SET name=?,root_path=?,updated_at=? WHERE id=? RETURNING *")
      .bind(&input.name)
      .bind(&input.root_path)
      .bind(chrono::Utc::now().to_rfc3339())
      .bind(&id)
      .fetch_optional(&mut *tx)
      .await
      .map_err(database_error)?
      .ok_or_else(|| HttpError::not_found("WORKSPACE_NOT_FOUND", "Workspace not found."))?;
  common::audit(
    &mut *tx,
    "admin",
    Some(admin_id),
    Some(email),
    "workspace.updated",
    None,
    None,
    Some("workspace"),
    Some(&id),
    serde_json::json!({"name":input.name}),
  )
  .await?;
  tx.commit().await?;
  Ok(HttpResponse::ok(row, "WORKSPACE_UPDATED"))
}

#[utoipa::path(delete, path=crate::constants::api::workspaces::ITEM, tag="workspaces", params(("id"=String, Path)), responses((status=200, description="Empty workspace deleted")), security(("bearerAuth"=[]), ("cookieAuth"=[])))]
pub async fn delete(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path(id): Path<String>,
) -> Result<HttpResponse<serde_json::Value>, HttpError> {
  require_mutation(&identity, &headers)?;
  let (admin_id, email) = require_project_manager(&identity)?;
  let mut tx = state.db.pool().begin_with("BEGIN IMMEDIATE").await?;
  let count: i64 =
    sqlx::query_scalar("SELECT COUNT(*) FROM project_locations WHERE workspace_id=?")
      .bind(&id)
      .fetch_one(&mut *tx)
      .await?;
  if count > 0 {
    return Err(HttpError::conflict(
      "WORKSPACE_NOT_EMPTY",
      "Move the workspace's projects before deleting it.",
    ));
  }
  if sqlx::query("DELETE FROM workspaces WHERE id=?")
    .bind(&id)
    .execute(&mut *tx)
    .await?
    .rows_affected()
    != 1
  {
    return Err(HttpError::not_found(
      "WORKSPACE_NOT_FOUND",
      "Workspace not found.",
    ));
  }
  common::audit(
    &mut *tx,
    "admin",
    Some(admin_id),
    Some(email),
    "workspace.deleted",
    None,
    None,
    Some("workspace"),
    Some(&id),
    serde_json::json!({}),
  )
  .await?;
  tx.commit().await?;
  Ok(HttpResponse::ok(serde_json::json!({}), "WORKSPACE_DELETED"))
}

pub fn routes() -> Router<AppState> {
  Router::new()
    .route(
      crate::constants::api::workspaces::COLLECTION,
      get(list).post(create),
    )
    .route(
      crate::constants::api::workspaces::ITEM,
      axum::routing::patch(update).delete(delete),
    )
}

#[derive(OpenApi)]
#[openapi(paths(list,create,update,delete), components(schemas(Workspace,CreateWorkspaceInput,UpdateWorkspaceInput)), tags((name="workspaces")))]
struct WorkspaceApi;
pub fn openapi() -> utoipa::openapi::OpenApi {
  WorkspaceApi::openapi()
}
