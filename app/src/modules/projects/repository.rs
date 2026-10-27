use super::model::ProjectResponse;
use crate::utils::environment_reference::normalize_path;
use sqlx::{Sqlite, SqlitePool, Transaction};

const SELECT: &str = "SELECT p.id,p.name,l.workspace_id,l.relative_path,p.created_at,p.updated_at FROM projects p LEFT JOIN project_locations l ON l.project_id=p.id";

pub async fn list(pool: &SqlitePool) -> Result<Vec<ProjectResponse>, sqlx::Error> {
  sqlx::query_as(&format!("{SELECT} ORDER BY p.name"))
    .fetch_all(pool)
    .await
}

pub async fn find_all(
  pool: &SqlitePool,
  reference: &str,
) -> Result<Vec<ProjectResponse>, sqlx::Error> {
  if let Some((workspace, project_path)) = reference.split_once('/') {
    let project_path = normalize_path(project_path);
    sqlx::query_as(&format!(
      "{SELECT}
       JOIN workspaces w ON w.id = l.workspace_id
       WHERE w.name = ?
         AND (p.id = ? OR p.name = ? OR l.relative_path = ?)"
    ))
    .bind(workspace)
    .bind(&project_path)
    .bind(&project_path)
    .bind(&project_path)
    .fetch_all(pool)
    .await
  } else {
    sqlx::query_as(&format!("{SELECT} WHERE p.id=? OR p.name=?"))
      .bind(reference)
      .bind(reference)
      .fetch_all(pool)
      .await
  }
}

pub async fn name_taken_in_workspace(
  pool: &SqlitePool,
  workspace_id: &str,
  name: &str,
  exclude_project_id: Option<&str>,
) -> Result<bool, sqlx::Error> {
  let taken: bool = match exclude_project_id {
    Some(project_id) => {
      sqlx::query_scalar(
        "SELECT EXISTS(
         SELECT 1 FROM projects p
         INNER JOIN project_locations l ON l.project_id = p.id
         WHERE l.workspace_id = ? AND p.name = ? AND p.id != ?)",
      )
      .bind(workspace_id)
      .bind(name)
      .bind(project_id)
      .fetch_one(pool)
      .await?
    }
    None => {
      sqlx::query_scalar(
        "SELECT EXISTS(
         SELECT 1 FROM projects p
         INNER JOIN project_locations l ON l.project_id = p.id
         WHERE l.workspace_id = ? AND p.name = ?)",
      )
      .bind(workspace_id)
      .bind(name)
      .fetch_one(pool)
      .await?
    }
  };
  Ok(taken)
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
    workspace_id: None,
    relative_path: None,
    created_at: now.into(),
    updated_at: now.into(),
  })
}
