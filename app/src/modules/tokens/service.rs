use super::{model::*, repository};
use crate::modules::common;
use crate::{
  constants::{
    errors::TOKEN_SCOPE_INVALID,
    tokens::{RUNNER_TOKEN_ID_PREFIX, RUNNER_TOKEN_PREFIX},
  },
  extractors::require_admin,
  http::HttpError,
  models::AuthIdentity,
  services::token,
  state::AppState,
};
use chrono::Utc;
pub async fn list(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
) -> Result<Vec<TokenMetadata>, HttpError> {
  require_admin(identity)?;
  crate::modules::environments::service::show(state, id).await?;
  Ok(repository::list(state.db.pool(), id).await?)
}
pub async fn create(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
  request: CreateTokenRequest,
) -> Result<CreatedTokenResponse, HttpError> {
  let (admin_id, email) = require_admin(identity)?;
  if request.role != "runner" {
    return Err(HttpError::validation(std::collections::BTreeMap::from([(
      TOKEN_SCOPE_INVALID.into(),
      "Only the runner role is supported in v0.0.1.".into(),
    )])));
  }
  let name = request.name.trim();
  if name.is_empty() || name.len() > 64 {
    return Err(HttpError::validation(std::collections::BTreeMap::from([(
      "TOKEN_NAME_INVALID".into(),
      "Token names must contain between 1 and 64 characters.".into(),
    )])));
  }
  let env = crate::modules::environments::service::show(state, id).await?;
  let token_id = token::public_id(RUNNER_TOKEN_ID_PREFIX);
  let raw = token::generate(RUNNER_TOKEN_PREFIX).map_err(|_| HttpError::internal())?;
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  let result = sqlx::query(
    "INSERT INTO runner_tokens(id,environment_id,name,token_hash,created_at)VALUES(?,?,?,?,?)",
  )
  .bind(&token_id)
  .bind(id)
  .bind(name)
  .bind(token::hash(&raw))
  .bind(&now)
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
    serde_json::json!({"name":name}),
  )
  .await?;
  tx.commit().await?;
  Ok(CreatedTokenResponse {
    token: TokenMetadata {
      id: token_id,
      environment_id: id.into(),
      name: name.into(),
      created_at: now,
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
) -> Result<TokenMetadata, HttpError> {
  let (admin_id, email) = require_admin(identity)?;
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
  let mut tx = state.db.pool().begin().await?;
  sqlx::query("UPDATE runner_tokens SET revoked_at=? WHERE id=?")
    .bind(&now)
    .bind(id)
    .execute(&mut *tx)
    .await?;
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
    serde_json::json!({"name":token.name}),
  )
  .await?;
  tx.commit().await?;
  repository::find(state.db.pool(), id)
    .await?
    .ok_or_else(HttpError::internal)
}
