use super::{
  model::*,
  repository::{self, Visibility},
};
use crate::{
  constants::errors::{AUTHORIZATION_DENIED, REQUEST_INVALID},
  http::HttpError,
  models::AuthIdentity,
  services::{
    audit_chain,
    authx::{self, Access, Capability, Catalog},
  },
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
/// The account reading the audit log and its role. People read it with
/// their session; tokens only when they are audit tokens.
fn reader(identity: &AuthIdentity) -> Result<(&str, &Access), HttpError> {
  match identity {
    AuthIdentity::Admin {
      admin_id, access, ..
    }
    | AuthIdentity::Token {
      admin_id,
      access,
      audit: true,
      ..
    } => Ok((admin_id, access)),
    AuthIdentity::Token { .. } => Err(HttpError::forbidden(
      AUTHORIZATION_DENIED,
      "This token reads project data, not the audit log.",
    )),
    AuthIdentity::Runner { .. } => Err(HttpError::forbidden(
      AUTHORIZATION_DENIED,
      "This operation is not available to runner tokens.",
    )),
  }
}

/// audit.read_all sees everything. audit.read sees instance events and
/// events on the projects the reader reaches. Everyone else sees their own
/// actions. Audit tokens see what their account sees, and data tokens none.
async fn visibility<'a>(
  state: &AppState,
  identity: &'a AuthIdentity,
) -> Result<Visibility<'a>, HttpError> {
  let (actor_id, access) = reader(identity)?;
  if access.has(Capability::AuditReadAll) {
    return Ok(Visibility::All);
  }
  if !access.has(Capability::AuditRead) {
    return Ok(Visibility::Own { actor_id });
  }
  let mut conn = state.db.pool().acquire().await?;
  let data = authx::account_data_access(&mut conn, actor_id, access).await?;
  let catalog = Catalog::load(&mut conn).await?;
  let projects: Vec<&String> = catalog
    .projects
    .iter()
    .filter(|id| data.sees_project(id, catalog.environments_of(id)))
    .collect();
  Ok(Visibility::Projects {
    actor_id,
    projects: serde_json::to_string(&projects).map_err(|_| HttpError::internal())?,
  })
}

pub async fn list(
  state: &AppState,
  identity: &AuthIdentity,
  query: AuditQuery,
) -> Result<AuditPage, HttpError> {
  let visibility = visibility(state, identity).await?;
  let limit = query.limit.unwrap_or(50).clamp(1, 200);
  let cursor = query.cursor.as_deref().map(decode).transpose()?;
  let mut items = repository::list(
    state.db.pool(),
    &visibility,
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
  let (_, access) = reader(identity)?;
  if !access.has(Capability::AuditVerify) {
    return Err(HttpError::forbidden(
      AUTHORIZATION_DENIED,
      "Your role does not allow this operation.",
    ));
  }
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
