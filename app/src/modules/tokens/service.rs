use super::{model::*, repository};
use crate::modules::common;
use crate::{
  constants::{
    errors::TOKEN_SCOPE_INVALID,
    tokens::{
      RUNNER_TOKEN_ID_PREFIX, RUNNER_TOKEN_PREFIX, WORKSPACE_TOKEN_ID_PREFIX,
      WORKSPACE_TOKEN_PREFIX,
    },
  },
  extractors::require_project_manager,
  http::HttpError,
  models::AuthIdentity,
  services::token,
  state::AppState,
};
use chrono::Utc;

fn validate_token_name(name: &str) -> Result<&str, HttpError> {
  let name = name.trim();
  if name.is_empty() || name.len() > 64 {
    return Err(HttpError::validation(std::collections::BTreeMap::from([(
      "TOKEN_NAME_INVALID".into(),
      "Token names must contain between 1 and 64 characters.".into(),
    )])));
  }
  Ok(name)
}

fn parse_expiry(request: &CreateTokenRequest) -> Result<Option<String>, HttpError> {
  let now = Utc::now();
  let expires_at = request.expires_in.as_deref().unwrap_or("never");
  token::expiry_duration(expires_at)
    .map_err(|message| HttpError::bad_request("EXPIRY_INVALID", message))
    .map(|duration| duration.map(|value| (now + value).to_rfc3339()))
}

async fn ensure_workspace(
  pool: &sqlx::SqlitePool,
  workspace_id: &str,
) -> Result<(), HttpError> {
  let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workspaces WHERE id=?)")
    .bind(workspace_id)
    .fetch_one(pool)
    .await
    .map_err(HttpError::from)?;
  if exists {
    Ok(())
  } else {
    Err(HttpError::not_found(
      "WORKSPACE_NOT_FOUND",
      "Workspace not found.",
    ))
  }
}

pub async fn list(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
) -> Result<Vec<TokenMetadata>, HttpError> {
  require_project_manager(identity)?;
  crate::modules::environments::service::show(state, id).await?;
  Ok(repository::list(state.db.pool(), id).await?)
}

pub async fn list_workspace(
  state: &AppState,
  identity: &AuthIdentity,
  workspace_id: &str,
) -> Result<Vec<WorkspaceTokenMetadata>, HttpError> {
  require_project_manager(identity)?;
  ensure_workspace(state.db.pool(), workspace_id).await?;
  Ok(repository::list_workspace(state.db.pool(), workspace_id).await?)
}

pub async fn create(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
  request: CreateTokenRequest,
) -> Result<CreatedTokenResponse, HttpError> {
  let (admin_id, email) = require_project_manager(identity)?;
  if request.role != "runner" {
    return Err(HttpError::validation(std::collections::BTreeMap::from([(
      TOKEN_SCOPE_INVALID.into(),
      "Only the runner role is supported by this token endpoint.".into(),
    )])));
  }
  let name = validate_token_name(&request.name)?;
  let env = crate::modules::environments::service::show(state, id).await?;
  let expires_at = parse_expiry(&request)?;
  let token_id = token::public_id(RUNNER_TOKEN_ID_PREFIX);
  let raw = token::generate(RUNNER_TOKEN_PREFIX).map_err(|_| HttpError::internal())?;
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  let result = sqlx::query(
    "INSERT INTO runner_tokens(id,environment_id,name,token_hash,created_at,expires_at)VALUES(?,?,?,?,?,?)",
  )
  .bind(&token_id)
  .bind(id)
  .bind(name)
  .bind(token::hash(&raw))
  .bind(&now)
  .bind(&expires_at)
  .execute(&mut *tx)
  .await;
  if let Err(error) = result {
    if error.to_string().contains("UNIQUE") {
      return Err(HttpError::conflict(
        "TOKEN_ALREADY_EXISTS",
        "A token with this name already exists in the environment.",
      ));
    }
    return Err(HttpError::from(error));
  }
  common::audit(
    &mut *tx,
    "admin",
    Some(admin_id),
    Some(email),
    "token.created",
    Some(&env.project_id),
    Some(id),
    Some("token"),
    Some(&token_id),
    serde_json::json!({"name":name,"scope":"environment"}),
  )
  .await?;
  tx.commit().await?;
  Ok(CreatedTokenResponse {
    token: TokenMetadata {
      id: token_id,
      environment_id: id.into(),
      name: name.into(),
      created_at: now,
      expires_at,
      last_used_at: None,
      revoked_at: None,
    },
    plaintext_token: raw,
  })
}

