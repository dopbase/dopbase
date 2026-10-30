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
    level: Default::default(),
    created_at: now.into(),
    updated_at: now.into(),
  })
}

/// Makes the creator of a project its project manager.
pub async fn grant_creator(
  tx: &mut Transaction<'_, Sqlite>,
  project_id: &str,
  admin_id: &str,
  now: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query("INSERT INTO grants(id,principal_type,principal_id,scope_type,scope_id,level,granted_by,created_at) VALUES(?,'user',?,'project',?,'project_manager',?,?)")
    .bind(crate::services::token::public_id(crate::constants::tokens::GRANT_ID_PREFIX))
    .bind(admin_id)
    .bind(project_id)
    .bind(admin_id)
    .bind(now)
    .execute(&mut **tx)
    .await?;
  Ok(())
}
