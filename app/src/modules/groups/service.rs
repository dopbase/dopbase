use super::{model::*, repository};
use crate::{
  constants::tokens::GROUP_ID_PREFIX,
  extractors::{
    require_browser, require_capability, require_capability_browser, require_mutation,
    require_recent_browser_auth,
  },
  http::HttpError,
  models::AuthIdentity,
  modules::{
    common,
    grants::{self, model::ScopeType},
  },
  services::{
    authx::{self, Capability, Level},
    token,
  },
  state::AppState,
};
use axum::http::HeaderMap;
use chrono::Utc;
use std::collections::BTreeMap;

fn not_found() -> HttpError {
  HttpError::not_found("GROUP_NOT_FOUND", "The group was not found.")
}

fn map_unique(error: sqlx::Error) -> HttpError {
  if error.to_string().contains("UNIQUE") {
    HttpError::conflict(
      "GROUP_ALREADY_EXISTS",
      "A group with this name already exists.",
    )
  } else {
    error.into()
  }
}

fn validate_name(value: &str) -> Result<String, HttpError> {
  let name = value.trim();
  if name.is_empty() || name.chars().count() > 64 {
    return Err(HttpError::validation(BTreeMap::from([(
      "GROUP_NAME_INVALID".into(),
      "Group names must contain between 1 and 64 characters.".into(),
    )])));
  }
  Ok(name.to_owned())
}

fn validate_description(value: &str) -> Result<String, HttpError> {
  let description = value.trim();
  if description.chars().count() > 256 {
    return Err(HttpError::validation(BTreeMap::from([(
      "GROUP_DESCRIPTION_INVALID".into(),
      "Descriptions may contain at most 256 characters.".into(),
    )])));
  }
  Ok(description.to_owned())
}

/// Seeing groups needs users.view or groups.manage, from the Admin UI.
fn require_view(identity: &AuthIdentity) -> Result<(), HttpError> {
  require_browser(identity)?;
  match identity {
    AuthIdentity::Admin { access, .. } if access.has(Capability::GroupsManage) => Ok(()),
    _ => require_capability(identity, Capability::UsersView).map(|_| ()),
  }
}

/// Changing a group changes what its members reach, so a person may change
/// it only when every grant it holds is on a project they manage. Grants on
/// all projects leave the group to root.
async fn require_manages(
  identity: &AuthIdentity,
  conn: &mut sqlx::SqliteConnection,
  group_id: &str,
) -> Result<(), HttpError> {
  let AuthIdentity::Admin {
    admin_id, access, ..
  } = identity
  else {
    return Err(HttpError::internal());
  };
  if access.is_root {
    return Ok(());
  }
  let grants = grants::repository::for_group(conn, group_id).await?;
  let data = authx::account_data_access(conn, admin_id, access).await?;
  let within = grants.iter().all(|grant| {
    grant.scope_type != ScopeType::All && data.project(&grant.project_id) == Level::ProjectManager
  });
  if !within {
    return Err(HttpError::forbidden(
      "ACCESS_EXCEEDS_YOURS",
      "This group has access you don't manage, so you can't change it.",
    ));
  }
  Ok(())
}

async fn detail(
  conn: &mut sqlx::SqliteConnection,
  id: &str,
) -> Result<GroupDetailResponse, HttpError> {
  let group = repository::find(conn, id).await?.ok_or_else(not_found)?;
  Ok(GroupDetailResponse {
    members: repository::members(conn, id).await?,
    grants: grants::repository::for_group(conn, id).await?,
    group,
  })
}

pub async fn list(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<Vec<GroupResponse>, HttpError> {
  require_view(identity)?;
  Ok(repository::list(state.db.pool()).await?)
}

pub async fn get(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
) -> Result<GroupDetailResponse, HttpError> {
  require_view(identity)?;
  let mut conn = state.db.pool().acquire().await?;
  detail(&mut conn, id).await
}

fn require_change(
  identity: &AuthIdentity,
  headers: &HeaderMap,
) -> Result<(String, String), HttpError> {
  require_mutation(identity, headers)?;
  require_recent_browser_auth(identity)?;
  let (id, email) = require_capability_browser(identity, Capability::GroupsManage)?;
  Ok((id.to_owned(), email.to_owned()))
}

pub async fn create(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  request: CreateGroupRequest,
) -> Result<GroupDetailResponse, HttpError> {
  let (actor_id, actor_email) = require_change(identity, headers)?;
  let name = validate_name(&request.name)?;
  let description = validate_description(request.description.as_deref().unwrap_or(""))?;
  let id = token::public_id(GROUP_ID_PREFIX);
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  repository::insert(&mut tx, &id, &name, &description, &actor_id, &now)
    .await
    .map_err(map_unique)?;
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(&actor_id),
    Some(&actor_email),
    "group.created",
    None,
    None,
    Some("group"),
    Some(&id),
    serde_json::json!({"name":name}),
  )
  .await?;
  let group = detail(&mut tx, &id).await?;
  tx.commit().await?;
  Ok(group)
}

