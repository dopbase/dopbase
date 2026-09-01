use super::{model::*, repository};
use crate::modules::common;
use crate::{
  constants::{
    errors::{
      ENVIRONMENT_NAME_INVALID, IMPORT_DUPLICATE_KEY, SECRET_COUNT_LIMIT_MESSAGE,
      SECRET_LIMIT_EXCEEDED, SECRET_TOTAL_SIZE_LIMIT_MESSAGE,
    },
    limits::{MAX_SECRET_COLLECTION_BYTES, MAX_SECRETS_PER_ENVIRONMENT},
    tokens::{ENVIRONMENT_ID_PREFIX, PROJECT_ID_PREFIX},
  },
  http::HttpError,
  models::{AffectedCounts, AuthIdentity},
  services::token,
  state::AppState,
};
use chrono::Utc;
use std::collections::HashSet;
fn map_unique(error: sqlx::Error) -> HttpError {
  if error.to_string().contains("UNIQUE") {
    HttpError::conflict(
      "PROJECT_ALREADY_EXISTS",
      "A project with this name already exists.",
    )
  } else {
    HttpError::from(error)
  }
}
pub async fn list(state: &AppState) -> Result<Vec<ProjectResponse>, HttpError> {
  Ok(repository::list(state.db.pool()).await?)
}
pub async fn show(
  state: &AppState,
  reference: &str,
) -> Result<ProjectResponse, HttpError> {
  repository::find(state.db.pool(), reference)
    .await?
    .ok_or_else(|| {
      HttpError::not_found("PROJECT_NOT_FOUND", "The requested project was not found.")
    })
}
pub async fn create(
  state: &AppState,
  identity: &AuthIdentity,
  request: CreateProjectRequest,
) -> Result<ProjectResponse, HttpError> {
  common::validate_slug(&request.name, "PROJECT_NAME_INVALID", "Project name")?;
  let (admin_id, email) = crate::extractors::require_admin(identity)?;
  let id = token::public_id(PROJECT_ID_PREFIX);
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  let project = repository::insert(&mut tx, &id, &request.name, &now)
    .await
    .map_err(map_unique)?;
  common::audit(
    &mut *tx,
    "admin",
    Some(admin_id),
    Some(email),
    "project.created",
    Some(&id),
    None,
    Some("project"),
    Some(&id),
    serde_json::json!({"name":request.name}),
  )
  .await?;
  tx.commit().await?;
  Ok(project)
}
pub async fn rename(
  state: &AppState,
  identity: &AuthIdentity,
  reference: &str,
  request: RenameProjectRequest,
) -> Result<ProjectResponse, HttpError> {
  common::validate_slug(&request.name, "PROJECT_NAME_INVALID", "Project name")?;
  let (admin_id, email) = crate::extractors::require_admin(identity)?;
  let project = show(state, reference).await?;
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  let updated = sqlx::query("UPDATE projects SET name=?,updated_at=? WHERE id=?")
    .bind(&request.name)
    .bind(&now)
    .bind(&project.id)
    .execute(&mut *tx)
    .await
    .map_err(map_unique)?;
  if updated.rows_affected() != 1 {
    return Err(HttpError::not_found(
      "PROJECT_NOT_FOUND",
      "The requested project was not found.",
    ));
  }
  common::audit(
    &mut *tx,
    "admin",
    Some(admin_id),
    Some(email),
    "project.renamed",
    Some(&project.id),
    None,
    Some("project"),
    Some(&project.id),
    serde_json::json!({"oldName":project.name,"newName":request.name}),
  )
  .await?;
  tx.commit().await?;
  show(state, &project.id).await
}
pub async fn delete(
  state: &AppState,
  identity: &AuthIdentity,
  reference: &str,
) -> Result<DeleteProjectResponse, HttpError> {
  let (admin_id, email) = crate::extractors::require_admin(identity)?;
  let project = show(state, reference).await?;
  let mut tx = state.db.pool().begin_with("BEGIN IMMEDIATE").await?;
  let environments: i64 =
    sqlx::query_scalar("SELECT COUNT(*) FROM environments WHERE project_id=?")
      .bind(&project.id)
      .fetch_one(&mut *tx)
      .await?;
  let secrets:i64=sqlx::query_scalar("SELECT COUNT(*) FROM secrets WHERE environment_id IN(SELECT id FROM environments WHERE project_id=?)").bind(&project.id).fetch_one(&mut *tx).await?;
  let tokens:i64=sqlx::query_scalar("SELECT COUNT(*) FROM runner_tokens WHERE environment_id IN(SELECT id FROM environments WHERE project_id=?)").bind(&project.id).fetch_one(&mut *tx).await?;
  let deleted = sqlx::query("DELETE FROM projects WHERE id=?")
    .bind(&project.id)
    .execute(&mut *tx)
    .await?;
  if deleted.rows_affected() != 1 {
    return Err(HttpError::not_found(
      "PROJECT_NOT_FOUND",
      "The requested project was not found.",
    ));
  }
  common::audit(&mut *tx,"admin",Some(admin_id),Some(email),"project.deleted",Some(&project.id),None,Some("project"),Some(&project.id),serde_json::json!({"name":project.name,"environments":environments,"secrets":secrets,"tokens":tokens})).await?;
  tx.commit().await?;
  Ok(DeleteProjectResponse {
    affected: AffectedCounts {
      projects: 1,
      environments: environments as u64,
      secrets: secrets as u64,
      tokens: tokens as u64,
    },
  })
}
pub async fn init(
  state: &AppState,
  identity: &AuthIdentity,
  request: InitProjectRequest,
) -> Result<InitProjectResponse, HttpError> {
  common::validate_slug(
    &request.project_name,
    "PROJECT_NAME_INVALID",
    "Project name",
  )?;
  common::validate_slug(
    &request.environment_name,
    ENVIRONMENT_NAME_INVALID,
    "Environment name",
  )?;
  if request.entries.len() > MAX_SECRETS_PER_ENVIRONMENT {
    return Err(HttpError::new(
      axum::http::StatusCode::UNPROCESSABLE_ENTITY,
      SECRET_LIMIT_EXCEEDED,
      SECRET_COUNT_LIMIT_MESSAGE,
    ));
  }
  let mut keys = HashSet::new();
  let total: usize = request.entries.iter().map(|entry| entry.value.len()).sum();
  if total > MAX_SECRET_COLLECTION_BYTES {
    return Err(HttpError::new(
      axum::http::StatusCode::UNPROCESSABLE_ENTITY,
      SECRET_LIMIT_EXCEEDED,
      SECRET_TOTAL_SIZE_LIMIT_MESSAGE,
    ));
  }
  for entry in &request.entries {
    crate::modules::secrets::service::validate_entry(entry)?;
    if !keys.insert(entry.key.clone()) {
      return Err(HttpError::new(
        axum::http::StatusCode::UNPROCESSABLE_ENTITY,
        IMPORT_DUPLICATE_KEY,
        "The import contains a duplicate secret key.",
      ));
    }
  }
  let (admin_id, email) = crate::extractors::require_admin(identity)?;
  let project_id = token::public_id(PROJECT_ID_PREFIX);
  let environment_id = token::public_id(ENVIRONMENT_ID_PREFIX);
  let now = Utc::now().to_rfc3339();
  let encrypted = request
    .entries
    .iter()
    .map(|entry| {
      state
        .crypto
        .encrypt(entry.value.as_bytes(), &environment_id, &entry.key, 1)
        .map(|value| (entry, value))
    })
    .collect::<Result<Vec<_>, _>>()
    .map_err(|_| HttpError::internal())?;
  let mut tx = state.db.pool().begin().await?;
  let project = repository::insert(&mut tx, &project_id, &request.project_name, &now)
    .await
    .map_err(map_unique)?;
  sqlx::query(
    "INSERT INTO environments(id,project_id,name,created_at,updated_at)VALUES(?,?,?,?,?)",
  )
  .bind(&environment_id)
  .bind(&project_id)
  .bind(&request.environment_name)
  .bind(&now)
  .bind(&now)
  .execute(&mut *tx)
  .await?;
  for (entry, value) in encrypted {
    sqlx::query("INSERT INTO secrets(environment_id,key,version,ciphertext,value_nonce,wrapped_key,key_nonce,created_at,updated_at)VALUES(?,?,1,?,?,?,?,?,?)").bind(&environment_id).bind(&entry.key).bind(value.ciphertext).bind(value.value_nonce).bind(value.wrapped_key).bind(value.key_nonce).bind(&now).bind(&now).execute(&mut *tx).await?;
  }
  common::audit(
    &mut *tx,
    "admin",
    Some(admin_id),
    Some(email),
    "project.initialized",
    Some(&project_id),
    Some(&environment_id),
    Some("project"),
    Some(&project_id),
    serde_json::json!({"secretCount":request.entries.len()}),
  )
  .await?;
  tx.commit().await?;
  Ok(InitProjectResponse {
    project,
    environment_id,
    secret_count: request.entries.len(),
  })
}
