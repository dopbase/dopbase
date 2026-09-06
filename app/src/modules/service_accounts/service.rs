use super::model::*;
use crate::{
  extractors::{require_admin_browser, require_mutation, require_recent_browser_auth},
  http::HttpError,
  models::AuthIdentity,
  modules::common,
  services::token,
  state::AppState,
};
use axum::http::HeaderMap;
use chrono::{DateTime, Duration, Utc};

fn actor(identity: &AuthIdentity) -> (&str, &str) {
  match identity {
    AuthIdentity::Admin {
      admin_id, email, ..
    } => (admin_id, email),
    _ => ("", ""),
  }
}
fn validate_name(name: &str) -> Result<String, HttpError> {
  let value = name.trim();
  if value.len() < 2 || value.len() > 100 {
    return Err(HttpError::validation(std::collections::BTreeMap::from([(
      "NAME_INVALID".into(),
      "Names must contain between 2 and 100 characters.".into(),
    )])));
  }
  Ok(value.to_owned())
}
fn parse_expiry(value: Option<String>) -> Result<Option<String>, HttpError> {
  let Some(value) = value else {
    return Ok(Some((Utc::now() + Duration::days(30)).to_rfc3339()));
  };
  let parsed = DateTime::parse_from_rfc3339(value.trim())
    .map_err(|_| {
      HttpError::bad_request("EXPIRY_INVALID", "expiresAt must be an RFC3339 timestamp.")
    })?
    .with_timezone(&Utc);
  let now = Utc::now();
  if parsed <= now || parsed > now + Duration::days(90) {
    return Err(HttpError::bad_request(
      "EXPIRY_INVALID",
      "Agent tokens must expire within 90 days.",
    ));
  }
  Ok(Some(parsed.to_rfc3339()))
}
async fn account(
  state: &AppState,
  id: &str,
) -> Result<ServiceAccountResponse, HttpError> {
  sqlx::query_as::<_, (String, String, String, String, String)>(
    "SELECT id,name,role,created_at,updated_at FROM service_accounts WHERE id=?",
  )
  .bind(id)
  .fetch_optional(state.db.pool())
  .await?
  .map(
    |(id, name, _role, created_at, updated_at)| ServiceAccountResponse {
      id,
      name,
      role: "ai_agent",
      created_at,
      updated_at,
    },
  )
  .ok_or_else(|| {
    HttpError::not_found(
      "SERVICE_ACCOUNT_NOT_FOUND",
      "The service account was not found.",
    )
  })
}
fn map_token(
  row: (
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
  )
) -> AgentTokenResponse {
  let (id, account, name, created, expires, last, revoked) = row;
  AgentTokenResponse {
    id,
    service_account_id: account,
    name,
    created_at: created,
    expires_at: expires,
    last_used_at: last,
    revoked_at: revoked,
  }
}
pub async fn list(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<Vec<ServiceAccountResponse>, HttpError> {
  require_admin_browser(identity)?;
  let rows = sqlx::query_as::<_, (String, String, String, String, String)>(
    "SELECT id,name,role,created_at,updated_at FROM service_accounts ORDER BY name",
  )
  .fetch_all(state.db.pool())
  .await?;
  rows
    .into_iter()
    .map(|(id, name, _role, created, updated)| {
      Ok(ServiceAccountResponse {
        id,
        name,
        role: "ai_agent",
        created_at: created,
        updated_at: updated,
      })
    })
    .collect()
}
pub async fn get(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
) -> Result<ServiceAccountResponse, HttpError> {
  require_admin_browser(identity)?;
  account(state, id).await
}
pub async fn create(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  request: CreateServiceAccountRequest,
) -> Result<ServiceAccountResponse, HttpError> {
  require_mutation(identity, headers)?;
  require_recent_browser_auth(identity)?;
  require_admin_browser(identity)?;
  let name = validate_name(&request.name)?;
  let id = token::public_id("aia_");
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  if let Err(error) = sqlx::query(
    "INSERT INTO service_accounts(id,name,role,created_at,updated_at) VALUES(?,?, 'ai_agent', ?,?)",
  )
  .bind(&id)
  .bind(&name)
  .bind(&now)
  .bind(&now)
  .execute(&mut *tx)
  .await
  {
    if error.to_string().contains("UNIQUE") {
      return Err(HttpError::conflict(
        "SERVICE_ACCOUNT_EXISTS",
        "A service account with this name already exists.",
      ));
    }
    return Err(error.into());
  }
  let (aid, email) = actor(identity);
  common::audit(
    &mut *tx,
    "admin",
    Some(aid),
    Some(email),
    "service_account.created",
    None,
    None,
    Some("service_account"),
    Some(&id),
    serde_json::json!({"name":name}),
  )
  .await?;
  tx.commit().await?;
  Ok(ServiceAccountResponse {
    id,
    name,
    role: "ai_agent",
    created_at: now.clone(),
    updated_at: now,
  })
}
pub async fn delete(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  id: &str,
) -> Result<(), HttpError> {
  require_mutation(identity, headers)?;
  require_recent_browser_auth(identity)?;
  require_admin_browser(identity)?;
  let _ = account(state, id).await?;
  let (aid, email) = actor(identity);
  let mut tx = state.db.pool().begin().await?;
  sqlx::query("DELETE FROM service_accounts WHERE id=?")
    .bind(id)
    .execute(&mut *tx)
    .await?;
  common::audit(
    &mut *tx,
    "admin",
    Some(aid),
    Some(email),
    "service_account.deleted",
    None,
    None,
    Some("service_account"),
    Some(id),
    serde_json::json!({}),
  )
  .await?;
  tx.commit().await?;
  Ok(())
}
pub async fn tokens(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
) -> Result<Vec<AgentTokenResponse>, HttpError> {
  require_admin_browser(identity)?;
  let _ = account(state, id).await?;
  let rows=sqlx::query_as::<_,(String,String,String,String,Option<String>,Option<String>,Option<String>)>("SELECT id,service_account_id,name,created_at,expires_at,last_used_at,revoked_at FROM agent_tokens WHERE service_account_id=? ORDER BY created_at DESC").bind(id).fetch_all(state.db.pool()).await?;
  Ok(rows.into_iter().map(map_token).collect())
}
pub async fn create_token(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  id: &str,
  request: CreateAgentTokenRequest,
) -> Result<CreatedAgentTokenResponse, HttpError> {
  require_mutation(identity, headers)?;
  require_recent_browser_auth(identity)?;
  require_admin_browser(identity)?;
  let _ = account(state, id).await?;
  let name = validate_name(&request.name)?;
  let expires = parse_expiry(request.expires_at)?;
  let token_id = token::public_id("ait_");
  let raw = token::generate("dpa_").map_err(|_| HttpError::internal())?;
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  let result=sqlx::query("INSERT INTO agent_tokens(id,service_account_id,name,token_hash,created_at,expires_at) VALUES(?,?,?,?,?,?)").bind(&token_id).bind(id).bind(&name).bind(token::hash(&raw)).bind(&now).bind(&expires).execute(&mut *tx).await;
  if let Err(error) = result {
    if error.to_string().contains("UNIQUE") {
      return Err(HttpError::conflict(
        "TOKEN_ALREADY_EXISTS",
        "A token with this name already exists.",
      ));
    }
    return Err(error.into());
  }
  let (aid, email) = actor(identity);
  common::audit(
    &mut *tx,
    "admin",
    Some(aid),
    Some(email),
    "service_account.token_created",
    None,
    None,
    Some("agent_token"),
    Some(&token_id),
    serde_json::json!({"serviceAccountId":id,"name":name}),
  )
  .await?;
  tx.commit().await?;
  Ok(CreatedAgentTokenResponse {
    token: AgentTokenResponse {
      id: token_id,
      service_account_id: id.into(),
      name,
      created_at: now,
      expires_at: expires,
      last_used_at: None,
      revoked_at: None,
    },
    plaintext_token: raw,
  })
}
pub async fn revoke_token(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  id: &str,
  token_id: &str,
) -> Result<AgentTokenResponse, HttpError> {
  require_mutation(identity, headers)?;
  require_recent_browser_auth(identity)?;
  require_admin_browser(identity)?;
  let mut tx = state.db.pool().begin().await?;
  let now = Utc::now().to_rfc3339();
  let result=sqlx::query("UPDATE agent_tokens SET revoked_at=? WHERE id=? AND service_account_id=? AND revoked_at IS NULL").bind(&now).bind(token_id).bind(id).execute(&mut *tx).await?;
  if result.rows_affected() != 1 {
    return Err(HttpError::not_found(
      "TOKEN_NOT_FOUND",
      "The active agent token was not found.",
    ));
  }
  let (aid, email) = actor(identity);
  common::audit(
    &mut *tx,
    "admin",
    Some(aid),
    Some(email),
    "service_account.token_revoked",
    None,
    None,
    Some("agent_token"),
    Some(token_id),
    serde_json::json!({}),
  )
  .await?;
  tx.commit().await?;
  let row=sqlx::query_as::<_,(String,String,String,String,Option<String>,Option<String>,Option<String>)>("SELECT id,service_account_id,name,created_at,expires_at,last_used_at,revoked_at FROM agent_tokens WHERE id=?").bind(token_id).fetch_one(state.db.pool()).await?;
  Ok(map_token(row))
}
