use crate::models::SecretInput;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
#[derive(Clone, Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SecretMetadata {
  pub key: String,
  pub version: i64,
  pub created_at: String,
  pub updated_at: String,
  /// Version of the previous value, present while a rotation grace period
  /// keeps the old credential valid.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub previous_version: Option<i64>,
  /// When the previous value stops being served.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub previous_expires_at: Option<String>,
}
#[derive(Deserialize, ToSchema)]
pub struct SetSecretRequest {
  pub value: String,
}
/// Stores a new value in the free slot of an existing secret.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RotateSecretRequest {
  pub value: String,
  /// Grace period in seconds during which the previous value stays valid.
  /// Omitted uses 24 hours; `0` retires the previous value immediately.
  #[serde(default)]
  pub grace_seconds: Option<i64>,
}
/// Verification tooling can ask for both values of a rotation.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RevealQuery {
  #[serde(default)]
  pub include_previous: bool,
}
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreviousRevealedValue {
  pub value: String,
  pub version: i64,
  /// When this value stops being served.
  pub expires_at: String,
}
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RevealedSecret {
  pub key: String,
  pub value: String,
  pub version: i64,
  /// The value replaced by the latest rotation, while its grace period runs.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub previous: Option<PreviousRevealedValue>,
}
#[derive(Clone, Copy, Debug, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ImportMode {
  Merge,
  Replace,
}
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImportSecretsRequest {
  pub mode: ImportMode,
  #[serde(default)]
  pub dry_run: bool,
  pub entries: Vec<SecretInput>,
  /// `.env` editor layout (comments, ordering, empty `KEY=` slots with no
  /// values). When present, it is persisted alongside the import so the
  /// editor view survives reloads.
  #[serde(default)]
  pub env_layout: Option<String>,
  /// Collection revision returned by export or a dry run. Preview and apply
  /// fail if the environment has changed since that snapshot.
  #[serde(default)]
  pub expected_revision: Option<String>,
}
/// The stored `.env` editor layout for one environment. Contains no secret
/// values — comments, blank lines, ordering, and empty `KEY=` slots only.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnvLayoutResponse {
  pub layout: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImportSecretsResponse {
  pub added_keys: Vec<String>,
  pub updated_keys: Vec<String>,
  pub unchanged_keys: Vec<String>,
  pub deleted_keys: Vec<String>,
  pub dry_run: bool,
  pub revision: String,
}
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExportSecretsResponse {
  pub entries: Vec<SecretInput>,
  /// Revision of the same snapshot as the exported values and layout.
  pub revision: String,
  /// Comments and empty key slots. Contains no values.
  pub env_layout: Option<String>,
}
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSecretsResponse {
  pub project: String,
  pub environment: String,
  pub environment_id: String,
  pub entries: Vec<SecretInput>,
}
