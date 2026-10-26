use super::{
  model::*,
  repository::{self, NewUser, UserRow},
};
use crate::{
  constants::errors::PASSWORD_REQUIRED,
  extractors::{require_capability_browser, require_mutation, require_recent_browser_auth},
  http::HttpError,
  models::{AdminRole, AuthIdentity},
  modules::common,
  services::{
    authx::{self, Access, Capability, Catalog},
    token,
  },
  state::AppState,
};
use axum::http::HeaderMap;
use chrono::Utc;

fn actor(identity: &AuthIdentity) -> (&str, &str) {
  match identity {
    AuthIdentity::Admin {
      admin_id, email, ..
    } => (admin_id, email),
    _ => ("", ""),
  }
}

/// The role an account gets, picked by id or by its fixed name.
struct ChosenRole {
  id: String,
  /// The legacy `admins.role` value.
  legacy: AdminRole,
  /// Holders sign in with personal tokens only, like AI agents.
  token_only: bool,
}

async fn choose_role(
  state: &AppState,
  role: Option<AdminRole>,
  role_id: Option<&str>,
) -> Result<ChosenRole, HttpError> {
  let id = match (role_id, role.unwrap_or(AdminRole::Admin)) {
    (Some(id), _) => id,
    (None, AdminRole::Root) => {
      return Err(HttpError::forbidden(
        "ROOT_PROTECTED",
        "The root role is reserved for the setup account.",
      ));
    }
    (None, AdminRole::Admin) => "rol_admin",
    (None, AdminRole::Member) => "rol_member",
    (None, AdminRole::AiAgent) => "rol_ai_agent",
  };
  let token_only = repository::role_sign_in(state.db.pool(), id)
    .await?
    .ok_or_else(|| HttpError::bad_request("ROLE_INVALID", "The role was not found."))?;
  let legacy = match id {
    "rol_admin" => AdminRole::Admin,
    _ if token_only => AdminRole::AiAgent,
    _ => AdminRole::Member,
  };
  Ok(ChosenRole {
    id: id.to_owned(),
    legacy,
    token_only,
  })
}

/// An AI agent's name, which is its handle where people have an email.
/// It never contains an @, so it can never match a person's email.
fn validate_agent_name(value: &str) -> Result<String, HttpError> {
  let name = value.trim();
  let valid = (2..=64).contains(&name.len())
    && name
      .bytes()
      .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
  if !valid {
    return Err(HttpError::validation(std::collections::BTreeMap::from([(
      "AGENT_NAME_INVALID".into(),
      "Use 2 to 64 letters, digits, dots, dashes, or underscores.".into(),
    )])));
  }
  Ok(name.to_owned())
}

fn agents_stay_agents() -> HttpError {
  HttpError::bad_request(
    "ROLE_INVALID",
    "An AI agent can't become a person, or a person an AI agent. Create a new account instead.",
  )
}

fn exceeds_access() -> HttpError {
  HttpError::forbidden(
    "ACCESS_EXCEEDS_YOURS",
    "This account has access you don't have, so you can't manage it.",
  )
}

/// The caller's own access and id.
fn caller(identity: &AuthIdentity) -> Result<(&str, &Access), HttpError> {
  match identity {
    AuthIdentity::Admin {
      admin_id, access, ..
    } => Ok((admin_id, access)),
    _ => Err(HttpError::internal()),
  }
}

/// Only roles within the caller's own capabilities and ceiling can be
/// handed out.
async fn require_assignable(
  state: &AppState,
  identity: &AuthIdentity,
  role_id: &str,
) -> Result<(), HttpError> {
  let (_, access) = caller(identity)?;
  let mut conn = state.db.pool().acquire().await?;
  let role = authx::role_access(&mut conn, role_id)
    .await?
    .ok_or_else(|| HttpError::bad_request("ROLE_INVALID", "The role was not found."))?;
  if !access.includes(&role) {
    return Err(HttpError::forbidden(
      "ROLE_EXCEEDS_YOURS",
      "You can only give roles that are within your own.",
    ));
  }
  Ok(())
}

