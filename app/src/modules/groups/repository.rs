use super::model::{GroupMember, GroupResponse};
use sqlx::{SqliteConnection, SqlitePool};

const SELECT: &str = "SELECT g.id, g.name, g.description, (SELECT COUNT(*) FROM group_members m WHERE m.group_id = g.id) AS member_count, g.created_at, g.updated_at FROM groups g";

pub async fn list(pool: &SqlitePool) -> Result<Vec<GroupResponse>, sqlx::Error> {
  sqlx::query_as(&format!("{SELECT} ORDER BY g.name"))
    .fetch_all(pool)
    .await
}

pub async fn find(
  conn: &mut SqliteConnection,
  id: &str,
) -> Result<Option<GroupResponse>, sqlx::Error> {
  sqlx::query_as(&format!("{SELECT} WHERE g.id = ?"))
    .bind(id)
    .fetch_optional(conn)
    .await
}

pub async fn members(
  conn: &mut SqliteConnection,
  id: &str,
) -> Result<Vec<GroupMember>, sqlx::Error> {
  sqlx::query_as("SELECT a.id, a.email, m.created_at AS added_at FROM group_members m JOIN admins a ON a.id = m.user_id WHERE m.group_id = ? ORDER BY a.email")
    .bind(id)
    .fetch_all(conn)
    .await
}

/// Fails with a unique violation when the name is taken.
pub async fn insert(
  conn: &mut SqliteConnection,
  id: &str,
  name: &str,
  description: &str,
  created_by: &str,
  now: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query(
    "INSERT INTO groups(id,name,description,created_by,created_at,updated_at) VALUES(?,?,?,?,?,?)",
  )
  .bind(id)
  .bind(name)
  .bind(description)
  .bind(created_by)
  .bind(now)
  .bind(now)
  .execute(conn)
  .await?;
  Ok(())
}

/// Fails with a unique violation when the name is taken.
pub async fn update(
  conn: &mut SqliteConnection,
  id: &str,
  name: &str,
  description: &str,
  now: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query("UPDATE groups SET name = ?, description = ?, updated_at = ? WHERE id = ?")
    .bind(name)
    .bind(description)
    .bind(now)
    .bind(id)
    .execute(conn)
    .await?;
  Ok(())
}

/// Its grants and memberships go with it.
pub async fn delete(
  conn: &mut SqliteConnection,
  id: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query("DELETE FROM groups WHERE id = ?")
    .bind(id)
    .execute(conn)
    .await?;
  Ok(())
}

/// Returns false when the user was already a member.
pub async fn add_member(
  conn: &mut SqliteConnection,
  group_id: &str,
  user_id: &str,
  added_by: &str,
  now: &str,
) -> Result<bool, sqlx::Error> {
  Ok(
    sqlx::query("INSERT INTO group_members(group_id,user_id,added_by,created_at) VALUES(?,?,?,?) ON CONFLICT DO NOTHING")
      .bind(group_id)
      .bind(user_id)
      .bind(added_by)
      .bind(now)
      .execute(conn)
      .await?
      .rows_affected()
      == 1,
  )
}

/// Returns false when the user was not a member.
pub async fn remove_member(
  conn: &mut SqliteConnection,
  group_id: &str,
  user_id: &str,
) -> Result<bool, sqlx::Error> {
  Ok(
    sqlx::query("DELETE FROM group_members WHERE group_id = ? AND user_id = ?")
      .bind(group_id)
      .bind(user_id)
      .execute(conn)
      .await?
      .rows_affected()
      == 1,
  )
}

/// The email and root flag of an account.
pub async fn account(
  conn: &mut SqliteConnection,
  id: &str,
) -> Result<Option<(String, bool)>, sqlx::Error> {
  sqlx::query_as("SELECT email, is_root = 1 FROM admins WHERE id = ?")
    .bind(id)
    .fetch_optional(conn)
    .await
}
