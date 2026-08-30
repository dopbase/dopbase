use super::model::EnvironmentResponse;
use sqlx::SqlitePool;
const SELECT: &str = "SELECT e.id,e.project_id,p.name AS project_name,e.name,e.created_at,e.updated_at FROM environments e JOIN projects p ON p.id=e.project_id";
pub async fn find_id(
  pool: &SqlitePool,
  id: &str,
) -> Result<Option<EnvironmentResponse>, sqlx::Error> {
  sqlx::query_as(&format!("{SELECT} WHERE e.id=?"))
    .bind(id)
    .fetch_optional(pool)
    .await
}
pub async fn resolve(
  pool: &SqlitePool,
  reference: &str,
) -> Result<Option<EnvironmentResponse>, sqlx::Error> {
  if reference.starts_with("env_") {
    return find_id(pool, reference).await;
  }
  let Some((project, environment)) = reference.split_once('/') else {
    return Ok(None);
  };
  sqlx::query_as(&format!("{SELECT} WHERE p.name=? AND e.name=?"))
    .bind(project)
    .bind(environment)
    .fetch_optional(pool)
    .await
}
pub async fn list(
  pool: &SqlitePool,
  project: Option<&str>,
) -> Result<Vec<EnvironmentResponse>, sqlx::Error> {
  match project {
    Some(value) => {
      sqlx::query_as(&format!(
        "{SELECT} WHERE p.id=? OR p.name=? ORDER BY p.name,e.name"
      ))
      .bind(value)
      .bind(value)
      .fetch_all(pool)
      .await
    }
    None => {
      sqlx::query_as(&format!("{SELECT} ORDER BY p.name,e.name"))
        .fetch_all(pool)
        .await
    }
  }
}
