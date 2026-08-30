use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};
#[derive(Clone, Debug, Serialize, sqlx::FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuditEvent {
  pub id: String,
  pub actor_type: String,
  pub actor_id: Option<String>,
  pub actor_label: Option<String>,
  pub action: String,
  pub project_id: Option<String>,
  pub environment_id: Option<String>,
  pub resource_type: Option<String>,
  pub resource_id: Option<String>,
  #[sqlx(json)]
  pub metadata: Value,
  pub created_at: String,
}
#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in=Query)]
#[serde(rename_all = "camelCase")]
pub struct AuditQuery {
  pub cursor: Option<String>,
  pub limit: Option<u32>,
  pub action: Option<String>,
  pub project_id: Option<String>,
  pub environment_id: Option<String>,
  pub actor: Option<String>,
}
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuditPage {
  pub items: Vec<AuditEvent>,
  pub next_cursor: Option<String>,
}
