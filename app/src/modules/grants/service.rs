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
  modules::{common, environments, projects, users},
  services::{
    authx::{self, Capability, Level},
    features::Feature,
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

/// What the grant's principal is when it is an AI agent: `Some(owner)`.
/// Groups, people and missing accounts are `None`.
async fn principal_agent(
  state: &AppState,
  principal_type: PrincipalType,
  principal_id: &str,
) -> Result<Option<Option<String>>, HttpError> {
  if principal_type != PrincipalType::User {
    return Ok(None);
  }
  Ok(users::repository::agent(state.db.pool(), principal_id).await?)
}

/// Whether the caller is giving or removing access for an AI agent they
/// own, on one project or environment. Root's all-projects grants always go
/// the usual way.
fn for_own_agent(
  identity: &AuthIdentity,
  actor_id: &str,
  agent: Option<&Option<String>>,
  scope_type: ScopeType,
) -> bool {
  scope_type != ScopeType::All
    && agent.is_some_and(|owner| owner.as_deref() == Some(actor_id))
    && require_capability(identity, Capability::AgentsView).is_ok()
}

/// Checks an owner may give their own agent `level` here: they must reach
/// the project or environment at that level themselves, so an agent never
/// gets more than its owner has. Protected environments count, because the
/// owner's level on them already does. No access needs only sight of it.
async fn authorize_own_agent_scope(
  state: &AppState,
  identity: &AuthIdentity,
  scope_type: ScopeType,
  scope: Option<&str>,
  level: Level,
) -> Result<Scope, HttpError> {
  let needed = level.max(Level::Metadata);
  match (scope_type, scope.filter(|value| !value.is_empty())) {
    (ScopeType::Project, Some(reference)) => {
      let project = projects::service::authorize(state, identity, reference, needed).await?;
      Ok(Scope {
        scope_id: project.id.clone(),
        project_id: Some(project.id),
        environment_id: None,
      })
    }
    (ScopeType::Environment, Some(id)) => {
      let environment = environments::service::authorize(state, identity, id, needed).await?;
      Ok(Scope {
        scope_id: environment.id.clone(),
        project_id: Some(environment.project_id),
        environment_id: Some(environment.id),
      })
    }
    _ => authorize_scope(state, identity, scope_type, scope).await,
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

fn principal_invalid() -> HttpError {
  HttpError::validation(BTreeMap::from([(
    "GRANT_PRINCIPAL_INVALID".into(),
    "This account can't be given access.".into(),
  )]))
}

/// The principal's label, after checking a grant at `level` may be given
/// to it. A caller grants to groups and people they may list, to AI agents
/// with agents.manage, and to their own agents, as the Admin UI offers, so
/// ids, root and role ceilings can't be probed.
#[allow(clippy::too_many_arguments)]
async fn principal_label(
  conn: &mut sqlx::SqliteConnection,
  identity: &AuthIdentity,
  actor_id: &str,
  principal_type: PrincipalType,
  principal_id: &str,
  level: Level,
  agent: bool,
  own_agent: bool,
) -> Result<String, HttpError> {
  let can = |capability| require_capability(identity, capability).is_ok();
  match principal_type {
    PrincipalType::Group if !can(Capability::GroupsManage) && !can(Capability::UsersView) => {
      Err(principal_invalid())
    }
    PrincipalType::Group => repository::group_name(conn, principal_id)
      .await?
      .ok_or_else(|| HttpError::not_found("GROUP_NOT_FOUND", "The group was not found.")),
    // People need users.view and AI agents agents.manage; users.view
    // doesn't reach agents. An owner gives their own agent access without
    // either. A missing account answers like a person.
    PrincipalType::User
      if !own_agent
        && !can(if agent {
          Capability::AgentsManage
        } else {
          Capability::UsersView
        }) =>
    {
      Err(principal_invalid())
    }
    PrincipalType::User => {
      // Root takes no grants, and a missing id gets the same answer.
      let (email, _, role_id) = repository::account(conn, principal_id)
        .await?
        .filter(|(_, is_root, _)| !is_root)
        .ok_or_else(principal_invalid)?;
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
  let agent = principal_agent(state, request.principal_type, &request.principal_id).await?;
  // Without RBAC only AI agents take new access.
  if agent.is_none() {
    state.config.features.require(Feature::Rbac)?;
  }
  let own_agent = for_own_agent(identity, actor_id, agent.as_ref(), request.scope_type);
  let scope = if own_agent {
    authorize_own_agent_scope(
      state,
      identity,
      request.scope_type,
      request.scope.as_deref(),
      request.level,
    )
    .await?
  } else {
    authorize_scope(
      state,
      identity,
      request.scope_type,
      request.scope.as_deref(),
    )
    .await?
  };
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin_with("BEGIN IMMEDIATE").await?;
  let label = principal_label(
    &mut tx,
    identity,
    actor_id,
    request.principal_type,
    &request.principal_id,
    request.level,
    agent.is_some(),
    own_agent,
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
  let agent = principal_agent(state, grant.principal_type, &grant.principal_id).await?;
  if agent.is_none() {
    state.config.features.require(Feature::Rbac)?;
  }
  let own_agent = for_own_agent(identity, actor_id, agent.as_ref(), grant.scope_type);
  // A grant on something the caller cannot see does not exist for them. An
  // owner takes access away from their own agent wherever they can see.
  let checked = if own_agent {
    authorize_own_agent_scope(
      state,
      identity,
      grant.scope_type,
      Some(&grant.scope_id),
      Level::Metadata,
    )
    .await
  } else {
    authorize_scope(state, identity, grant.scope_type, Some(&grant.scope_id)).await
  };
  let scope = checked.map_err(|error| {
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

/// Where an account's access comes from. Anyone can see their own. A
/// person's needs users.view; an AI agent's needs agents.view_all, or
/// agents.view when the caller owns it.
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
    // An AI agent's access shows to its owner and to agents.view_all;
    // a person's to users.view.
    match users::repository::agent(state.db.pool(), id).await? {
      Some(owner) if owner.as_deref() == Some(actor_id) => {
        require_capability(identity, Capability::AgentsView)?;
      }
      Some(_) => {
        require_capability(identity, Capability::AgentsViewAll)?;
      }
      None => {
        require_capability(identity, Capability::UsersView)?;
      }
    }
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
  let mut grants = repository::for_user(state.db.pool(), id).await?;
  let owner_email = match users::repository::agent(state.db.pool(), id).await? {
    Some(owner_id) => cap_by_owner(&mut conn, owner_id.as_deref(), &mut grants).await?,
    None => None,
  };
  drop(conn);
  Ok(UserAccessResponse {
    user_id: id.to_owned(),
    email,
    role_name: access
      .as_ref()
      .map_or_else(String::new, |access| access.role_name.clone()),
    owner_email,
    max_level: access.map_or(Level::None, |access| access.max_level),
    token,
    groups: repository::groups_of(state.db.pool(), id).await?,
    grants,
  })
}

/// Marks each of an AI agent's grants that its owner doesn't back in full
/// with the owner's own level there, and returns the owner's email. A
/// missing or disabled owner backs nothing, the same as when the agent
/// calls the API.
async fn cap_by_owner(
  conn: &mut sqlx::SqliteConnection,
  owner_id: Option<&str>,
  grants: &mut [GrantResponse],
) -> Result<Option<String>, HttpError> {
  let owner = match owner_id {
    Some(owner_id) => match authx::load_access(conn, owner_id).await? {
      Some(access) => Some(authx::account_data_access(conn, owner_id, &access).await?),
      None => None,
    },
    None => None,
  };
  for grant in grants.iter_mut() {
    let owner_level = match (&owner, grant.scope_type) {
      (None, _) => Level::None,
      (Some(owner), ScopeType::Project) => owner.project(&grant.project_id),
      (Some(owner), ScopeType::Environment) => {
        let protected: bool = sqlx::query_scalar("SELECT protected FROM environments WHERE id = ?")
          .bind(&grant.scope_id)
          .fetch_optional(&mut *conn)
          .await?
          .unwrap_or(false);
        owner.environment(authx::EnvironmentTarget {
          project_id: &grant.project_id,
          environment_id: &grant.scope_id,
          protected,
        })
      }
      // Agents get no all-projects or audit grants.
      (Some(_), _) => continue,
    };
    if owner_level < grant.level {
      grant.owner_level = Some(owner_level);
    }
  }
  let Some(owner_id) = owner_id else {
    return Ok(None);
  };
  Ok(
    sqlx::query_scalar("SELECT email FROM admins WHERE id = ?")
      .bind(owner_id)
      .fetch_optional(&mut *conn)
      .await?,
  )
}
