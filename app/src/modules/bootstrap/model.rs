use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapStatus {
  pub state: &'static str,
}
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapAdminRequest {
  pub setup_token: String,
  pub email: String,
  pub password: String,
}
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapAdminResponse {
  pub admin_id: String,
  pub email: String,
  pub csrf_token: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapRestoreResponse {
  pub message: String,
  pub restored: bool,
  pub key: String,
  pub size: u64,
}
