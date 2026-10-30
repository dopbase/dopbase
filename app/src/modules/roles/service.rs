use super::{
  model::*,
  repository::{self, RoleRow, RoleSettings},
};
use crate::{
  constants::{errors::AUTHORIZATION_DENIED, tokens::ROLE_ID_PREFIX},
  extractors::{
    require_capability_browser, require_mutation, require_person, require_recent_browser_auth,
  },
  http::HttpError,
  models::AuthIdentity,
  modules::common,
  services::{
    authx::{Access, Capability, Level, SignIn},
    token,
  },
  state::AppState,
};
use axum::http::HeaderMap;
use chrono::Utc;
use std::collections::BTreeMap;

fn not_found() -> HttpError {
  HttpError::not_found("ROLE_NOT_FOUND", "The role was not found.")
}

fn invalid(
  code: &str,
  message: &str,
) -> HttpError {
  HttpError::validation(BTreeMap::from([(code.into(), message.into())]))
}

fn map_unique(error: sqlx::Error) -> HttpError {
  if error.to_string().contains("UNIQUE") {
    HttpError::conflict(
      "ROLE_ALREADY_EXISTS",
      "A role with this name already exists.",
    )
  } else {
    error.into()
  }
}

fn sign_in(value: &str) -> SignIn {
  if value == "token_only" {
    SignIn::TokenOnly
  } else {
    SignIn::Interactive
  }
}

fn response(
  row: RoleRow,
  capabilities: Vec<String>,
  caller: &Access,
) -> RoleResponse {
  let capabilities: Vec<Capability> = capabilities
    .iter()
    .filter_map(|value| Capability::parse(value))
    .collect();
  let max_level = Level::parse(&row.max_level).unwrap_or_default();
  let role = Access::role(
    row.id.clone(),
    row.name.clone(),
    capabilities.iter().copied(),
    max_level,
    sign_in(&row.sign_in),
  );
  RoleResponse {
    assignable: caller.includes(&role),
    id: row.id,
    name: row.name,
    description: row.description,
    builtin: row.builtin,
    capabilities,
    max_level,
    sign_in: sign_in(&row.sign_in),
    requires_owner: row.requires_owner,
    token_scope: TokenScope::parse(&row.token_scope).unwrap_or(TokenScope::None),
    token_max_level: Level::parse(&row.token_max_level).unwrap_or_default(),
    token_default_ttl_days: row.token_default_ttl_days,
    token_max_ttl_days: row.token_max_ttl_days,
    rank: row.rank,
    members: row.members,
    created_at: row.created_at,
    updated_at: row.updated_at,
  }
}

fn caller(identity: &AuthIdentity) -> Result<&Access, HttpError> {
  match identity {
    AuthIdentity::Admin { access, .. } => Ok(access),
    _ => Err(HttpError::internal()),
  }
}

