use crate::{models::SecretInput, services::authx::Level};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
#[derive(Clone, Debug, Serialize, sqlx::FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SecretMetadata {
  pub key: String,
  pub version: i64,
  pub created_at: String,
  pub updated_at: String,
}
#[derive(Deserialize, ToSchema)]
pub struct SetSecretRequest {
  pub value: String,
}
#[derive(Serialize, ToSchema)]
pub struct RevealedSecret {
  pub key: String,
  pub value: String,
  pub version: i64,
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

/// One stored version of a secret. Never contains the value.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SecretVersion {
  pub version: i64,
  /// `created`, `updated`, `imported`, `rolled_back`, or `legacy` for a
  /// value stored before version history existed.
  pub change_type: String,
  /// The version whose value a rollback restored.
  pub source_version: Option<i64>,
  pub actor_type: String,
  pub actor_label: Option<String>,
  pub created_at: String,
  /// True for the version the secret currently holds.
  pub current: bool,
  /// False once retention removed the encrypted value. The metadata stays.
  pub restorable: bool,
}
#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in=Query)]
#[serde(rename_all = "camelCase")]
pub struct SecretVersionQuery {
  /// Page size, 1 to 200. Defaults to 50.
  pub limit: Option<u32>,
  /// Return only versions older than this one.
  pub before: Option<i64>,
}
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SecretVersionPage {
  /// Newest first.
  pub items: Vec<SecretVersion>,
  /// Pass as `before` to fetch the next page. Absent on the last page.
  pub next_before: Option<i64>,
  /// How many of the newest versions keep their value.
  pub retained_versions: u32,
}
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RollbackSecretRequest {
  /// The version whose value becomes current again.
  pub version: i64,
  /// The current version the caller saw. When it no longer matches, the
  /// rollback is refused with `SECRET_VERSION_CONFLICT`.
  #[serde(default)]
  pub expected_version: Option<i64>,
}
#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct CompareQuery {
  /// Comma-separated ids of up to three environments in the base's
  /// project. Each is read against the base.
  pub with: Option<String>,
}
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompareEnvironment {
  pub id: String,
  pub name: String,
  pub protected: bool,
  /// What the caller can do in this environment.
  pub level: Level,
  /// False when the caller lacks Read here or on the base, so only which
  /// keys exist is known.
  pub values_compared: bool,
}
/// One environment's state for one key.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompareCell {
  /// The current version, or absent when the key isn't set there.
  pub version: Option<i64>,
  /// Whether the value matches the base. Absent when either side is
  /// missing or values weren't compared.
  pub same_as_base: Option<bool>,
}
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompareKey {
  pub key: String,
  /// The base's current version, or absent when the base lacks the key.
  pub base: Option<i64>,
  /// One cell per compared environment, in request order.
  pub others: Vec<CompareCell>,
}
/// Every key in the base or any compared environment. Never contains
/// values or anything derived from them beyond `sameAsBase`.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompareResponse {
  pub base: CompareEnvironment,
  pub environments: Vec<CompareEnvironment>,
  pub keys: Vec<CompareKey>,
}
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CopySecretRequest {
  /// The environment to copy the value from, in the same project.
  pub from: String,
  /// The target's current version the caller saw, or absent when the
  /// target didn't have the key. A mismatch is refused with
  /// `SECRET_VERSION_CONFLICT`.
  #[serde(default)]
  pub expected_version: Option<i64>,
}
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RollbackSecretResponse {
  pub key: String,
  /// The new current version.
  pub version: i64,
  /// The version whose value was restored.
  pub source_version: i64,
  /// The version that was current before the rollback.
  pub previous_version: i64,
  pub created_at: String,
  pub updated_at: String,
}
