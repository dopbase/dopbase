use super::{
  model::*,
  repository::{self, NewGrant},
};
use crate::{
  constants::tokens::GRANT_ID_PREFIX,
  extractors::{
    require_browser, require_capability, require_mutation, require_person,
    require_recent_browser_auth,
  },
  http::HttpError,
  models::AuthIdentity,
  modules::{common, environments, projects},
  services::{
    authx::{self, Capability, Level},
    token,
  },
  state::AppState,
};
use axum::http::{HeaderMap, StatusCode};
use chrono::Utc;
use std::collections::BTreeMap;

fn not_found() -> HttpError {
  HttpError::not_found("GRANT_NOT_FOUND", "The grant was not found.")
}

fn invalid_level(message: &str) -> HttpError {
  HttpError::validation(BTreeMap::from([(
    "GRANT_LEVEL_INVALID".into(),
    message.into(),
  )]))
}

/// The ids a checked scope stands for.
struct Scope {
  scope_id: String,
  project_id: Option<String>,
  environment_id: Option<String>,
}

/// Checks the caller may grant on this scope: root for all projects, a
/// project manager of the project otherwise. Granting never goes above
/// the caller's own level, because a project manager already holds the
/// highest level a project or environment has.
async fn authorize_scope(
  state: &AppState,
  identity: &AuthIdentity,
  scope_type: ScopeType,
  scope: Option<&str>,
) -> Result<Scope, HttpError> {
  let scope = scope.filter(|value| !value.is_empty());
  match (scope_type, scope) {
    (ScopeType::All, None) => {
      require_capability(identity, Capability::GrantsAllProjects)?;
      Ok(Scope {
        scope_id: String::new(),
        project_id: None,
        environment_id: None,
      })
    }
    (ScopeType::Project, Some(reference)) => {
      let project =
        projects::service::authorize(state, identity, reference, Level::ProjectManager).await?;
      Ok(Scope {
        scope_id: project.id.clone(),
        project_id: Some(project.id),
        environment_id: None,
      })
    }
    (ScopeType::Environment, Some(id)) => {
      let environment =
        environments::service::authorize(state, identity, id, Level::Metadata).await?;
      projects::service::authorize(
        state,
        identity,
        &environment.project_id,
        Level::ProjectManager,
      )
      .await?;
      Ok(Scope {
        scope_id: environment.id.clone(),
        project_id: Some(environment.project_id),
        environment_id: Some(environment.id),
      })
    }
    _ => Err(HttpError::validation(BTreeMap::from([(
      "GRANT_SCOPE_INVALID".into(),
      "Name a project or an environment, or leave the scope out for all projects.".into(),
    )]))),
  }
}

fn validate_level(
  scope_type: ScopeType,
  level: Level,
) -> Result<(), HttpError> {
  match (scope_type, level) {
    (ScopeType::Environment, Level::ProjectManager) => Err(invalid_level(
      "Project manager is given on a project, not on one environment.",
    )),
    (ScopeType::All | ScopeType::Project, Level::None) => Err(invalid_level(
      "No access is set on one environment. Remove the grant instead.",
    )),
    _ => Ok(()),
  }
}

fn self_grant() -> HttpError {
  HttpError::forbidden("SELF_GRANT", "You can't change your own access.")
}

/// The principal's label, after checking a grant at `level` may be given
/// to it.
async fn principal_label(
  conn: &mut sqlx::SqliteConnection,
  actor_id: &str,
  principal_type: PrincipalType,
  principal_id: &str,
  level: Level,
) -> Result<String, HttpError> {
  match principal_type {
    PrincipalType::Group => repository::group_name(conn, principal_id)
      .await?
      .ok_or_else(|| HttpError::not_found("GROUP_NOT_FOUND", "The group was not found.")),
    PrincipalType::User => {
      let (email, is_root, role_id) = repository::account(conn, principal_id)
        .await?
        .ok_or_else(|| HttpError::not_found("USER_NOT_FOUND", "The user was not found."))?;
      if is_root {
        return Err(HttpError::forbidden(
          "ROOT_PROTECTED",
          "Root reaches everything and takes no grants.",
        ));
      }
      if principal_id == actor_id {
        return Err(self_grant());
      }
      let ceiling = match role_id {
        Some(role_id) => authx::role_access(conn, &role_id)
          .await?
          .map_or(Level::None, |role| role.max_level),
        None => Level::None,
      };
      if level > ceiling {
        return Err(invalid_level(
          "This level is above what the account's role allows.",
        ));
      }
      Ok(email)
    }
  }
}

pub async fn set(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  request: SetGrantRequest,
) -> Result<GrantResponse, HttpError> {
  require_mutation(identity, headers)?;
  require_recent_browser_auth(identity)?;
  let (actor_id, actor_email) = require_browser(identity)?;
  validate_level(request.scope_type, request.level)?;
  let scope = authorize_scope(
    state,
    identity,
    request.scope_type,
    request.scope.as_deref(),
  )
  .await?;
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin_with("BEGIN IMMEDIATE").await?;
  let label = principal_label(
    &mut tx,
    actor_id,
    request.principal_type,
    &request.principal_id,
    request.level,
  )
  .await?;
  let previous = repository::existing_level(
    &mut tx,
    request.principal_type,
    &request.principal_id,
    request.scope_type,
    &scope.scope_id,
  )
  .await?;
  let id = repository::upsert(
    &mut tx,
    NewGrant {
      id: &token::public_id(GRANT_ID_PREFIX),
      principal_type: request.principal_type,
      principal_id: &request.principal_id,
      scope_type: request.scope_type,
      scope_id: &scope.scope_id,
      level: request.level,
      granted_by: actor_id,
      now: &now,
    },
  )
  .await?;
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(actor_id),
    Some(actor_email),
    "grant.set",
    scope.project_id.as_deref(),
    scope.environment_id.as_deref(),
    Some("grant"),
    Some(&id),
    serde_json::json!({
      "principalType": request.principal_type.as_str(),
      "principalId": request.principal_id,
      "principal": label,
      "scopeType": request.scope_type.as_str(),
      "scopeId": scope.scope_id,
      "level": request.level.as_str(),
      "previousLevel": previous,
    }),
  )
  .await?;
  tx.commit().await?;
  let mut conn = state.db.pool().acquire().await?;
  repository::find(&mut conn, &id)
    .await?
    .ok_or_else(HttpError::internal)
}

