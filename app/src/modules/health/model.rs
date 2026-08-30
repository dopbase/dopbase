use serde::Serialize;
use utoipa::ToSchema;
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
  pub product: &'static str,
  pub version: &'static str,
  pub api_version: &'static str,
  pub status: &'static str,
}
