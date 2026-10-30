use super::{model::*, repository};
use crate::modules::common;
use crate::{
  constants::errors::AUTHORIZATION_DENIED,
  constants::{
    errors::{
      ENVIRONMENT_NAME_INVALID, IMPORT_DUPLICATE_KEY, SECRET_COUNT_LIMIT_MESSAGE,
      SECRET_LIMIT_EXCEEDED, SECRET_TOTAL_SIZE_LIMIT_MESSAGE,
    },
    limits::{MAX_SECRET_COLLECTION_BYTES, MAX_SECRETS_PER_ENVIRONMENT},
    tokens::PROJECT_ID_PREFIX,
  },
  extractors::{require_capability, require_read_access},
  http::HttpError,
  models::{AffectedCounts, AuthIdentity},
  services::{
    authx::{self, Capability, Catalog, DataAccess, Level},
    token,
  },
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
fn not_found() -> HttpError {
  HttpError::not_found("PROJECT_NOT_FOUND", "The requested project was not found.")
}

/// The caller's level on a project, or `None` when they cannot see it.
/// Seeing only some environments counts as metadata on the project.
fn level_on(
  access: &DataAccess,
  catalog: &Catalog,
  project_id: &str,
) -> Option<Level> {
  let level = access.project(project_id);
  if level > Level::None {
    Some(level)
  } else if access.sees_project(project_id, catalog.environments_of(project_id)) {
    Some(Level::Metadata)
  } else {
    None
  }
}

async fn load(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<(DataAccess, Catalog), HttpError> {
  let mut conn = state.db.pool().acquire().await?;
  let access = authx::data_access(&mut conn, identity).await?;
  let catalog = Catalog::load(&mut conn).await?;
  Ok((access, catalog))
}

pub async fn list(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<Vec<ProjectResponse>, HttpError> {
  require_read_access(identity)?;
  let (access, catalog) = load(state, identity).await?;
  Ok(
    repository::list(state.db.pool())
      .await?
      .into_iter()
      .filter_map(|mut project| {
        project.level = level_on(&access, &catalog, &project.id)?;
        Some(project)
      })
      .collect(),
  )
}

/// The project, when the caller's level on it is at least `needed`. A
/// project the caller cannot see at all is reported as missing.
pub async fn authorize(
  state: &AppState,
  identity: &AuthIdentity,
  reference: &str,
  needed: Level,
) -> Result<ProjectResponse, HttpError> {
  require_read_access(identity)?;
  let mut project = repository::find(state.db.pool(), reference)
    .await?
    .ok_or_else(not_found)?;
  let (access, catalog) = load(state, identity).await?;
  project.level = level_on(&access, &catalog, &project.id).ok_or_else(not_found)?;
  if project.level < needed {
    return Err(HttpError::forbidden(
      AUTHORIZATION_DENIED,
      "Your access to this project does not allow this.",
    ));
  }
  Ok(project)
}

/// The account to make project manager of a new project, when its grants
/// would not show it the project otherwise. Root needs no grant.
async fn creator_needing_grant<'a>(
  state: &AppState,
  identity: &'a AuthIdentity,
) -> Result<Option<&'a str>, HttpError> {
  let AuthIdentity::Admin {
    admin_id, access, ..
  } = identity
  else {
    return Ok(None);
  };
  let mut conn = state.db.pool().acquire().await?;
  let data = authx::account_data_access(&mut conn, admin_id, access).await?;
  // A new project has no grants of its own yet, so only an all-projects
  // grant can reach it.
  Ok((data.project("") < Level::ProjectManager).then_some(admin_id.as_str()))
}
pub async fn create(
  state: &AppState,
  identity: &AuthIdentity,
  request: CreateProjectRequest,
) -> Result<ProjectResponse, HttpError> {
  common::validate_slug(&request.name, "PROJECT_NAME_INVALID", "Project name")?;
  let (admin_id, email) = require_capability(identity, Capability::ProjectsCreate)?;
  let creator = creator_needing_grant(state, identity).await?;
  let id = token::public_id(PROJECT_ID_PREFIX);
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  let mut project = repository::insert(&mut tx, &id, &request.name, &now)
    .await
    .map_err(map_unique)?;
  if let Some(creator) = creator {
    repository::grant_creator(&mut tx, &id, creator, &now).await?;
  }
  project.level = Level::ProjectManager;
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
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
  let (admin_id, email) = crate::extractors::require_person(identity)?;
  let project = authorize(state, identity, reference, Level::ProjectManager).await?;
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
    return Err(not_found());
  }
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
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
  authorize(state, identity, &project.id, Level::Metadata).await
}
pub async fn delete(
  state: &AppState,
  identity: &AuthIdentity,
  reference: &str,
) -> Result<DeleteProjectResponse, HttpError> {
  let (admin_id, email) = crate::extractors::require_person(identity)?;
  let project = authorize(state, identity, reference, Level::ProjectManager).await?;
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
    return Err(not_found());
  }
  common::audit(&mut tx,identity.audit_actor_type(),Some(admin_id),Some(email),"project.deleted",Some(&project.id),None,Some("project"),Some(&project.id),serde_json::json!({"name":project.name,"environments":environments,"secrets":secrets,"tokens":tokens})).await?;
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
  let (admin_id, email) = require_capability(identity, Capability::ProjectsCreate)?;
  let creator = creator_needing_grant(state, identity).await?;
  let project_id = token::public_id(PROJECT_ID_PREFIX);
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin_with("BEGIN IMMEDIATE").await?;
  let mut project = repository::insert(&mut tx, &project_id, &request.project_name, &now)
    .await
    .map_err(map_unique)?;
  if let Some(creator) = creator {
    repository::grant_creator(&mut tx, &project_id, creator, &now).await?;
  }
  project.level = Level::ProjectManager;
  let environment_id = crate::modules::environments::service::insert_generated(
    &mut tx,
    &project_id,
    &request.environment_name,
    &now,
  )
  .await?;
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
  for (entry, value) in encrypted {
    crate::modules::secrets::repository::write_version(
      &mut tx,
      crate::modules::secrets::repository::NewVersion {
        environment_id: &environment_id,
        key: &entry.key,
        version: 1,
        encrypted: value,
        created_at: &now,
        now: &now,
        change: crate::modules::secrets::repository::ChangeType::Created,
        source_version: None,
        actor: crate::modules::secrets::repository::VersionActor {
          actor_type: identity.audit_actor_type(),
          actor_id: Some(admin_id),
          actor_label: Some(email),
        },
        keep: state.config.secret_history_versions,
      },
    )
    .await?;
  }
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
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