/// A person can manage another account only when that account's role and
/// data access are within their own. Root manages everyone.
pub(crate) async fn require_within(
  state: &AppState,
  identity: &AuthIdentity,
  target_id: &str,
) -> Result<(), HttpError> {
  let (actor_id, access) = caller(identity)?;
  if access.is_root {
    return Ok(());
  }
  let mut conn = state.db.pool().acquire().await?;
  let target = authx::load_access(&mut conn, target_id)
    .await?
    .unwrap_or_else(Access::none);
  if !access.includes(&target) {
    return Err(exceeds_access());
  }
  let mine = authx::account_data_access(&mut conn, actor_id, access).await?;
  let theirs = authx::account_data_access(&mut conn, target_id, &target).await?;
  let catalog = Catalog::load(&mut conn).await?;
  if !mine.covers(&theirs, &catalog.projects, &catalog.environments()) {
    return Err(exceeds_access());
  }
  Ok(())
}

/// The all-projects grant a new or changed account starts with. Only root
/// gives it, and only to admins; everyone else starts with no data access
/// and gets grants per project.
fn starting_grant(
  identity: &AuthIdentity,
  role_id: &str,
) -> Option<&'static str> {
  let (_, access) = caller(identity).ok()?;
  (role_id == "rol_admin" && access.has(Capability::GrantsAllProjects)).then_some("project_manager")
}

fn parse_role(value: &str) -> Result<AdminRole, HttpError> {
  match value {
    "ai_agent" => Ok(AdminRole::AiAgent),
    other => AdminRole::parse_human(other).ok_or_else(HttpError::internal),
  }
}

fn user(row: UserRow) -> Result<UserResponse, HttpError> {
  Ok(UserResponse {
    role: parse_role(&row.role)?,
    id: row.id,
    email: row.email,
    role_id: row.role_id,
    role_name: row.role_name,
    created_at: row.created_at,
    updated_at: row.updated_at,
    has_password: row.has_password,
    sso_linked: row.sso_linked,
    owner_id: row.owner_id,
  })
}

async fn fetch_user(
  state: &AppState,
  id: &str,
) -> Result<UserResponse, HttpError> {
  repository::find(state.db.pool(), id)
    .await?
    .map(user)
    .transpose()?
    .ok_or_else(|| HttpError::not_found("USER_NOT_FOUND", "The user was not found."))
}

pub async fn list(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<Vec<UserResponse>, HttpError> {
  require_capability_browser(identity, Capability::UsersView)?;
  repository::list(state.db.pool())
    .await?
    .into_iter()
    .map(user)
    .collect()
}

pub async fn get(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
) -> Result<UserResponse, HttpError> {
  require_capability_browser(identity, Capability::UsersView)?;
  fetch_user(state, id).await
}

pub async fn create(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  request: CreateUserRequest,
) -> Result<UserResponse, HttpError> {
  require_mutation(identity, headers)?;
  require_recent_browser_auth(identity)?;
  require_capability_browser(identity, Capability::UsersManage)?;
  let chosen = choose_role(state, request.role, request.role_id.as_deref()).await?;
  let agent = chosen.token_only;
  let email = if agent {
    validate_agent_name(&request.email)?
  } else {
    common::validate_email(&request.email)?
  };
  let password = request.password.filter(|password| !password.is_empty());
  let has_password = password.is_some();
  let hash = match password {
    Some(_) if agent => {
      return Err(HttpError::bad_request(
        "PASSWORD_NOT_ALLOWED",
        "AI agents sign in with tokens only, so they have no password.",
      ));
    }
    // An AI agent never signs in, so it never needs a password.
    None if agent => common::NO_PASSWORD_HASH.to_owned(),
    Some(password) => {
      common::validate_password(&password)?;
      common::hash_password_async(password).await?
    }
    // Without a password the only way in is single sign-on, so it must be on.
    None
      if crate::modules::sso::service::policy(state)
        .await?
        .enabled_name
        .is_some() =>
    {
      common::NO_PASSWORD_HASH.to_owned()
    }
    None => {
      return Err(HttpError::validation(std::collections::BTreeMap::from([(
        PASSWORD_REQUIRED.into(),
        "Enter a password. Accounts without one need single sign-on turned on.".into(),
      )])));
    }
  };
  let id = token::public_id("usr_");
  let now = Utc::now().to_rfc3339();
  let (actor_id, actor_email) = actor(identity);
  require_assignable(state, identity, &chosen.id).await?;
  let mut tx = state.db.pool().begin().await?;
  let inserted = repository::insert(
    &mut tx,
    NewUser {
      id: &id,
      email: &email,
      password_hash: &hash,
      role: chosen.legacy.as_str(),
      role_id: &chosen.id,
      // The person who creates an AI agent answers for it.
      owner_id: agent.then_some(actor_id),
      created_by: actor_id,
      now: &now,
    },
  )
  .await;
  if let Err(error) = inserted {
    if error.to_string().contains("UNIQUE") {
      return Err(HttpError::conflict(
        "EMAIL_ALREADY_EXISTS",
        "An account with this email already exists.",
      ));
    }
    return Err(error.into());
  }
  if let Some(level) = starting_grant(identity, &chosen.id) {
    repository::set_all_projects_grant(&mut tx, &id, Some(level), actor_id, &now).await?;
  }
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(actor_id),
    Some(actor_email),
    "admin.created",
    None,
    None,
    Some("admin"),
    Some(&id),
    serde_json::json!({"email":email,"role":chosen.id,"password":has_password}),
  )
  .await?;
  tx.commit().await?;
  fetch_user(state, &id).await
}