/// People who see users or manage roles see every role, and whether they
/// may give it to someone.
pub async fn list(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<Vec<RoleResponse>, HttpError> {
  require_person(identity)?;
  let access = caller(identity)?;
  if !access.has(Capability::UsersView) && !access.has(Capability::RolesManage) {
    return Err(HttpError::forbidden(
      AUTHORIZATION_DENIED,
      "Your role does not allow this operation.",
    ));
  }
  let rows = repository::list(state.db.pool()).await?;
  let mut conn = state.db.pool().acquire().await?;
  let mut roles = Vec::with_capacity(rows.len());
  for row in rows {
    let capabilities = repository::capabilities(&mut conn, &row.id).await?;
    roles.push(response(row, capabilities, access));
  }
  Ok(roles)
}

/// Everything a custom role is saved with, checked as a whole.
struct Settings {
  name: String,
  description: String,
  capabilities: Vec<Capability>,
  max_level: Level,
  sign_in: SignIn,
  token_scope: TokenScope,
  token_max_level: Level,
  token_default_ttl_days: i64,
  token_max_ttl_days: i64,
  rank: i64,
}

impl Settings {
  fn validate(mut self) -> Result<Self, HttpError> {
    self.name = self.name.trim().to_owned();
    if self.name.is_empty() || self.name.chars().count() > 64 {
      return Err(invalid(
        "ROLE_NAME_INVALID",
        "Role names must contain between 1 and 64 characters.",
      ));
    }
    self.description = self.description.trim().to_owned();
    if self.description.chars().count() > 256 {
      return Err(invalid(
        "ROLE_DESCRIPTION_INVALID",
        "Descriptions may contain at most 256 characters.",
      ));
    }
    if let Some(capability) = self
      .capabilities
      .iter()
      .find(|capability| Capability::ROOT_ONLY.contains(capability))
    {
      return Err(invalid(
        "ROLE_CAPABILITY_INVALID",
        &format!(
          "{} belongs to root and can't be given to a role.",
          capability.as_str()
        ),
      ));
    }
    self.capabilities.sort();
    self.capabilities.dedup();
    if self.max_level == Level::None {
      return Err(invalid(
        "ROLE_LEVEL_INVALID",
        "Choose how far into project data the role can reach, at least metadata.",
      ));
    }
    let tokens = self.capabilities.contains(&Capability::TokensPersonal);
    if tokens != (self.token_scope != TokenScope::None) {
      return Err(invalid(
        "ROLE_TOKENS_INVALID",
        "A role with tokens.personal needs a token scope, and one without it has none.",
      ));
    }
    if self.sign_in == SignIn::TokenOnly && !tokens {
      return Err(invalid(
        "ROLE_TOKENS_INVALID",
        "Accounts that sign in with tokens only need tokens.personal.",
      ));
    }
    if self.token_scope == TokenScope::Audit
      && !self.capabilities.contains(&Capability::AuditRead)
      && !self.capabilities.contains(&Capability::AuditReadAll)
    {
      return Err(invalid(
        "ROLE_TOKENS_INVALID",
        "Audit tokens need audit.read or audit.read_all.",
      ));
    }
    if self.token_scope != TokenScope::Data {
      // The column needs a value; audit tokens and roles without tokens
      // never use it.
      self.token_max_level = Level::Metadata;
    }
    if !(Level::Metadata..=Level::Write).contains(&self.token_max_level)
      || self.token_max_level > self.max_level
    {
      return Err(invalid(
        "ROLE_TOKENS_INVALID",
        "Token levels go from metadata to write, and no deeper than the role.",
      ));
    }
    if !(1..=1095).contains(&self.token_max_ttl_days)
      || !(1..=self.token_max_ttl_days).contains(&self.token_default_ttl_days)
    {
      return Err(invalid(
        "ROLE_TOKENS_INVALID",
        "Token lifetimes go from 1 to 1095 days, with the default no longer than the limit.",
      ));
    }
    if !(1..=10_000).contains(&self.rank) {
      return Err(invalid("ROLE_RANK_INVALID", "Rank goes from 1 to 10000."));
    }
    Ok(self)
  }

  fn row(&self) -> RoleSettings<'_> {
    RoleSettings {
      name: &self.name,
      description: &self.description,
      max_level: self.max_level.as_str(),
      sign_in: match self.sign_in {
        SignIn::Interactive => "interactive",
        SignIn::TokenOnly => "token_only",
      },
      // Accounts that never sign in always answer to someone.
      requires_owner: self.sign_in == SignIn::TokenOnly,
      token_scope: self.token_scope.as_str(),
      token_max_level: self.token_max_level.as_str(),
      token_default_ttl_days: self.token_default_ttl_days,
      token_max_ttl_days: self.token_max_ttl_days,
      rank: self.rank,
    }
  }

  fn capability_names(&self) -> Vec<&'static str> {
    self
      .capabilities
      .iter()
      .map(|capability| capability.as_str())
      .collect()
  }

  fn audit(&self) -> serde_json::Value {
    serde_json::json!({
      "name": self.name,
      "capabilities": self.capability_names(),
      "maxLevel": self.max_level.as_str(),
      "tokenScope": self.token_scope.as_str(),
      "tokenMaxLevel": self.token_max_level.as_str(),
    })
  }
}

fn require_root_change(
  identity: &AuthIdentity,
  headers: &HeaderMap,
) -> Result<(String, String), HttpError> {
  require_mutation(identity, headers)?;
  require_recent_browser_auth(identity)?;
  let (id, email) = require_capability_browser(identity, Capability::RolesManage)?;
  Ok((id.to_owned(), email.to_owned()))
}

async fn fetch(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
) -> Result<RoleResponse, HttpError> {
  let mut conn = state.db.pool().acquire().await?;
  let row = repository::find(&mut conn, id)
    .await?
    .ok_or_else(not_found)?;
  let capabilities = repository::capabilities(&mut conn, id).await?;
  Ok(response(row, capabilities, caller(identity)?))
}

