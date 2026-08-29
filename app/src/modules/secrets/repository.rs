use super::model::SecretMetadata;
use crate::services::crypto::EncryptedValue;
use sqlx::SqlitePool;
#[derive(sqlx::FromRow)]
pub struct SecretRow {
    pub key: String,
    pub version: i64,
    pub ciphertext: Vec<u8>,
    pub value_nonce: Vec<u8>,
    pub wrapped_key: Vec<u8>,
    pub key_nonce: Vec<u8>,
    pub created_at: String,
    pub updated_at: String,
}
impl SecretRow {
    pub fn encrypted(&self) -> EncryptedValue {
        EncryptedValue {
            ciphertext: self.ciphertext.clone(),
            value_nonce: self.value_nonce.clone(),
            wrapped_key: self.wrapped_key.clone(),
            key_nonce: self.key_nonce.clone(),
        }
    }
    pub fn metadata(&self) -> SecretMetadata {
        SecretMetadata {
            key: self.key.clone(),
            version: self.version,
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
        }
    }
}
pub async fn list(
    pool: &SqlitePool,
    environment_id: &str,
) -> Result<Vec<SecretMetadata>, sqlx::Error> {
    sqlx::query_as(
        "SELECT key,version,created_at,updated_at FROM secrets WHERE environment_id=? ORDER BY key",
    )
    .bind(environment_id)
    .fetch_all(pool)
    .await
}
pub async fn rows(pool: &SqlitePool, environment_id: &str) -> Result<Vec<SecretRow>, sqlx::Error> {
    sqlx::query_as("SELECT key,version,ciphertext,value_nonce,wrapped_key,key_nonce,created_at,updated_at FROM secrets WHERE environment_id=? ORDER BY key").bind(environment_id).fetch_all(pool).await
}
pub async fn find(
    pool: &SqlitePool,
    environment_id: &str,
    key: &str,
) -> Result<Option<SecretRow>, sqlx::Error> {
    sqlx::query_as("SELECT key,version,ciphertext,value_nonce,wrapped_key,key_nonce,created_at,updated_at FROM secrets WHERE environment_id=? AND key=?").bind(environment_id).bind(key).fetch_optional(pool).await
}
pub async fn layout(
    pool: &SqlitePool,
    environment_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT layout FROM environment_env_layout WHERE environment_id=?")
            .bind(environment_id)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|(layout,)| layout))
}
pub async fn upsert_layout(
    tx: &mut sqlx::SqliteConnection,
    environment_id: &str,
    layout: &str,
    now: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO environment_env_layout(environment_id,layout,updated_at)VALUES(?,?,?) ON CONFLICT(environment_id) DO UPDATE SET layout=excluded.layout,updated_at=excluded.updated_at")
        .bind(environment_id)
        .bind(layout)
        .bind(now)
        .execute(tx)
        .await?;
    Ok(())
}
