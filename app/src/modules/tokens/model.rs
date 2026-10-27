use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, sqlx::FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TokenMetadata {
  pub id: String,
  pub environment_id: String,
  pub name: String,
  pub created_at: String,
  pub expires_at: Option<String>,
  pub last_used_at: Option<String>,
  pub revoked_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTokenMetadata {
  pub id: String,
  pub workspace_id: String,
  pub name: String,
  pub created_at: String,
  pub expires_at: Option<String>,
  pub last_used_at: Option<String>,
  pub revoked_at: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateTokenRequest {
  pub name: String,
  pub role: String,
  #[serde(rename = "expiresIn")]
  pub expires_in: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatedTokenResponse {
  pub token: TokenMetadata,
  pub plaintext_token: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatedWorkspaceTokenResponse {
  pub token: WorkspaceTokenMetadata,
  pub plaintext_token: String,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RevokedTokenMetadata {
  pub id: String,
  pub name: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub environment_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub workspace_id: Option<String>,
  pub created_at: String,
  pub expires_at: Option<String>,
  pub last_used_at: Option<String>,
  pub revoked_at: Option<String>,
}

impl From<TokenMetadata> for RevokedTokenMetadata {
  fn from(value: TokenMetadata) -> Self {
    Self {
      id: value.id,
      name: value.name,
      environment_id: Some(value.environment_id),
      workspace_id: None,
      created_at: value.created_at,
      expires_at: value.expires_at,
      last_used_at: value.last_used_at,
      revoked_at: value.revoked_at,
    }
  }
}

impl From<WorkspaceTokenMetadata> for RevokedTokenMetadata {
  fn from(value: WorkspaceTokenMetadata) -> Self {
    Self {
      id: value.id,
      name: value.name,
      environment_id: None,
      workspace_id: Some(value.workspace_id),
      created_at: value.created_at,
      expires_at: value.expires_at,
      last_used_at: value.last_used_at,
      revoked_at: value.revoked_at,
    }
  }
}
