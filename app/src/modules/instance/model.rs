use serde::Serialize;
use utoipa::ToSchema;
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InstanceStatus {
    pub version: &'static str,
    pub public_url: String,
    pub initialization_state: &'static str,
    pub database_health: &'static str,
    pub key_availability: &'static str,
    pub configuration_reload: &'static str,
}
