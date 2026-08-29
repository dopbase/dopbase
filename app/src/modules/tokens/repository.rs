use super::model::TokenMetadata;
use sqlx::SqlitePool;
pub async fn list(pool: &SqlitePool, id: &str) -> Result<Vec<TokenMetadata>, sqlx::Error> {
    sqlx::query_as("SELECT id,environment_id,name,created_at,last_used_at,revoked_at FROM runner_tokens WHERE environment_id=? ORDER BY name").bind(id).fetch_all(pool).await
}
pub async fn find(pool: &SqlitePool, id: &str) -> Result<Option<TokenMetadata>, sqlx::Error> {
    sqlx::query_as("SELECT id,environment_id,name,created_at,last_used_at,revoked_at FROM runner_tokens WHERE id=?").bind(id).fetch_optional(pool).await
}
