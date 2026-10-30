use crate::{models::AffectedCounts, services::authx::Level};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
#[derive(Clone, Debug, Serialize, sqlx::FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentResponse {
  pub id: String,
  pub project_id: String,
  pub project_name: String,
  pub name: String,
  /// Reached only through a grant on the environment itself or a project
  /// manager grant.
  pub protected: bool,
  /// What the caller can do here.
  #[sqlx(skip)]
  pub level: Level,
  pub created_at: String,
  pub updated_at: String,
}
#[derive(Deserialize, ToSchema)]
pub struct CreateEnvironmentRequest {
  pub name: String,
}
#[derive(Deserialize, ToSchema)]
pub struct UpdateEnvironmentRequest {
  /// New name.
  #[serde(default)]
  pub name: Option<String>,
  /// Turn protection on or off.
  #[serde(default)]
  pub protected: Option<bool>,
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
