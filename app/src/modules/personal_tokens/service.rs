use super::{
  model::*,
  repository::{self, Holder, NewToken},
};
use crate::{
  constants::tokens::{PERSONAL_TOKEN_ID_PREFIX, PERSONAL_TOKEN_PREFIX},
  extractors::{
    require_browser, require_capability, require_mutation, require_person,
    require_recent_browser_auth,
  },
  http::HttpError,
  models::AuthIdentity,
  modules::{common, environments, grants::model::ScopeType, projects, users},
  services::{
    authx::{self, Capability, Level},
    token,
  },
  state::AppState,
};
use axum::http::HeaderMap;
use chrono::{Duration, Utc};
use std::collections::BTreeMap;

/// Who the caller is to the account whose tokens they touch.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Relation {
  Myself,
  /// The caller answers for this AI agent account.
  Owner,
  Other,
}

struct Target<'a> {
  id: String,
  holder: Holder,
  relation: Relation,
  actor_id: &'a str,
  actor_email: &'a str,
}

/// Resolves `me` and the account, for people signed in. Tokens never manage
/// tokens.
async fn target<'a>(
  state: &AppState,
  identity: &'a AuthIdentity,
  id: &str,
) -> Result<Target<'a>, HttpError> {
  let (actor_id, actor_email) = require_person(identity)?;
  let id = if id == "me" { actor_id } else { id };
  let holder = repository::holder(state.db.pool(), id)
    .await?
    .ok_or_else(|| HttpError::not_found("USER_NOT_FOUND", "The user was not found."))?;
  let relation = if id == actor_id {
    Relation::Myself
  } else if holder.owner_id.as_deref() == Some(actor_id) {
    Relation::Owner
  } else {
    Relation::Other
  };
  Ok(Target {
    id: id.to_owned(),
    holder,
    relation,
    actor_id,
    actor_email,
  })
}

fn invalid(
  code: &str,
  message: &str,
) -> HttpError {
  HttpError::validation(BTreeMap::from([(code.into(), message.into())]))
}

pub async fn list(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
) -> Result<Vec<PersonalTokenResponse>, HttpError> {
  let target = target(state, identity, id).await?;
  if target.relation == Relation::Other {
    require_capability(identity, Capability::UsersView)?;
  }
  Ok(repository::list(state.db.pool(), &target.id).await?)
}

/// Who may create the token: the account for itself, or the owner of an AI
/// agent, or someone who manages people and reaches at least as far as it.
async fn require_issuer(
  state: &AppState,
  identity: &AuthIdentity,
  target: &Target<'_>,
) -> Result<(), HttpError> {
  if target.holder.is_root {
    return Err(HttpError::forbidden(
      "ROOT_PROTECTED",
      "Root has no personal tokens. Sign in instead.",
    ));
  }
  match target.relation {
    Relation::Myself => {
      require_capability(identity, Capability::TokensPersonal)?;
    }
    Relation::Owner | Relation::Other => {
      require_browser(identity)?;
      if target.holder.sign_in.as_deref() != Some("token_only") {
        return Err(HttpError::forbidden(
          "TOKEN_OWNER_INVALID",
          "People create their own tokens. You can create them for AI agents only.",
        ));
      }
      if target.relation == Relation::Other {
        require_capability(identity, Capability::UsersManage)?;
        users::service::require_within(state, identity, &target.id).await?;
      }
    }
  }
  // The account's role must allow tokens at all, and it must be active.
  let mut conn = state.db.pool().acquire().await?;
  let holds = authx::load_access(&mut conn, &target.id)
    .await?
    .is_some_and(|access| access.has(Capability::TokensPersonal));
  if !holds {
    return Err(HttpError::forbidden(
      "TOKENS_NOT_ALLOWED",
      "This account's role can't hold personal tokens.",
    ));
  }
  Ok(())
}

/// Checks the scope against what the caller can see and returns its type and
/// id.
async fn resolve_scope(
  state: &AppState,
  identity: &AuthIdentity,
  scope_type: ScopeType,
  scope: Option<&str>,
) -> Result<String, HttpError> {
  let scope = scope.filter(|value| !value.is_empty());
  match (scope_type, scope) {
    (ScopeType::All, None) => Ok(String::new()),
    (ScopeType::Project, Some(reference)) => Ok(
      projects::service::authorize(state, identity, reference, Level::Metadata)
        .await?
        .id,
    ),
    (ScopeType::Environment, Some(id)) => Ok(
      environments::service::authorize(state, identity, id, Level::Metadata)
        .await?
        .id,
    ),
    _ => Err(invalid(
      "TOKEN_SCOPE_INVALID",
      "Name a project or an environment, or leave the scope out for all projects.",
    )),
  }
}

