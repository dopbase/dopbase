use super::model::RuntimeSnapshot;
use crate::{
  constants::errors::RUNNER_TOKEN_REQUIRED,
  http::HttpError,
  models::AuthIdentity,
  modules::{
    environments::{self, model::EnvironmentResponse},
    secrets::{self, service::RuntimeReader},
  },
  services::authx::Level,
  state::AppState,
};
use sha2::{Digest, Sha256};

pub enum RuntimeSecrets {
  /// The caller already holds this tag.
  Unchanged { tag: String },
  Fetched {
    tag: String,
    snapshot: RuntimeSnapshot,
  },
}

/// The runner token's own environment. `reference`, when given, must name
/// it; otherwise nothing is read. When `if_none_match` names the current
/// tag nothing is decrypted or audited, and no ciphertext is read, so
/// polling stays cheap.
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
  if let Some(value) = if_none_match {
    let tag = tag(
      &secrets::service::revision(state, environment_id).await?,
      &env,
    );
    if etag_matches(value, &tag) {
      return Ok(RuntimeSecrets::Unchanged { tag });
    }
  }
  let snapshot = secrets::service::snapshot(state, environment_id).await?;
  let reader = RuntimeReader {
    actor_type: identity.audit_actor_type(),
    actor_id: token_id,
    actor_label: None,
  };
  let entries =
    secrets::service::runtime_entries(state, reader, &env.project_id, environment_id, &snapshot)
      .await?;
  Ok(RuntimeSecrets::Fetched {
    tag: tag(&snapshot.revision, &env),
    snapshot: RuntimeSnapshot {
      project_id: env.project_id,
      project: env.project_name,
      environment: env.name,
      environment_id: env.id,
      revision: snapshot.revision,
      entries,
    },
  })
}

/// The ETag: the revision plus the names the response carries, so renaming
/// the project or environment also ends a run of 304s. The revision alone
/// stays what `export` returns, since imports compare against it.
fn tag(
  revision: &str,
  env: &EnvironmentResponse,
) -> String {
  let mut hash = Sha256::new();
  for part in [revision, &env.project_name, &env.name] {
    hash.update(part.as_bytes());
    hash.update([0]);
  }
  format!("{:x}", hash.finalize())
}

/// Whether an `If-None-Match` value names `tag`. It may list several tags,
/// and weak tags compare like strong ones, as a conditional GET allows.
pub fn etag_matches(
  header: &str,
  tag: &str,
) -> bool {
  header.split(',').map(str::trim).any(|candidate| {
    let candidate = candidate.strip_prefix("W/").unwrap_or(candidate);
    candidate == "*"
      || candidate
        .strip_prefix('"')
        .and_then(|candidate| candidate.strip_suffix('"'))
        .is_some_and(|candidate| candidate == tag)
  })
}
