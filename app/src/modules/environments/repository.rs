use super::model::EnvironmentResponse;
use crate::utils::environment_reference::{EnvironmentReference, normalize_path, parse};
use sqlx::{Sqlite, SqlitePool, Transaction};

const SELECT: &str = "SELECT e.id,e.project_id,p.name AS project_name,e.name,e.created_at,e.updated_at FROM environments e JOIN projects p ON p.id=e.project_id";

pub async fn insert(
  tx: &mut Transaction<'_, Sqlite>,
  id: &str,
  project_id: &str,
  name: &str,
  now: &str,
) -> Result<bool, sqlx::Error> {
  let result = sqlx::query(
    "INSERT INTO environments(id,project_id,name,created_at,updated_at)VALUES(?,?,?,?,?) ON CONFLICT(id) DO NOTHING",
  )
  .bind(id)
  .bind(project_id)
  .bind(name)
  .bind(now)
  .bind(now)
  .execute(&mut **tx)
  .await?;
  Ok(result.rows_affected() == 1)
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

pub async fn resolve_all(
  pool: &SqlitePool,
  reference: &str,
) -> Result<Vec<EnvironmentResponse>, sqlx::Error> {
  let parsed = parse(reference).ok_or_else(|| sqlx::Error::RowNotFound)?;
  match parsed {
    EnvironmentReference::Id(id) => Ok(find_id(pool, &id).await?.into_iter().collect()),
    EnvironmentReference::Legacy {
      project,
      environment,
    } => {
      sqlx::query_as(&format!("{SELECT} WHERE (p.id=? OR p.name=?) AND e.name=?"))
        .bind(&project)
        .bind(&project)
        .bind(&environment)
        .fetch_all(pool)
        .await
    }
    EnvironmentReference::Qualified {
      workspace,
      project_path,
      environment,
    } => resolve_qualified(pool, &workspace, &project_path, &environment).await,
  }
}

async fn resolve_qualified(
  pool: &SqlitePool,
  workspace: &str,
  project_path: &str,
  environment: &str,
) -> Result<Vec<EnvironmentResponse>, sqlx::Error> {
  let project_path = normalize_path(project_path);
  sqlx::query_as(&format!(
    "{SELECT}
     JOIN project_locations l ON l.project_id = p.id
     JOIN workspaces w ON w.id = l.workspace_id
     WHERE w.name = ?
       AND e.name = ?
       AND (p.name = ? OR l.relative_path = ?)"
  ))
  .bind(workspace)
  .bind(environment)
  .bind(&project_path)
  .bind(&project_path)
  .fetch_all(pool)
  .await
}

pub async fn list(
  pool: &SqlitePool,
  project: Option<&str>,
) -> Result<Vec<EnvironmentResponse>, sqlx::Error> {
  match project {
    Some(value) => {
      if let Some((workspace, project_path)) = value.split_once('/') {
        let project_path = normalize_path(project_path);
        sqlx::query_as(&format!(
          "{SELECT}
           JOIN project_locations l ON l.project_id = p.id
           JOIN workspaces w ON w.id = l.workspace_id
           WHERE w.name = ?
             AND (p.id = ? OR p.name = ? OR l.relative_path = ?)
           ORDER BY p.name,e.name"
        ))
        .bind(workspace)
        .bind(&project_path)
        .bind(&project_path)
        .bind(&project_path)
        .fetch_all(pool)
        .await
      } else {
        sqlx::query_as(&format!(
          "{SELECT} WHERE p.id=? OR p.name=? ORDER BY p.name,e.name"
        ))
        .bind(value)
        .bind(value)
        .fetch_all(pool)
        .await
      }
    }
    None => {
      sqlx::query_as(&format!("{SELECT} ORDER BY p.name,e.name"))
        .fetch_all(pool)
        .await
    }
  }
}