pub async fn create_workspace(
  state: &AppState,
  identity: &AuthIdentity,
  workspace_id: &str,
  request: CreateTokenRequest,
) -> Result<CreatedWorkspaceTokenResponse, HttpError> {
  let (admin_id, email) = require_project_manager(identity)?;
  if request.role != "workspace" {
    return Err(HttpError::validation(std::collections::BTreeMap::from([(
      TOKEN_SCOPE_INVALID.into(),
      "Only the workspace role is supported by this token endpoint.".into(),
    )])));
  }
  let name = validate_token_name(&request.name)?;
  ensure_workspace(state.db.pool(), workspace_id).await?;
  let expires_at = parse_expiry(&request)?;
  let token_id = token::public_id(WORKSPACE_TOKEN_ID_PREFIX);
  let raw = token::generate(WORKSPACE_TOKEN_PREFIX).map_err(|_| HttpError::internal())?;
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  let result = sqlx::query(
    "INSERT INTO workspace_tokens(id,workspace_id,name,token_hash,created_at,expires_at)VALUES(?,?,?,?,?,?)",
  )
  .bind(&token_id)
  .bind(workspace_id)
  .bind(name)
  .bind(token::hash(&raw))
  .bind(&now)
  .bind(&expires_at)
  .execute(&mut *tx)
  .await;
  if let Err(error) = result {
    if error.to_string().contains("UNIQUE") {
      return Err(HttpError::conflict(
        "TOKEN_ALREADY_EXISTS",
        "A token with this name already exists in the workspace.",
      ));
    }
    return Err(HttpError::from(error));
  }
  common::audit(
    &mut *tx,
    "admin",
    Some(admin_id),
    Some(email),
    "token.created",
    None,
    None,
    Some("token"),
    Some(&token_id),
    serde_json::json!({"name":name,"scope":"workspace","workspaceId":workspace_id}),
  )
  .await?;
  tx.commit().await?;
  Ok(CreatedWorkspaceTokenResponse {
    token: WorkspaceTokenMetadata {
      id: token_id,
      workspace_id: workspace_id.into(),
      name: name.into(),
      created_at: now,
      expires_at,
      last_used_at: None,
      revoked_at: None,
    },
    plaintext_token: raw,
  })
}

pub async fn revoke(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
) -> Result<RevokedTokenMetadata, HttpError> {
  let (admin_id, email) = require_project_manager(identity)?;
  if repository::is_workspace_token_id(id) {
    return revoke_workspace(state, admin_id, email, id).await;
  }
  revoke_runner(state, admin_id, email, id).await
}

async fn revoke_runner(
  state: &AppState,
  admin_id: &str,
  email: &str,
  id: &str,
) -> Result<RevokedTokenMetadata, HttpError> {
  let token = repository::find(state.db.pool(), id)
    .await?
    .ok_or_else(|| HttpError::not_found("TOKEN_NOT_FOUND", "The requested token was not found."))?;
  if token.revoked_at.is_some() {
    return Err(HttpError::conflict(
      "TOKEN_REVOKED",
      "The token has already been revoked.",
    ));
  }
  let env = crate::modules::environments::service::show(state, &token.environment_id).await?;
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin_with("BEGIN IMMEDIATE").await?;
  let updated =
    sqlx::query("UPDATE runner_tokens SET revoked_at=? WHERE id=? AND revoked_at IS NULL")
      .bind(&now)
      .bind(id)
      .execute(&mut *tx)
      .await?;
  if updated.rows_affected() != 1 {
    return Err(HttpError::conflict(
      "TOKEN_REVOKED",
      "The token has already been revoked.",
    ));
  }
  common::audit(
    &mut *tx,
    "admin",
    Some(admin_id),
    Some(email),
    "token.revoked",
    Some(&env.project_id),
    Some(&env.id),
    Some("token"),
    Some(id),
    serde_json::json!({"name":token.name,"scope":"environment"}),
  )
  .await?;
  tx.commit().await?;
  repository::find(state.db.pool(), id)
    .await?
    .map(RevokedTokenMetadata::from)
    .ok_or_else(HttpError::internal)
}

async fn revoke_workspace(
  state: &AppState,
  admin_id: &str,
  email: &str,
  id: &str,
) -> Result<RevokedTokenMetadata, HttpError> {
  let token = repository::find_workspace(state.db.pool(), id)
    .await?
    .ok_or_else(|| HttpError::not_found("TOKEN_NOT_FOUND", "The requested token was not found."))?;
  if token.revoked_at.is_some() {
    return Err(HttpError::conflict(
      "TOKEN_REVOKED",
      "The token has already been revoked.",
    ));
  }
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin_with("BEGIN IMMEDIATE").await?;
  let updated =
    sqlx::query("UPDATE workspace_tokens SET revoked_at=? WHERE id=? AND revoked_at IS NULL")
      .bind(&now)
      .bind(id)
      .execute(&mut *tx)
      .await?;
  if updated.rows_affected() != 1 {
    return Err(HttpError::conflict(
      "TOKEN_REVOKED",
      "The token has already been revoked.",
    ));
  }
  common::audit(
    &mut *tx,
    "admin",
    Some(admin_id),
    Some(email),
    "token.revoked",
    None,
    None,
    Some("token"),
    Some(id),
    serde_json::json!({
      "name": token.name,
      "scope": "workspace",
      "workspaceId": token.workspace_id
    }),
  )
  .await?;
  tx.commit().await?;
  repository::find_workspace(state.db.pool(), id)
    .await?
    .map(RevokedTokenMetadata::from)
    .ok_or_else(HttpError::internal)
}
