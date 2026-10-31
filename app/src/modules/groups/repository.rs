use super::model::{AccountRef, GroupMember, GroupResponse};
use sqlx::{SqliteConnection, SqlitePool};

const SELECT: &str = "SELECT g.id, g.name, g.description, (SELECT COUNT(*) FROM group_members m WHERE m.group_id = g.id) AS member_count, c.id AS created_by_id, c.email AS created_by_email, g.created_at, g.updated_at FROM groups g LEFT JOIN admins c ON c.id = g.created_by";

/// An account that may be gone, from a LEFT JOIN on admins.
fn account_ref(
  id: Option<String>,
  email: Option<String>,
) -> Option<AccountRef> {
  Some(AccountRef {
    id: id?,
    email: email?,
  })
}

#[derive(sqlx::FromRow)]
struct GroupRow {
  id: String,
  name: String,
  description: String,
  member_count: i64,
  created_by_id: Option<String>,
  created_by_email: Option<String>,
  created_at: String,
  updated_at: String,
}

impl From<GroupRow> for GroupResponse {
  fn from(row: GroupRow) -> Self {
    Self {
      id: row.id,
      name: row.name,
      description: row.description,
      member_count: row.member_count,
      created_by: account_ref(row.created_by_id, row.created_by_email),
      created_at: row.created_at,
      updated_at: row.updated_at,
    }
  }
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<GroupResponse>, sqlx::Error> {
  let rows: Vec<GroupRow> = sqlx::query_as(&format!("{SELECT} ORDER BY g.name"))
    .fetch_all(pool)
    .await?;
  Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn find(
  conn: &mut SqliteConnection,
  id: &str,
) -> Result<Option<GroupResponse>, sqlx::Error> {
  let row: Option<GroupRow> = sqlx::query_as(&format!("{SELECT} WHERE g.id = ?"))
    .bind(id)
    .fetch_optional(conn)
    .await?;
  Ok(row.map(Into::into))
}

#[derive(sqlx::FromRow)]
struct MemberRow {
  id: String,
  email: String,
  added_at: String,
  added_by_id: Option<String>,
  added_by_email: Option<String>,
}

pub async fn members(
  conn: &mut SqliteConnection,
  id: &str,
) -> Result<Vec<GroupMember>, sqlx::Error> {
  let rows: Vec<MemberRow> = sqlx::query_as("SELECT a.id, a.email, m.created_at AS added_at, b.id AS added_by_id, b.email AS added_by_email FROM group_members m JOIN admins a ON a.id = m.user_id LEFT JOIN admins b ON b.id = m.added_by WHERE m.group_id = ? ORDER BY a.email")
    .bind(id)
    .fetch_all(conn)
    .await?;
  Ok(
    rows
      .into_iter()
      .map(|row| GroupMember {
        id: row.id,
        email: row.email,
        added_at: row.added_at,
        added_by: account_ref(row.added_by_id, row.added_by_email),
      })
      .collect(),
  )
}

/// Every group's members as (group id, member), each group's by email.
pub async fn all_members(pool: &SqlitePool) -> Result<Vec<(String, AccountRef)>, sqlx::Error> {
  let rows: Vec<(String, String, String)> = sqlx::query_as("SELECT m.group_id, a.id, a.email FROM group_members m JOIN admins a ON a.id = m.user_id ORDER BY a.email")
    .fetch_all(pool)
    .await?;
  Ok(
    rows
      .into_iter()
      .map(|(group_id, id, email)| (group_id, AccountRef { id, email }))
      .collect(),
  )
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
