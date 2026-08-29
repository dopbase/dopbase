use crate::models::AffectedCounts;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
#[derive(Clone, Debug, Serialize, sqlx::FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentResponse {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Deserialize, ToSchema)]
pub struct CreateEnvironmentRequest {
    pub name: String,
}
#[derive(Deserialize, ToSchema)]
pub struct RenameEnvironmentRequest {
    pub name: String,
}
#[derive(Deserialize)]
pub struct EnvironmentListQuery {
    pub project: Option<String>,
}
#[derive(Deserialize)]
pub struct ResolveEnvironmentQuery {
    pub reference: String,
}
#[derive(Serialize, ToSchema)]
pub struct DeleteEnvironmentResponse {
    pub affected: AffectedCounts,
}