pub async fn update(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  id: &str,
  request: UpdateGroupRequest,
) -> Result<GroupDetailResponse, HttpError> {
  let (actor_id, actor_email) = require_change(identity, headers)?;
  let mut tx = state.db.pool().begin_with("BEGIN IMMEDIATE").await?;
  let group = repository::find(&mut tx, id).await?.ok_or_else(not_found)?;
  require_manages(identity, &mut tx, id).await?;
  let name = match &request.name {
    Some(value) => validate_name(value)?,
    None => group.name.clone(),
  };
  let description = match &request.description {
    Some(value) => validate_description(value)?,
    None => group.description.clone(),
  };
  let now = Utc::now().to_rfc3339();
  repository::update(&mut tx, id, &name, &description, &now)
    .await
    .map_err(map_unique)?;
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(&actor_id),
    Some(&actor_email),
    "group.updated",
    None,
    None,
    Some("group"),
    Some(id),
    serde_json::json!({"oldName":group.name,"name":name}),
  )
  .await?;
  let group = detail(&mut tx, id).await?;
  tx.commit().await?;
  Ok(group)
}

pub async fn delete(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  id: &str,
) -> Result<(), HttpError> {
  let (actor_id, actor_email) = require_change(identity, headers)?;
  let mut tx = state.db.pool().begin_with("BEGIN IMMEDIATE").await?;
  let group = repository::find(&mut tx, id).await?.ok_or_else(not_found)?;
  require_manages(identity, &mut tx, id).await?;
  repository::delete(&mut tx, id).await?;
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(&actor_id),
    Some(&actor_email),
    "group.deleted",
    None,
    None,
    Some("group"),
    Some(id),
    serde_json::json!({"name":group.name,"members":group.member_count}),
  )
  .await?;
  tx.commit().await?;
  Ok(())
}

pub async fn add_member(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  id: &str,
  user_id: &str,
) -> Result<GroupDetailResponse, HttpError> {
  let (actor_id, actor_email) = require_change(identity, headers)?;
  let mut tx = state.db.pool().begin_with("BEGIN IMMEDIATE").await?;
  let group = repository::find(&mut tx, id).await?.ok_or_else(not_found)?;
  require_manages(identity, &mut tx, id).await?;
  let (email, is_root) = repository::account(&mut tx, user_id)
    .await?
    .ok_or_else(|| HttpError::not_found("USER_NOT_FOUND", "The user was not found."))?;
  if is_root {
    return Err(HttpError::forbidden(
      "ROOT_PROTECTED",
      "Root reaches everything and can't join groups.",
    ));
  }
  let now = Utc::now().to_rfc3339();
  if repository::add_member(&mut tx, id, user_id, &actor_id, &now).await? {
    common::audit(
      &mut tx,
      identity.audit_actor_type(),
      Some(&actor_id),
      Some(&actor_email),
      "group_member.added",
      None,
      None,
      Some("group"),
      Some(id),
      serde_json::json!({"group":group.name,"userId":user_id,"email":email}),
    )
    .await?;
  }
  let group = detail(&mut tx, id).await?;
  tx.commit().await?;
  Ok(group)
}

pub async fn remove_member(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  id: &str,
  user_id: &str,
) -> Result<GroupDetailResponse, HttpError> {
  let (actor_id, actor_email) = require_change(identity, headers)?;
  let mut tx = state.db.pool().begin_with("BEGIN IMMEDIATE").await?;
  let group = repository::find(&mut tx, id).await?.ok_or_else(not_found)?;
  require_manages(identity, &mut tx, id).await?;
  if !repository::remove_member(&mut tx, id, user_id).await? {
    return Err(HttpError::not_found(
      "GROUP_MEMBER_NOT_FOUND",
      "The user is not in this group.",
    ));
  }
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(&actor_id),
    Some(&actor_email),
    "group_member.removed",
    None,
    None,
    Some("group"),
    Some(id),
    serde_json::json!({"group":group.name,"userId":user_id}),
  )
  .await?;
  let group = detail(&mut tx, id).await?;
  tx.commit().await?;
  Ok(group)
}
