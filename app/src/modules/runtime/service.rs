use super::model::RuntimeSnapshot;
use crate::{
  constants::errors::RUNNER_TOKEN_REQUIRED,
  http::HttpError,
  models::AuthIdentity,
  modules::{common, environments, secrets},
  services::authx::Level,
  state::AppState,
};

pub enum RuntimeSecrets {
  /// The caller already holds this revision.
  Unchanged {
    revision: String,
  },
  Fetched(RuntimeSnapshot),
}

/// The runner token's own environment. `reference`, when given, must name
/// it; otherwise nothing is read. When `if_none_match` names the current
/// revision nothing is decrypted or audited, so polling stays cheap.
pub async fn secrets(
  state: &AppState,
  identity: &AuthIdentity,
  reference: Option<&str>,
  if_none_match: Option<&str>,
) -> Result<RuntimeSecrets, HttpError> {
  let AuthIdentity::Runner {
    token_id,
    environment_id,
  } = identity
  else {
    return Err(HttpError::forbidden(
      RUNNER_TOKEN_REQUIRED,
      "This endpoint needs a runner token.",
    ));
  };
  let env = environments::service::authorize(state, identity, environment_id, Level::Read).await?;
  if let Some(reference) = reference
    && environments::service::resolve_id(state, reference)
      .await?
      .as_ref()
      != Some(environment_id)
  {
    // Name neither the token's environment nor whether `reference` exists:
    // the refusal often ends up in CI logs.
    return Err(environments::service::scope_refused(reference));
  }
  let snapshot = secrets::service::snapshot(state, environment_id).await?;
  if if_none_match.is_some_and(|value| etag_matches(value, &snapshot.revision)) {
    return Ok(RuntimeSecrets::Unchanged {
      revision: snapshot.revision,
    });
  }
  let entries = secrets::service::decrypt_rows(state, environment_id, &snapshot.rows)?;
  common::audit_standalone(
    state.db.pool(),
    identity.audit_actor_type(),
    Some(token_id),
    None,
    "secret.runtime_accessed",
    Some(&env.project_id),
    Some(environment_id),
    Some("environment"),
    Some(environment_id),
    serde_json::json!({"count":entries.len(),"revision":snapshot.revision}),
  )
  .await?;
  Ok(RuntimeSecrets::Fetched(RuntimeSnapshot {
    project_id: env.project_id,
    project: env.project_name,
    environment: env.name,
    environment_id: env.id,
    revision: snapshot.revision,
    entries,
  }))
}

/// Whether an `If-None-Match` value names `revision`. It may list several
/// tags, and weak tags compare like strong ones, as a conditional GET allows.
pub fn etag_matches(
  header: &str,
  revision: &str,
) -> bool {
  header.split(',').map(str::trim).any(|tag| {
    let tag = tag.strip_prefix("W/").unwrap_or(tag);
    tag == "*"
      || tag
        .strip_prefix('"')
        .and_then(|tag| tag.strip_suffix('"'))
        .is_some_and(|tag| tag == revision)
  })
}
