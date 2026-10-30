use super::{model::*, repository};
use crate::modules::common;
use crate::{
  constants::errors::{AUTHORIZATION_DENIED, ENVIRONMENT_NAME_INVALID, TOKEN_SCOPE_INVALID},
  extractors::require_person,
  http::HttpError,
  models::{AffectedCounts, AuthIdentity},
  services::authx::{self, DataAccess, EnvironmentTarget, Level},
  state::AppState,
  utils::generator,
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
  const MAX_ATTEMPTS: usize = 5;
  for _ in 0..MAX_ATTEMPTS {
    let id = generator::environment_id().map_err(|error| {
      tracing::error!(%error, "failed to generate environment id");
      HttpError::internal()
    })?;
    if repository::insert(tx, &id, project_id, name, now)
      .await
      .map_err(unique)?
    {
      return Ok(id);
    }
  }
  tracing::error!(
    attempts = MAX_ATTEMPTS,
    "environment id generation collided repeatedly"
  );
  Err(HttpError::internal())
}

fn not_found() -> HttpError {
  HttpError::not_found(
    "ENVIRONMENT_NOT_FOUND",
    "The requested environment was not found.",
  )
}

fn target(environment: &EnvironmentResponse) -> EnvironmentTarget<'_> {
  EnvironmentTarget {
    project_id: &environment.project_id,
    environment_id: &environment.id,
    protected: environment.protected,
  }
}

/// Fills in the caller's level and checks it reaches `needed`. An
/// environment the caller cannot see at all is reported as missing, so its
/// existence does not leak.
fn check(
  mut environment: EnvironmentResponse,
  access: &DataAccess,
  needed: Level,
) -> Result<EnvironmentResponse, HttpError> {
  environment.level = access.environment(target(&environment));
  if environment.level == Level::None {
    return Err(not_found());
  }
  if environment.level < needed {
    return Err(HttpError::forbidden(
      AUTHORIZATION_DENIED,
      "Your access to this environment does not allow this.",
    ));
  }
  Ok(environment)
}

pub(crate) async fn data_access(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<DataAccess, HttpError> {
  let mut conn = state.db.pool().acquire().await?;
  Ok(authx::data_access(&mut conn, identity).await?)
}

/// The environment, when the caller's level on it is at least `needed`.
pub async fn authorize(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
  needed: Level,
) -> Result<EnvironmentResponse, HttpError> {
  let environment = show(state, id).await?;
  check(environment, &data_access(state, identity).await?, needed)
}

pub async fn list(
  state: &AppState,
  identity: &AuthIdentity,
  project: Option<&str>,
) -> Result<Vec<EnvironmentResponse>, HttpError> {
  crate::extractors::require_metadata_access(identity)?;
  let access = data_access(state, identity).await?;
  Ok(
    repository::list(state.db.pool(), project)
      .await?
      .into_iter()
      .filter_map(|environment| check(environment, &access, Level::Metadata).ok())
      .collect(),
  )
}
/// The environment without any access check.
pub(crate) async fn show(
  state: &AppState,
  id: &str,
) -> Result<EnvironmentResponse, HttpError> {
  repository::find_id(state.db.pool(), id)
    .await?
    .ok_or_else(not_found)
}
pub async fn resolve(
  state: &AppState,
  identity: &AuthIdentity,
  reference: &str,
) -> Result<EnvironmentResponse, HttpError> {
  let environment = repository::resolve(state.db.pool(), reference)
    .await?
    .ok_or_else(not_found)?;
  if let AuthIdentity::Runner { environment_id, .. } = identity
    && *environment_id != environment.id
  {
    return Err(HttpError::forbidden(
      TOKEN_SCOPE_INVALID,
      "The runner token cannot access this environment.",
    ));
  }
  check(
    environment,
    &data_access(state, identity).await?,
    Level::Metadata,
  )
}
pub async fn create(
  state: &AppState,
  identity: &AuthIdentity,
  project_ref: &str,
  request: CreateEnvironmentRequest,
) -> Result<EnvironmentResponse, HttpError> {
  common::validate_slug(&request.name, ENVIRONMENT_NAME_INVALID, "Environment name")?;
  let (admin_id, email) = require_person(identity)?;
  let project = crate::modules::projects::service::authorize(
    state,
    identity,
    project_ref,
    Level::ProjectManager,
  )
  .await?;
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin_with("BEGIN IMMEDIATE").await?;
  let id = insert_generated(&mut tx, &project.id, &request.name, &now).await?;
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
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
  authorize(state, identity, &id, Level::Metadata).await
}
pub async fn update(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
  request: UpdateEnvironmentRequest,
) -> Result<EnvironmentResponse, HttpError> {
  if let Some(name) = &request.name {
    common::validate_slug(name, ENVIRONMENT_NAME_INVALID, "Environment name")?;
  }
  let (admin_id, email) = require_person(identity)?;
  let environment = authorize(state, identity, id, Level::Manage).await?;
  let name = request.name.as_deref().unwrap_or(&environment.name);
  let protected = request.protected.unwrap_or(environment.protected);
  let mut tx = state.db.pool().begin().await?;
  let updated = sqlx::query("UPDATE environments SET name=?,protected=?,updated_at=? WHERE id=?")
    .bind(name)
    .bind(protected)
    .bind(Utc::now().to_rfc3339())
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(unique)?;
  if updated.rows_affected() != 1 {
    return Err(not_found());
  }
  if name != environment.name {
    common::audit(
      &mut tx,
      identity.audit_actor_type(),
      Some(admin_id),
      Some(email),
      "environment.renamed",
      Some(&environment.project_id),
      Some(id),
      Some("environment"),
      Some(id),
      serde_json::json!({"oldName":environment.name,"newName":name}),
    )
    .await?;
  }
  if protected != environment.protected {
    common::audit(
      &mut tx,
      identity.audit_actor_type(),
      Some(admin_id),
      Some(email),
      if protected {
        "environment.protected"
      } else {
        "environment.unprotected"
      },
      Some(&environment.project_id),
      Some(id),
      Some("environment"),
      Some(id),
      serde_json::json!({"name":name}),
    )
    .await?;
  }
  tx.commit().await?;
  // Unprotecting can leave the caller with less than before, but never
  // with nothing they could not already see.
  authorize(state, identity, id, Level::Metadata).await
}
pub async fn delete(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
) -> Result<DeleteEnvironmentResponse, HttpError> {
  let (admin_id, email) = require_person(identity)?;
  let environment = authorize(state, identity, id, Level::Manage).await?;
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
    return Err(not_found());
  }
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
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
