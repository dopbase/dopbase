use super::model::ProjectResponse;
use sqlx::{Sqlite, SqlitePool, Transaction};
pub async fn list(pool: &SqlitePool) -> Result<Vec<ProjectResponse>, sqlx::Error> {
    sqlx::query_as("SELECT id,name,created_at,updated_at FROM projects ORDER BY name")
        .fetch_all(pool)
        .await
}
pub async fn find(
    pool: &SqlitePool,
    reference: &str,
) -> Result<Option<ProjectResponse>, sqlx::Error> {
    sqlx::query_as("SELECT id,name,created_at,updated_at FROM projects WHERE id=? OR name=?")
        .bind(reference)
        .bind(reference)
        .fetch_optional(pool)
        .await
}
pub async fn insert(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
    name: &str,
    now: &str,
) -> Result<ProjectResponse, sqlx::Error> {
    sqlx::query("INSERT INTO projects(id,name,created_at,updated_at)VALUES(?,?,?,?)")
        .bind(id)
        .bind(name)
        .bind(now)
        .bind(now)
        .execute(&mut **tx)
        .await?;
    Ok(ProjectResponse {
        id: id.into(),
        name: name.into(),
        created_at: now.into(),
        updated_at: now.into(),
    })
}