pub async fn delete(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  id: &str,
) -> Result<(), HttpError> {
  require_mutation(identity, headers)?;
  require_recent_browser_auth(identity)?;
  let (actor_id, actor_email) = require_browser(identity)?;
  let mut conn = state.db.pool().acquire().await?;
  let grant = repository::find(&mut conn, id)
    .await?
    .ok_or_else(not_found)?;
  drop(conn);
  // A grant on something the caller cannot see does not exist for them.
  let scope = authorize_scope(state, identity, grant.scope_type, Some(&grant.scope_id))
    .await
    .map_err(|error| {
      if error.status == StatusCode::NOT_FOUND {
        not_found()
      } else {
        error
      }
    })?;
  if grant.principal_type == PrincipalType::User && grant.principal_id == actor_id {
    return Err(self_grant());
  }
  let mut tx = state.db.pool().begin_with("BEGIN IMMEDIATE").await?;
  if !repository::delete(&mut tx, id).await? {
    return Err(not_found());
  }
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(actor_id),
    Some(actor_email),
    "grant.removed",
    scope.project_id.as_deref(),
    scope.environment_id.as_deref(),
    Some("grant"),
    Some(id),
    serde_json::json!({
      "principalType": grant.principal_type.as_str(),
      "principalId": grant.principal_id,
      "principal": grant.principal_label,
      "scopeType": grant.scope_type.as_str(),
      "scopeId": grant.scope_id,
      "level": grant.level.as_str(),
    }),
  )
  .await?;
  tx.commit().await?;
  Ok(())
}

/// Who can reach a project. Its project managers can see this, and so can
/// anyone whose role reports on access.
pub async fn project_access(
  state: &AppState,
  identity: &AuthIdentity,
  reference: &str,
) -> Result<ProjectAccessResponse, HttpError> {
  require_person(identity)?;
  let needed = match identity {
    AuthIdentity::Admin { access, .. } if access.has(Capability::AccessReport) => Level::Metadata,
    _ => Level::ProjectManager,
  };
  let project = projects::service::authorize(state, identity, reference, needed).await?;
  let environments = environments::service::list(state, identity, Some(&project.id)).await?;
  let grants = repository::for_project(state.db.pool(), &project.id).await?;
  Ok(ProjectAccessResponse {
    project,
    environments,
    grants,
  })
}

/// Where an account's access comes from. Anyone can see their own; seeing
/// someone else's needs the users.view capability.
pub async fn user_access(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
) -> Result<UserAccessResponse, HttpError> {
  // A personal token can ask about its own account only.
  let (actor_id, token) = match identity {
    AuthIdentity::Token {
      admin_id,
      token_name,
      scope,
      level,
      audit,
      ..
    } => (
      admin_id.as_str(),
      Some(TokenLimit {
        name: token_name.clone(),
        scope_type: match scope {
          _ if *audit => ScopeType::Audit,
          authx::Scope::All => ScopeType::All,
          authx::Scope::Project(_) => ScopeType::Project,
          authx::Scope::Environment(_) => ScopeType::Environment,
        },
        scope_id: match scope {
          authx::Scope::All => String::new(),
          authx::Scope::Project(id) | authx::Scope::Environment(id) => id.clone(),
        },
        scope_label: String::new(),
        level: *level,
      }),
    ),
    _ => (require_person(identity)?.0, None),
  };
  let id = if id == "me" { actor_id } else { id };
  if id != actor_id {
    require_capability(identity, Capability::UsersView)?;
  }
  let mut conn = state.db.pool().acquire().await?;
  let email: Option<String> = sqlx::query_scalar("SELECT email FROM admins WHERE id = ?")
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?;
  let Some(email) = email else {
    return Err(HttpError::not_found(
      "USER_NOT_FOUND",
      "The user was not found.",
    ));
  };
  let access = authx::load_access(&mut conn, id).await?;
  let mut token = token;
  if let Some(token) = token.as_mut() {
    token.scope_label = sqlx::query_scalar(
      "SELECT COALESCE((SELECT name FROM projects WHERE id = ?1), (SELECT p.name || '/' || e.name FROM environments e JOIN projects p ON p.id = e.project_id WHERE e.id = ?1), '')",
    )
    .bind(&token.scope_id)
    .fetch_one(&mut *conn)
    .await?;
  }
  drop(conn);
  Ok(UserAccessResponse {
    user_id: id.to_owned(),
    email,
    role_name: access
      .as_ref()
      .map_or_else(String::new, |access| access.role_name.clone()),
    max_level: access.map_or(Level::None, |access| access.max_level),
    token,
    groups: repository::groups_of(state.db.pool(), id).await?,
    grants: repository::for_user(state.db.pool(), id).await?,
  })
}
