use super::{model::*, repository};
use crate::modules::common;
use crate::{
  constants::errors::{ENVIRONMENT_NAME_INVALID, TOKEN_SCOPE_INVALID},
  http::HttpError,
  models::{AffectedCounts, AuthIdentity},
  services::environment_id::{self, LAST_ENVIRONMENT_NUMBER},
  state::AppState,
};
use chrono::Utc;
use sqlx::{Sqlite, Transaction};

fn unique(error: sqlx::Error) -> HttpError {
  if error.to_string().contains("UNIQUE") {
    HttpError::conflict(
      "ENVIRONMENT_ALREADY_EXISTS",
      "An environment with this name already exists in the project.",
    )
  } else {
    HttpError::from(error)
  }
}

pub(crate) async fn insert_generated(
  tx: &mut Transaction<'_, Sqlite>,
  project_id: &str,
  name: &str,
  now: &str,
) -> Result<String, HttpError> {
  let mut number = repository::next_id_number(tx).await?;
  while number <= LAST_ENVIRONMENT_NUMBER {
    let id = environment_id::from_number(number);
    if repository::reserve_id(tx, &id).await? {
      repository::advance_id_number(tx, number + 1).await?;
      repository::insert(tx, &id, project_id, name, now)
        .await
        .map_err(unique)?;
      return Ok(id);
    }
    number += 1;
  }
  tracing::error!("environment id number space is exhausted");
  Err(HttpError::internal())
}

pub async fn list(
  state: &AppState,
  identity: &AuthIdentity,
  project: Option<&str>,
) -> Result<Vec<EnvironmentResponse>, HttpError> {
  crate::extractors::require_metadata_access(identity)?;
  Ok(repository::list(state.db.pool(), project).await?)
}
pub async fn show(
  state: &AppState,
  id: &str,
) -> Result<EnvironmentResponse, HttpError> {
  repository::find_id(state.db.pool(), id)
    .await?
    .ok_or_else(|| {
      HttpError::not_found(
        "ENVIRONMENT_NOT_FOUND",
        "The requested environment was not found.",
      )
    })
}
pub async fn resolve(
  state: &AppState,
  identity: &AuthIdentity,
  reference: &str,
) -> Result<EnvironmentResponse, HttpError> {
  let environment = repository::resolve(state.db.pool(), reference)
    .await?
    .ok_or_else(|| {
      HttpError::not_found(
        "ENVIRONMENT_NOT_FOUND",
        "The requested environment was not found.",
      )
    })?;
  if let AuthIdentity::Runner { environment_id, .. } = identity
    && *environment_id != environment.id
  {
    return Err(HttpError::forbidden(
      TOKEN_SCOPE_INVALID,
      "The runner token cannot access this environment.",
    ));
  }
  Ok(environment)
}
pub async fn create(
  state: &AppState,
  identity: &AuthIdentity,
  project_ref: &str,
  request: CreateEnvironmentRequest,
) -> Result<EnvironmentResponse, HttpError> {
  common::validate_slug(&request.name, ENVIRONMENT_NAME_INVALID, "Environment name")?;
  let (admin_id, email) = crate::extractors::require_project_manager(identity)?;
  let project = crate::modules::projects::service::show(state, project_ref).await?;
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin_with("BEGIN IMMEDIATE").await?;
  let id = insert_generated(&mut tx, &project.id, &request.name, &now).await?;
  common::audit(
    &mut *tx,
    "admin",
    Some(admin_id),
    Some(email),
    "environment.created",
    Some(&project.id),
    Some(&id),
    Some("environment"),
    Some(&id),
    serde_json::json!({"name":request.name}),
  )
  .await?;
  tx.commit().await?;
  show(state, &id).await
}
pub async fn rename(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
  request: RenameEnvironmentRequest,
) -> Result<EnvironmentResponse, HttpError> {
  common::validate_slug(&request.name, ENVIRONMENT_NAME_INVALID, "Environment name")?;
  let (admin_id, email) = crate::extractors::require_project_manager(identity)?;
  let environment = show(state, id).await?;
  let mut tx = state.db.pool().begin().await?;
  let updated = sqlx::query("UPDATE environments SET name=?,updated_at=? WHERE id=?")
    .bind(&request.name)
    .bind(Utc::now().to_rfc3339())
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(unique)?;
  if updated.rows_affected() != 1 {
    return Err(HttpError::not_found(
      "ENVIRONMENT_NOT_FOUND",
      "The requested environment was not found.",
    ));
  }
  common::audit(
    &mut *tx,
    "admin",
    Some(admin_id),
    Some(email),
    "environment.renamed",
    Some(&environment.project_id),
    Some(id),
    Some("environment"),
    Some(id),
    serde_json::json!({"oldName":environment.name,"newName":request.name}),
  )
  .await?;
  tx.commit().await?;
  show(state, id).await
}
pub async fn delete(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
) -> Result<DeleteEnvironmentResponse, HttpError> {
  let (admin_id, email) = crate::extractors::require_project_manager(identity)?;
  let environment = show(state, id).await?;
  let mut tx = state.db.pool().begin_with("BEGIN IMMEDIATE").await?;
  let secrets: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM secrets WHERE environment_id=?")
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
  let tokens: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM runner_tokens WHERE environment_id=?")
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
  let deleted = sqlx::query("DELETE FROM environments WHERE id=?")
    .bind(id)
    .execute(&mut *tx)
    .await?;
  if deleted.rows_affected() != 1 {
    return Err(HttpError::not_found(
      "ENVIRONMENT_NOT_FOUND",
      "The requested environment was not found.",
    ));
  }
  common::audit(
    &mut *tx,
    "admin",
    Some(admin_id),
    Some(email),
    "environment.deleted",
    Some(&environment.project_id),
    Some(id),
    Some("environment"),
    Some(id),
    serde_json::json!({"name":environment.name,"secrets":secrets,"tokens":tokens}),
  )
  .await?;
  tx.commit().await?;
  Ok(DeleteEnvironmentResponse {
    affected: AffectedCounts {
      projects: 0,
      environments: 1,
      secrets: secrets as u64,
      tokens: tokens as u64,
    },
  })
}
