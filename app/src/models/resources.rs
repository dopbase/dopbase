use serde::Serialize;
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AffectedCounts {
    pub projects: u64,
    pub environments: u64,
    pub secrets: u64,
    pub tokens: u64,
}
