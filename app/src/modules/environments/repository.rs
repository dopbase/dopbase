use super::model::EnvironmentResponse;
use sqlx::{Sqlite, SqlitePool, Transaction};
const SELECT: &str = "SELECT e.id,e.project_id,p.name AS project_name,e.name,e.created_at,e.updated_at FROM environments e JOIN projects p ON p.id=e.project_id";

pub async fn reserve_id(
  tx: &mut Transaction<'_, Sqlite>,
  id: &str,
) -> Result<bool, sqlx::Error> {
  let result = sqlx::query("INSERT OR IGNORE INTO environment_id_reservations(id)VALUES(?)")
    .bind(id)
    .execute(&mut **tx)
    .await?;
  Ok(result.rows_affected() == 1)
}

pub async fn next_id_number(tx: &mut Transaction<'_, Sqlite>) -> Result<u32, sqlx::Error> {
  sqlx::query_scalar("SELECT next_number FROM environment_id_sequence WHERE id=1")
    .fetch_one(&mut **tx)
    .await
}

pub async fn advance_id_number(
  tx: &mut Transaction<'_, Sqlite>,
  next_number: u32,
) -> Result<(), sqlx::Error> {
  sqlx::query("UPDATE environment_id_sequence SET next_number=? WHERE id=1")
    .bind(next_number)
    .execute(&mut **tx)
    .await?;
  Ok(())
}

pub async fn insert(
  tx: &mut Transaction<'_, Sqlite>,
  id: &str,
  project_id: &str,
  name: &str,
  now: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query(
    "INSERT INTO environments(id,project_id,name,created_at,updated_at)VALUES(?,?,?,?,?)",
  )
  .bind(id)
  .bind(project_id)
  .bind(name)
  .bind(now)
  .bind(now)
  .execute(&mut **tx)
  .await?;
  Ok(())
}

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
