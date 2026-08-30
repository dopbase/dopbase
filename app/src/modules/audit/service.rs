use super::{model::*, repository};
use crate::{
  constants::errors::REQUEST_INVALID, extractors::require_admin, http::HttpError,
  models::AuthIdentity, state::AppState,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
fn decode(value: &str) -> Result<(String, String), HttpError> {
  let bytes = URL_SAFE_NO_PAD
    .decode(value)
    .map_err(|_| HttpError::bad_request(REQUEST_INVALID, "The audit cursor is invalid."))?;
  let value = String::from_utf8(bytes)
    .map_err(|_| HttpError::bad_request(REQUEST_INVALID, "The audit cursor is invalid."))?;
  let (time, id) = value
    .split_once('|')
    .ok_or_else(|| HttpError::bad_request(REQUEST_INVALID, "The audit cursor is invalid."))?;
  Ok((time.into(), id.into()))
}
pub async fn list(
  state: &AppState,
  identity: &AuthIdentity,
  query: AuditQuery,
) -> Result<AuditPage, HttpError> {
  require_admin(identity)?;
  let limit = query.limit.unwrap_or(50).clamp(1, 200);
  let cursor = query.cursor.as_deref().map(decode).transpose()?;
  let mut items = repository::list(
    state.db.pool(),
    cursor.as_ref().map(|(a, b)| (a.as_str(), b.as_str())),
    limit + 1,
    query.action.as_deref(),
    query.project_id.as_deref(),
    query.environment_id.as_deref(),
    query.actor.as_deref(),
  )
  .await?;
  let next_cursor = if items.len() > limit as usize {
    items.truncate(limit as usize);
    items
      .last()
      .map(|item| URL_SAFE_NO_PAD.encode(format!("{}|{}", item.created_at, item.id)))
  } else {
    None
  };
  Ok(AuditPage { items, next_cursor })
}
