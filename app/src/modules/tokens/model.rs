use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
#[derive(Clone, Debug, Serialize, sqlx::FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TokenMetadata {
    pub id: String,
    pub environment_id: String,
    pub name: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub revoked_at: Option<String>,
}
#[derive(Deserialize, ToSchema)]
pub struct CreateTokenRequest {
    pub name: String,
    pub role: String,
}
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatedTokenResponse {
    pub token: TokenMetadata,
    pub plaintext_token: String,
}
