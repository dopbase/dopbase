use super::{model::*, repository};
use crate::{
  constants::errors::REQUEST_INVALID,
  extractors::require_capability,
  http::HttpError,
  models::AuthIdentity,
  services::{audit_chain, authx::Capability},
  state::AppState,
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
  require_capability(identity, Capability::AuditRead)?;
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

/// Recompute every audit record's hash in sequence order, and compare the
/// log with an outside anchor when one is given.
pub async fn verify(
  state: &AppState,
  identity: &AuthIdentity,
  query: ChainVerifyQuery,
) -> Result<ChainVerifyReport, HttpError> {
  require_capability(identity, Capability::AuditVerify)?;
  let anchor = match (query.anchor_seq, query.anchor_hash) {
    (None, None) => None,
    (Some(seq), Some(hash)) if seq > 0 && !hash.is_empty() => {
      Some(audit_chain::Anchor { seq, hash })
    }
    _ => {
      return Err(HttpError::bad_request(
        REQUEST_INVALID,
        "anchorSeq and anchorHash must be given together, with a positive sequence.",
      ));
    }
  };
  let report = audit_chain::verify_against(state.db.pool(), anchor.as_ref()).await?;
  // Every verification also leaves the head in the server log.
  audit_chain::log_checkpoint(state.db.pool()).await;
  Ok(report.into())
}
