use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct BackupItem {
  pub key: String,
  pub size: u64,
  pub created_at: String,
}

#[derive(Clone, Debug, Default, Deserialize, ToSchema)]
pub struct CreateBackupRequest {
  pub name: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, ToSchema)]
pub struct RestoreBackupRequest {
  pub master_key: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackupManifest {
  pub version: String,
  pub created_at: String,
  pub backup_name: String,
  pub magic: String,
}