pub async fn update(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  id: &str,
  request: UpdateUserRequest,
) -> Result<UserResponse, HttpError> {
  require_mutation(identity, headers)?;
  require_recent_browser_auth(identity)?;
  require_capability_browser(identity, Capability::UsersManage)?;
  let (old_email, is_root, old_role_id, agent) = repository::account(state.db.pool(), id)
    .await?
    .ok_or_else(|| HttpError::not_found("USER_NOT_FOUND", "The user was not found."))?;
  let (actor_id, _) = actor(identity);
  let changes_role = request.role.is_some() || request.role_id.is_some();
  if is_root && id != actor_id {
    return Err(HttpError::forbidden(
      "ROOT_PROTECTED",
      "The root administrator cannot be modified.",
    ));
  }
  if id == actor_id && changes_role {
    return Err(HttpError::forbidden(
      "SELF_ROLE_CHANGE",
      "You cannot change your own role.",
    ));
  }
  if is_root && changes_role {
    return Err(HttpError::forbidden(
      "ROOT_PROTECTED",
      "The root role cannot be changed.",
    ));
  }
  if is_root && request.password.is_some() {
    return Err(HttpError::forbidden(
      "ROOT_PROTECTED",
      "Use the root password change flow to change the root password.",
    ));
  }
  let new_role = if changes_role {
    Some(choose_role(state, request.role, request.role_id.as_deref()).await?)
  } else {
    None
  };
  if new_role
    .as_ref()
    .is_some_and(|role| role.token_only != agent)
  {
    return Err(agents_stay_agents());
  }
  if agent && request.password.is_some() {
    return Err(HttpError::bad_request(
      "PASSWORD_NOT_ALLOWED",
      "AI agents sign in with tokens only, so they have no password.",
    ));
  }
  if request.owner_id.is_some() && !agent {
    return Err(HttpError::bad_request(
      "OWNER_INVALID",
      "Only AI agents have an owner.",
    ));
  }
  if !is_root {
    require_within(state, identity, id).await?;
  }
  // Keeping the same role changes nothing.
  let new_role = new_role.filter(|role| old_role_id.as_deref() != Some(role.id.as_str()));
  if let Some(role) = &new_role {
    require_assignable(state, identity, &role.id).await?;
  }
  let email = match request.email {
    Some(value) if agent => validate_agent_name(&value)?,
    Some(value) => common::validate_email(&value)?,
    None => old_email,
  };
  let password_hash = if let Some(password) = request.password {
    common::validate_password(&password)?;
    Some(common::hash_password_async(password).await?)
  } else {
    None
  };
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  let updated = repository::update(
    &mut tx,
    id,
    &email,
    password_hash.as_deref(),
    new_role
      .as_ref()
      .map(|role| (role.legacy.as_str(), role.id.as_str())),
    &now,
  )
  .await;
  // A new role starts over on all projects; project grants stay and are
  // capped by the new role's ceiling.
  let result = match (updated, &new_role) {
    (Ok(()), Some(role)) => {
      repository::set_all_projects_grant(
        &mut tx,
        id,
        starting_grant(identity, &role.id),
        actor_id,
        &now,
      )
      .await
    }
    (other, _) => other,
  };
  if let Err(error) = result {
    if error.to_string().contains("UNIQUE") {
      return Err(HttpError::conflict(
        "EMAIL_ALREADY_EXISTS",
        "An account with this email already exists.",
      ));
    }
    return Err(error.into());
  }
  if let Some(owner_id) = &request.owner_id {
    if !repository::can_own(&mut tx, owner_id).await? {
      return Err(HttpError::bad_request(
        "OWNER_INVALID",
        "The owner must be an active person who signs in.",
      ));
    }
    repository::set_owner(&mut tx, id, owner_id).await?;
  }
  repository::revoke_sessions(&mut tx, id, &now).await?;
  let (_, actor_email) = actor(identity);
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(actor_id),
    Some(actor_email),
    "admin.updated",
    None,
    None,
    Some("admin"),
    Some(id),
    serde_json::json!({"email":email,"role":new_role.as_ref().map(|role| &role.id),"ownerId":request.owner_id}),
  )
  .await?;
  tx.commit().await?;
  fetch_user(state, id).await
}