/// The expiry for a new token, within the role's limits. Personal tokens
/// always expire.
fn expiry(
  holder: &Holder,
  expires_in: Option<&str>,
) -> Result<String, HttpError> {
  let default_days = holder.token_default_ttl_days.unwrap_or(30);
  let max_days = holder.token_max_ttl_days.unwrap_or(365);
  let duration = match expires_in {
    None => Duration::days(default_days),
    Some(value) => token::expiry_duration(value)
      .map_err(|message| invalid("TOKEN_EXPIRY_INVALID", message))?
      .ok_or_else(|| {
        invalid(
          "TOKEN_EXPIRY_INVALID",
          "Personal tokens always expire. Choose a number of hours or days.",
        )
      })?,
  };
  if duration > Duration::days(max_days) {
    return Err(invalid(
      "TOKEN_EXPIRY_INVALID",
      &format!("This account's tokens last at most {max_days} days."),
    ));
  }
  Ok((Utc::now() + duration).to_rfc3339())
}

pub async fn create(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  id: &str,
  request: CreatePersonalTokenRequest,
) -> Result<CreatedPersonalTokenResponse, HttpError> {
  require_mutation(identity, headers)?;
  require_recent_browser_auth(identity)?;
  let target = target(state, identity, id).await?;
  require_issuer(state, identity, &target).await?;
  let name = request.name.trim();
  if name.is_empty() || name.chars().count() > 64 {
    return Err(invalid(
      "TOKEN_NAME_INVALID",
      "Token names must contain between 1 and 64 characters.",
    ));
  }
  if target.holder.token_scope.as_deref() != Some("data") {
    return Err(HttpError::forbidden(
      "TOKEN_SCOPE_INVALID",
      "This role's tokens can only read the audit log, which isn't available yet.",
    ));
  }
  let max_level = target
    .holder
    .token_max_level
    .as_deref()
    .and_then(Level::parse)
    .unwrap_or(Level::Metadata);
  if request.level == Level::None || request.level > max_level {
    return Err(invalid(
      "TOKEN_LEVEL_INVALID",
      &format!(
        "This account's tokens reach at most the {} level.",
        max_level.as_str()
      ),
    ));
  }
  let scope_type = request.scope_type.unwrap_or(ScopeType::All);
  let scope_id = resolve_scope(state, identity, scope_type, request.scope.as_deref()).await?;
  let expires_at = expiry(&target.holder, request.expires_in.as_deref())?;
  let token_id = token::public_id(PERSONAL_TOKEN_ID_PREFIX);
  let raw = token::generate(PERSONAL_TOKEN_PREFIX).map_err(|_| HttpError::internal())?;
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  let inserted = repository::insert(
    &mut tx,
    NewToken {
      id: &token_id,
      user_id: &target.id,
      name,
      token_hash: token::hash(&raw),
      scope_type,
      scope_id: &scope_id,
      level: request.level,
      created_by: target.actor_id,
      created_at: &now,
      expires_at: &expires_at,
    },
  )
  .await;
  if let Err(error) = inserted {
    if error.to_string().contains("UNIQUE") {
      return Err(HttpError::conflict(
        "TOKEN_ALREADY_EXISTS",
        "A token with this name already exists.",
      ));
    }
    return Err(error.into());
  }
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(target.actor_id),
    Some(target.actor_email),
    "personal_token.created",
    (scope_type == ScopeType::Project).then_some(scope_id.as_str()),
    (scope_type == ScopeType::Environment).then_some(scope_id.as_str()),
    Some("personal_token"),
    Some(&token_id),
    serde_json::json!({
      "userId": target.id,
      "user": target.holder.email,
      "name": name,
      "scopeType": scope_type.as_str(),
      "scopeId": scope_id,
      "level": request.level.as_str(),
      "expiresAt": expires_at,
    }),
  )
  .await?;
  let token = repository::find(&mut tx, &target.id, &token_id)
    .await?
    .ok_or_else(HttpError::internal)?;
  tx.commit().await?;
  Ok(CreatedPersonalTokenResponse {
    token,
    plaintext_token: raw,
  })
}

pub async fn revoke(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  id: &str,
  token_id: &str,
) -> Result<PersonalTokenResponse, HttpError> {
  require_mutation(identity, headers)?;
  let target = target(state, identity, id).await?;
  if target.relation == Relation::Other {
    require_capability(identity, Capability::TokensRevokeOthers)?;
    users::service::require_within(state, identity, &target.id).await?;
  }
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin_with("BEGIN IMMEDIATE").await?;
  let token = repository::find(&mut tx, &target.id, token_id)
    .await?
    .ok_or_else(|| HttpError::not_found("TOKEN_NOT_FOUND", "The token was not found."))?;
  if !repository::revoke(&mut tx, &target.id, token_id, &now).await? {
    return Err(HttpError::conflict(
      "TOKEN_REVOKED",
      "The token has already been revoked.",
    ));
  }
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(target.actor_id),
    Some(target.actor_email),
    "personal_token.revoked",
    None,
    None,
    Some("personal_token"),
    Some(token_id),
    serde_json::json!({"userId": target.id, "user": target.holder.email, "name": token.name}),
  )
  .await?;
  let token = repository::find(&mut tx, &target.id, token_id)
    .await?
    .ok_or_else(HttpError::internal)?;
  tx.commit().await?;
  Ok(token)
}
