use crate::models::SecretInput;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Deserialize, IntoParams)]
pub struct RuntimeSecretsQuery {
  /// An environment ID or PROJECT_REF/ENVIRONMENT_NAME the caller expects.
  /// If it isn't the token's environment, the request fails before any
  /// value is read.
  pub reference: Option<String>,
}

/// The secrets of the runner token's environment.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSnapshot {
  pub project_id: String,
  pub project: String,
  pub environment: String,
  pub environment_id: String,
  /// Changes whenever a secret or the `.env` layout changes. Also sent as
  /// the `ETag`, so a client can ask with `If-None-Match` whether anything
  /// changed.
  pub revision: String,
  pub entries: Vec<SecretInput>,
}
