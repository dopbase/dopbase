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
  services::{authx::Capability, token},
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

// TODO(rbac): only the fixed admin and member roles can be given out here.
// Accept any assignable role from the role tables once roles are
// configurable.
fn human_role(role: Option<AdminRole>) -> Result<AdminRole, HttpError> {
  match role.unwrap_or(AdminRole::Admin) {
    AdminRole::Root => Err(HttpError::forbidden(
      "ROOT_PROTECTED",
      "The root role is reserved for the setup account.",
    )),
    AdminRole::AiAgent => Err(HttpError::bad_request(
      "ROLE_INVALID",
      "AI agents must be created as service accounts.",
    )),
    value => Ok(value),
  }
}

/// The role row and the all-projects grant that keep a fixed role's access
/// as it was: admins manage every project and members change them all.
// TODO(rbac): maps the fixed admin and member roles to their built-in role
// rows and shared grants. Take the role id from the request and grant
// access per project once grants can be managed.
fn role_access(role: AdminRole) -> (&'static str, &'static str) {
  match role {
    AdminRole::Admin => ("rol_admin", "project_manager"),
    _ => ("rol_member", "write"),
  }
}

fn parse_role(value: &str) -> Result<AdminRole, HttpError> {
  AdminRole::parse_human(value).ok_or_else(HttpError::internal)
}

fn user(row: UserRow) -> Result<UserResponse, HttpError> {
  let (id, email, role, created_at, updated_at, has_password, sso_linked) = row;
  Ok(UserResponse {
    id,
    email,
    role: parse_role(&role)?,
    created_at,
    updated_at,
    has_password,
    sso_linked,
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
  let email = common::validate_email(&request.email)?;
  let requested_role = human_role(request.role)?;
  let password = request.password.filter(|password| !password.is_empty());
  let has_password = password.is_some();
  let hash = match password {
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
  let (role_id, level) = role_access(requested_role);
  let mut tx = state.db.pool().begin().await?;
  let inserted = repository::insert(
    &mut tx,
    NewUser {
      id: &id,
      email: &email,
      password_hash: &hash,
      role: requested_role.as_str(),
      role_id,
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
  repository::set_all_projects_grant(&mut tx, &id, level, actor_id, &now).await?;
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
    serde_json::json!({"email":email,"role":requested_role.as_str(),"password":has_password}),
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
  let (old_email, old_role) = repository::account(state.db.pool(), id)
    .await?
    .ok_or_else(|| HttpError::not_found("USER_NOT_FOUND", "The user was not found."))?;
  let (actor_id, _) = actor(identity);
  // TODO(rbac): root is protected by its fixed role name here and below.
  // Check permissions instead once roles are configurable.
  if old_role == "root" && id != actor_id {
    return Err(HttpError::forbidden(
      "ROOT_PROTECTED",
      "The root administrator cannot be modified.",
    ));
  }
  if id == actor_id && request.role.is_some() {
    return Err(HttpError::forbidden(
      "SELF_ROLE_CHANGE",
      "You cannot change your own role.",
    ));
  }
  let new_role = match request.role {
    Some(value) => human_role(Some(value))?,
    None => parse_role(&old_role)?,
  };
  if old_role == "root" && new_role != AdminRole::Root {
    return Err(HttpError::forbidden(
      "ROOT_PROTECTED",
      "The root role cannot be changed.",
    ));
  }
  if old_role == "root" && request.password.is_some() {
    return Err(HttpError::forbidden(
      "ROOT_PROTECTED",
      "Use the root password change flow to change the root password.",
    ));
  }
  let email = match request.email {
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
  let result = if new_role == AdminRole::Root {
    repository::update(
      &mut tx,
      id,
      &email,
      password_hash.as_deref(),
      new_role.as_str(),
      None,
      &now,
    )
    .await
  } else {
    let (role_id, level) = role_access(new_role);
    let updated = repository::update(
      &mut tx,
      id,
      &email,
      password_hash.as_deref(),
      new_role.as_str(),
      Some(role_id),
      &now,
    )
    .await;
    match updated {
      Ok(()) if old_role != new_role.as_str() => {
        repository::set_all_projects_grant(&mut tx, id, level, actor_id, &now).await
      }
      other => other,
    }
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
    serde_json::json!({"email":email,"role":new_role.as_str()}),
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
  let target_role = repository::role(state.db.pool(), id).await?;
  match target_role.as_deref() {
    None => {
      return Err(HttpError::not_found(
        "USER_NOT_FOUND",
        "The user was not found.",
      ));
    }
    Some("root") => {
      return Err(HttpError::forbidden(
        "ROOT_PROTECTED",
        "The root administrator cannot be deleted.",
      ));
    }
    Some("admin") | Some("member") => {}
    _ => return Err(HttpError::internal()),
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
  crate::modules::sso::service::unlink(state, identity, id, &target.email).await?;
  fetch_user(state, id).await
}