pub async fn delete(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  id: &str,
) -> Result<(), HttpError> {
  require_mutation(identity, headers)?;
  require_recent_browser_auth(identity)?;
  require_capability_browser(identity, Capability::UsersManage)?;
  let (actor_id, actor_email) = actor(identity);
  if id == actor_id {
    return Err(HttpError::forbidden(
      "SELF_DELETE_FORBIDDEN",
      "You cannot delete your own account.",
    ));
  }
  let (_, is_root, _, _) = repository::account(state.db.pool(), id)
    .await?
    .ok_or_else(|| HttpError::not_found("USER_NOT_FOUND", "The user was not found."))?;
  if is_root {
    return Err(HttpError::forbidden(
      "ROOT_PROTECTED",
      "The root administrator cannot be deleted.",
    ));
  }
  require_within(state, identity, id).await?;
  if repository::owned_count(state.db.pool(), id).await? > 0 {
    return Err(HttpError::conflict(
      "OWNS_AI_AGENTS",
      "This person owns AI agents. Delete them or give them another owner first.",
    ));
  }
  let mut tx = state.db.pool().begin().await?;
  repository::delete(&mut tx, id).await?;
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(actor_id),
    Some(actor_email),
    "admin.deleted",
    None,
    None,
    Some("admin"),
    Some(id),
    serde_json::json!({}),
  )
  .await?;
  tx.commit().await?;
  Ok(())
}

/// Remove a user's single sign-on link, for example after their identity
/// was recreated at the provider. A verified email links them again on the
/// next sign-in. Root's link can only be removed by root.
pub async fn unlink_sso(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  id: &str,
) -> Result<UserResponse, HttpError> {
  require_mutation(identity, headers)?;
  require_recent_browser_auth(identity)?;
  require_capability_browser(identity, Capability::UsersManage)?;
  let target = fetch_user(state, id).await?;
  let (actor_id, _) = actor(identity);
  // TODO(rbac): only root may change root. Check a permission instead once
  // roles are configurable.
  if target.role == AdminRole::Root && id != actor_id {
    return Err(HttpError::forbidden(
      "ROOT_PROTECTED",
      "The root administrator cannot be modified.",
    ));
  }
  if target.role != AdminRole::Root {
    require_within(state, identity, id).await?;
  }
  crate::modules::sso::service::unlink(state, identity, id, &target.email).await?;
  fetch_user(state, id).await
}
