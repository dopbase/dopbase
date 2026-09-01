use super::{model::*, repository};
use crate::modules::common;
use crate::{
  constants::{
    errors::{ENVIRONMENT_NAME_INVALID, TOKEN_SCOPE_INVALID},
    tokens::ENVIRONMENT_ID_PREFIX,
  },
  http::HttpError,
  models::{AffectedCounts, AuthIdentity},
  services::token,
  state::AppState,
};
use chrono::Utc;
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
pub async fn list(
  state: &AppState,
  identity: &AuthIdentity,
  project: Option<&str>,
) -> Result<Vec<EnvironmentResponse>, HttpError> {
  crate::extractors::require_admin(identity)?;
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
  let (admin_id, email) = crate::extractors::require_admin(identity)?;
  let project = crate::modules::projects::service::show(state, project_ref).await?;
  let id = token::public_id(ENVIRONMENT_ID_PREFIX);
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  sqlx::query(
    "INSERT INTO environments(id,project_id,name,created_at,updated_at)VALUES(?,?,?,?,?)",
  )
  .bind(&id)
  .bind(&project.id)
  .bind(&request.name)
  .bind(&now)
  .bind(&now)
  .execute(&mut *tx)
  .await
  .map_err(unique)?;
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
  let (admin_id, email) = crate::extractors::require_admin(identity)?;
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
  let (admin_id, email) = crate::extractors::require_admin(identity)?;
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
