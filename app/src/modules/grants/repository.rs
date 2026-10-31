use super::model::{GrantResponse, GroupRef, PrincipalType, ScopeType};
use crate::services::authx::Level;
use sqlx::{SqliteConnection, SqlitePool};

const SELECT: &str = "SELECT g.id, g.principal_type, g.principal_id, \
  COALESCE(CASE g.principal_type WHEN 'user' THEN (SELECT email FROM admins WHERE id = g.principal_id) ELSE (SELECT name FROM groups WHERE id = g.principal_id) END, '') AS principal_label, \
  g.scope_type, g.scope_id, \
  COALESCE(CASE g.scope_type WHEN 'all' THEN '' WHEN 'project' THEN (SELECT name FROM projects WHERE id = g.scope_id) ELSE (SELECT p.name || '/' || e.name FROM environments e JOIN projects p ON p.id = e.project_id WHERE e.id = g.scope_id) END, '') AS scope_label, \
  COALESCE(CASE g.scope_type WHEN 'all' THEN '' WHEN 'project' THEN g.scope_id ELSE (SELECT project_id FROM environments WHERE id = g.scope_id) END, '') AS project_id, \
  g.level, g.granted_by, g.created_at FROM grants g";

#[derive(sqlx::FromRow)]
struct GrantRow {
  id: String,
  principal_type: String,
  principal_id: String,
  principal_label: String,
  scope_type: String,
  scope_id: String,
  scope_label: String,
  project_id: String,
  level: String,
  granted_by: Option<String>,
  created_at: String,
}

impl GrantRow {
  fn response(self) -> Option<GrantResponse> {
    Some(GrantResponse {
      id: self.id,
      principal_type: PrincipalType::parse(&self.principal_type)?,
      principal_id: self.principal_id,
      principal_label: self.principal_label,
      scope_type: ScopeType::parse(&self.scope_type)?,
      scope_id: self.scope_id,
      scope_label: self.scope_label,
      project_id: self.project_id,
      level: Level::parse(&self.level)?,
      granted_by: self.granted_by,
      created_at: self.created_at,
    })
  }
}

fn responses(rows: Vec<GrantRow>) -> Vec<GrantResponse> {
  rows.into_iter().filter_map(GrantRow::response).collect()
}

pub async fn find(
  conn: &mut SqliteConnection,
  id: &str,
) -> Result<Option<GrantResponse>, sqlx::Error> {
  let row: Option<GrantRow> = sqlx::query_as(&format!("{SELECT} WHERE g.id = ?"))
    .bind(id)
    .fetch_optional(conn)
    .await?;
  Ok(row.and_then(GrantRow::response))
}

/// Grants on a project and on each of its environments.
pub async fn for_project(
  pool: &SqlitePool,
  project_id: &str,
) -> Result<Vec<GrantResponse>, sqlx::Error> {
  let rows = sqlx::query_as(&format!(
    "{SELECT} WHERE (g.scope_type = 'project' AND g.scope_id = ?1) OR (g.scope_type = 'environment' AND g.scope_id IN (SELECT id FROM environments WHERE project_id = ?1)) ORDER BY g.scope_type DESC, scope_label, principal_label"
  ))
  .bind(project_id)
  .fetch_all(pool)
  .await?;
  Ok(responses(rows))
}

/// An account's own grants and those of its groups.
pub async fn for_user(
  pool: &SqlitePool,
  user_id: &str,
) -> Result<Vec<GrantResponse>, sqlx::Error> {
  let rows = sqlx::query_as(&format!(
    "{SELECT} WHERE (g.principal_type = 'user' AND g.principal_id = ?1) OR (g.principal_type = 'group' AND g.principal_id IN (SELECT group_id FROM group_members WHERE user_id = ?1)) ORDER BY scope_label, g.principal_type DESC"
  ))
  .bind(user_id)
  .fetch_all(pool)
  .await?;
  Ok(responses(rows))
}

