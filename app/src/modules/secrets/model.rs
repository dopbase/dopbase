use crate::models::SecretInput;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
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
}
#[derive(Serialize, ToSchema)]
pub struct ExportSecretsResponse {
    pub entries: Vec<SecretInput>,
}
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSecretsResponse {
    pub project: String,
    pub environment: String,
    pub environment_id: String,
    pub entries: Vec<SecretInput>,
}