pub async fn create(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  request: CreateRoleRequest,
) -> Result<RoleResponse, HttpError> {
  let (actor_id, actor_email) = require_root_change(identity, headers)?;
  let tokens = request.capabilities.contains(&Capability::TokensPersonal);
  let settings = Settings {
    name: request.name,
    description: request.description.unwrap_or_default(),
    capabilities: request.capabilities,
    max_level: request.max_level,
    sign_in: request.sign_in.unwrap_or(SignIn::Interactive),
    token_scope: request.token_scope.unwrap_or(if tokens {
      TokenScope::Data
    } else {
      TokenScope::None
    }),
    token_max_level: request
      .token_max_level
      .unwrap_or_else(|| request.max_level.min(Level::Write)),
    token_default_ttl_days: request.token_default_ttl_days.unwrap_or(30),
    token_max_ttl_days: request.token_max_ttl_days.unwrap_or(365),
    rank: request.rank.unwrap_or(100),
  }
  .validate()?;
  let id = token::public_id(ROLE_ID_PREFIX);
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  repository::insert(&mut tx, &id, &settings.row(), &now)
    .await
    .map_err(map_unique)?;
  repository::set_capabilities(&mut tx, &id, &settings.capability_names()).await?;
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(&actor_id),
    Some(&actor_email),
    "role.created",
    None,
    None,
    Some("role"),
    Some(&id),
    settings.audit(),
  )
  .await?;
  tx.commit().await?;
  fetch(state, identity, &id).await
}

fn builtin() -> HttpError {
  HttpError::forbidden(
    "ROLE_BUILTIN",
    "Built-in roles can't change. Create a custom role with the settings you need.",
  )
}

pub async fn update(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  id: &str,
  request: UpdateRoleRequest,
) -> Result<RoleResponse, HttpError> {
  let (actor_id, actor_email) = require_root_change(identity, headers)?;
  let current = fetch(state, identity, id).await?;
  if current.builtin {
    return Err(builtin());
  }
  let max_level = request.max_level.unwrap_or(current.max_level);
  let settings = Settings {
    name: request.name.unwrap_or(current.name),
    description: request.description.unwrap_or(current.description),
    capabilities: request.capabilities.unwrap_or(current.capabilities),
    max_level,
    sign_in: current.sign_in,
    token_scope: request.token_scope.unwrap_or(current.token_scope),
    // Lowering the role's ceiling lowers its tokens' with it.
    token_max_level: request
      .token_max_level
      .unwrap_or_else(|| current.token_max_level.min(max_level)),
    token_default_ttl_days: request
      .token_default_ttl_days
      .unwrap_or(current.token_default_ttl_days),
    token_max_ttl_days: request
      .token_max_ttl_days
      .unwrap_or(current.token_max_ttl_days),
    rank: request.rank.unwrap_or(current.rank),
  }
  .validate()?;
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin_with("BEGIN IMMEDIATE").await?;
  repository::update(&mut tx, id, &settings.row(), &now)
    .await
    .map_err(map_unique)?;
  repository::set_capabilities(&mut tx, id, &settings.capability_names()).await?;
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(&actor_id),
    Some(&actor_email),
    "role.updated",
    None,
    None,
    Some("role"),
    Some(id),
    settings.audit(),
  )
  .await?;
  tx.commit().await?;
  fetch(state, identity, id).await
}

pub async fn delete(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  id: &str,
) -> Result<(), HttpError> {
  let (actor_id, actor_email) = require_root_change(identity, headers)?;
  let current = fetch(state, identity, id).await?;
  if current.builtin {
    return Err(builtin());
  }
  if current.members > 0 {
    return Err(HttpError::conflict(
      "ROLE_IN_USE",
      "Accounts still hold this role. Give them another role first.",
    ));
  }
  let mut tx = state.db.pool().begin_with("BEGIN IMMEDIATE").await?;
  repository::delete(&mut tx, id).await?;
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(&actor_id),
    Some(&actor_email),
    "role.deleted",
    None,
    None,
    Some("role"),
    Some(id),
    serde_json::json!({"name": current.name}),
  )
  .await?;
  tx.commit().await?;
  Ok(())
}