pub async fn for_group(
  conn: &mut SqliteConnection,
  group_id: &str,
) -> Result<Vec<GrantResponse>, sqlx::Error> {
  let rows = sqlx::query_as(&format!(
    "{SELECT} WHERE g.principal_type = 'group' AND g.principal_id = ? ORDER BY scope_label"
  ))
  .bind(group_id)
  .fetch_all(conn)
  .await?;
  Ok(responses(rows))
}

/// The grants of every group, each group's by scope.
pub async fn for_groups(pool: &SqlitePool) -> Result<Vec<GrantResponse>, sqlx::Error> {
  let rows = sqlx::query_as(&format!(
    "{SELECT} WHERE g.principal_type = 'group' ORDER BY scope_label"
  ))
  .fetch_all(pool)
  .await?;
  Ok(responses(rows))
}

pub async fn groups_of(
  pool: &SqlitePool,
  user_id: &str,
) -> Result<Vec<GroupRef>, sqlx::Error> {
  sqlx::query_as("SELECT g.id, g.name FROM groups g JOIN group_members m ON m.group_id = g.id WHERE m.user_id = ? ORDER BY g.name")
    .bind(user_id)
    .fetch_all(pool)
    .await
}

/// The level a principal already holds on a scope.
pub async fn existing_level(
  conn: &mut SqliteConnection,
  principal_type: PrincipalType,
  principal_id: &str,
  scope_type: ScopeType,
  scope_id: &str,
) -> Result<Option<String>, sqlx::Error> {
  sqlx::query_scalar("SELECT level FROM grants WHERE principal_type = ? AND principal_id = ? AND scope_type = ? AND scope_id = ?")
    .bind(principal_type.as_str())
    .bind(principal_id)
    .bind(scope_type.as_str())
    .bind(scope_id)
    .fetch_optional(conn)
    .await
}

pub struct NewGrant<'a> {
  pub id: &'a str,
  pub principal_type: PrincipalType,
  pub principal_id: &'a str,
  pub scope_type: ScopeType,
  pub scope_id: &'a str,
  pub level: Level,
  pub granted_by: &'a str,
  pub now: &'a str,
}

/// Creates the grant, or changes the level of the one already on that
/// scope. Returns the grant's id.
pub async fn upsert(
  conn: &mut SqliteConnection,
  grant: NewGrant<'_>,
) -> Result<String, sqlx::Error> {
  sqlx::query_scalar(
    "INSERT INTO grants(id,principal_type,principal_id,scope_type,scope_id,level,granted_by,created_at) VALUES(?,?,?,?,?,?,?,?) \
     ON CONFLICT(principal_type,principal_id,scope_type,scope_id) DO UPDATE SET level=excluded.level, granted_by=excluded.granted_by, created_at=excluded.created_at \
     RETURNING id",
  )
  .bind(grant.id)
  .bind(grant.principal_type.as_str())
  .bind(grant.principal_id)
  .bind(grant.scope_type.as_str())
  .bind(grant.scope_id)
  .bind(grant.level.as_str())
  .bind(grant.granted_by)
  .bind(grant.now)
  .fetch_one(conn)
  .await
}

pub async fn delete(
  conn: &mut SqliteConnection,
  id: &str,
) -> Result<bool, sqlx::Error> {
  Ok(
    sqlx::query("DELETE FROM grants WHERE id = ?")
      .bind(id)
      .execute(conn)
      .await?
      .rows_affected()
      == 1,
  )
}

/// The email, root flag, and role of an account.
pub async fn account(
  conn: &mut SqliteConnection,
  id: &str,
) -> Result<Option<(String, bool, Option<String>)>, sqlx::Error> {
  sqlx::query_as("SELECT email, is_root = 1, role_id FROM admins WHERE id = ?")
    .bind(id)
    .fetch_optional(conn)
    .await
}

pub async fn group_name(
  conn: &mut SqliteConnection,
  id: &str,
) -> Result<Option<String>, sqlx::Error> {
  sqlx::query_scalar("SELECT name FROM groups WHERE id = ?")
    .bind(id)
    .fetch_optional(conn)
    .await
}
