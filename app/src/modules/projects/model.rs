use crate::models::{AffectedCounts, SecretInput};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
#[derive(Clone, Debug, Serialize, sqlx::FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectResponse {
  pub id: String,
  pub name: String,
  pub created_at: String,
  pub updated_at: String,
}
#[derive(Deserialize, ToSchema)]
pub struct CreateProjectRequest {
  pub name: String,
}
#[derive(Deserialize, ToSchema)]
pub struct RenameProjectRequest {
  pub name: String,
}
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InitProjectRequest {
  pub project_name: String,
  pub environment_name: String,
  pub entries: Vec<SecretInput>,
}
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InitProjectResponse {
  pub project: ProjectResponse,
  pub environment_id: String,
  pub secret_count: usize,
}
#[derive(Serialize, ToSchema)]
pub struct DeleteProjectResponse {
  pub affected: AffectedCounts,
}
