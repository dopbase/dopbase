use sqlx::{SqliteConnection, SqlitePool};

use crate::{
  constants::tokens::GRANT_ID_PREFIX, modules::grants::model::GroupRef, services::token,
};

const USER_COLUMNS: &str = "a.id,a.email,a.role,a.role_id,CASE WHEN a.is_root = 1 THEN 'Root' ELSE COALESCE(r.name, '') END AS role_name,a.created_at,a.updated_at,a.password_hash<>'' AS has_password,EXISTS(SELECT 1 FROM admin_identities i WHERE i.admin_id=a.id) AS sso_linked,a.owner_id,a.status,a.last_sign_in_at";

#[derive(sqlx::FromRow)]
pub struct UserRow {
  pub id: String,
  pub email: String,
  pub role: String,
  pub role_id: Option<String>,
  pub role_name: String,
  pub created_at: String,
  pub updated_at: String,
  pub has_password: bool,
  pub sso_linked: bool,
  pub owner_id: Option<String>,
  pub status: String,
  pub last_sign_in_at: Option<String>,
}

/// Root first, then by role rank, each by email.
pub async fn list(pool: &SqlitePool) -> Result<Vec<UserRow>, sqlx::Error> {
  sqlx::query_as(&format!(
    "SELECT {USER_COLUMNS} FROM admins a LEFT JOIN roles r ON r.id = a.role_id ORDER BY a.is_root DESC, r.rank, a.email"
  ))
  .fetch_all(pool)
  .await
}

pub async fn find(
  pool: &SqlitePool,
  id: &str,
) -> Result<Option<UserRow>, sqlx::Error> {
  sqlx::query_as(&format!(
    "SELECT {USER_COLUMNS} FROM admins a LEFT JOIN roles r ON r.id = a.role_id WHERE a.id=?"
  ))
  .bind(id)
  .fetch_optional(pool)
  .await
}

/// Every group membership as (user id, group), each user's by group name.
pub async fn memberships(pool: &SqlitePool) -> Result<Vec<(String, GroupRef)>, sqlx::Error> {
  let rows: Vec<(String, String, String)> = sqlx::query_as(
    "SELECT m.user_id, g.id, g.name FROM group_members m JOIN groups g ON g.id = m.group_id ORDER BY g.name",
  )
  .fetch_all(pool)
  .await?;
  Ok(
    rows
      .into_iter()
      .map(|(user_id, id, name)| (user_id, GroupRef { id, name }))
      .collect(),
  )
}

/// Sets `status` and returns false when the account already had it. Root
/// is never disabled.
pub async fn set_status(
  conn: &mut SqliteConnection,
  id: &str,
  status: &str,
  now: &str,
) -> Result<bool, sqlx::Error> {
  Ok(
    sqlx::query("UPDATE admins SET status=?,updated_at=? WHERE id=? AND is_root=0 AND status<>?")
      .bind(status)
      .bind(now)
      .bind(id)
      .bind(status)
      .execute(conn)
      .await?
      .rows_affected()
      == 1,
  )
}

/// The email, root flag, role id, and whether the role signs in with tokens
/// only, of an account.
pub async fn account(
  pool: &SqlitePool,
  id: &str,
) -> Result<Option<(String, bool, Option<String>, bool)>, sqlx::Error> {
  sqlx::query_as("SELECT a.email, a.is_root = 1, a.role_id, COALESCE(r.sign_in = 'token_only', 0) FROM admins a LEFT JOIN roles r ON r.id = a.role_id WHERE a.id=?")
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub struct NewUser<'a> {
  pub id: &'a str,
  pub email: &'a str,
  pub password_hash: &'a str,
  pub role: &'a str,
  pub role_id: &'a str,
  /// The accountable person, for roles that need one.
  pub owner_id: Option<&'a str>,
  pub created_by: &'a str,
  pub now: &'a str,
}

/// Fails with a unique violation when the email is taken.
pub async fn insert(
  conn: &mut SqliteConnection,
  user: NewUser<'_>,
) -> Result<(), sqlx::Error> {
  sqlx::query(
    "INSERT INTO admins(id,email,password_hash,role,role_id,owner_id,created_by,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?)",
  )
  .bind(user.id)
  .bind(user.email)
  .bind(user.password_hash)
  .bind(user.role)
  .bind(user.role_id)
  .bind(user.owner_id)
  .bind(user.created_by)
  .bind(user.now)
  .bind(user.now)
  .execute(conn)
  .await?;
  Ok(())
}

/// Leaves the password unchanged when `password_hash` is `None`, and the
/// role when `role` is `None`; `role` is the legacy role name and the role
/// id. Fails with a unique violation when the email is taken.
pub async fn update(
  conn: &mut SqliteConnection,
  id: &str,
  email: &str,
  password_hash: Option<&str>,
  role: Option<(&str, &str)>,
  now: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query("UPDATE admins SET email=?,password_hash=COALESCE(?,password_hash),role=COALESCE(?,role),role_id=COALESCE(?,role_id),updated_at=? WHERE id=?")
    .bind(email)
    .bind(password_hash)
    .bind(role.map(|(name, _)| name))
    .bind(role.map(|(_, id)| id))
    .bind(now)
    .bind(id)
    .execute(conn)
    .await?;
  Ok(())
}

/// Replaces the account's all-projects grant with `level`, or removes it.
pub async fn set_all_projects_grant(
  conn: &mut SqliteConnection,
  id: &str,
  level: Option<&str>,
  granted_by: &str,
  now: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query(
    "DELETE FROM grants WHERE principal_type='user' AND principal_id=? AND scope_type='all'",
  )
  .bind(id)
  .execute(&mut *conn)
  .await?;
  let Some(level) = level else {
    return Ok(());
  };
  sqlx::query("INSERT INTO grants(id,principal_type,principal_id,scope_type,scope_id,level,granted_by,created_at) VALUES(?,'user',?,'all','',?,?,?)")
    .bind(token::public_id(GRANT_ID_PREFIX))
    .bind(id)
    .bind(level)
    .bind(granted_by)
    .bind(now)
    .execute(&mut *conn)
    .await?;
  Ok(())
}

pub async fn revoke_sessions(
  conn: &mut SqliteConnection,
  id: &str,
  now: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query("UPDATE sessions SET revoked_at=? WHERE admin_id=? AND revoked_at IS NULL")
    .bind(now)
    .bind(id)
    .execute(conn)
    .await?;
  Ok(())
}

/// Root can never be deleted.
pub async fn delete(
  conn: &mut SqliteConnection,
  id: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query("DELETE FROM admins WHERE id=? AND is_root=0")
    .bind(id)
    .execute(conn)
    .await?;
  Ok(())
}

/// How many accounts this account owns, such as AI agents.
pub async fn owned_count(
  pool: &SqlitePool,
  id: &str,
) -> Result<i64, sqlx::Error> {
  sqlx::query_scalar("SELECT COUNT(*) FROM admins WHERE owner_id=?")
    .bind(id)
    .fetch_one(pool)
    .await
}

/// Whether `id` can own an AI agent: an active account that signs in.
pub async fn can_own(
  conn: &mut SqliteConnection,
  id: &str,
) -> Result<bool, sqlx::Error> {
  let found: Option<i64> = sqlx::query_scalar(
    "SELECT 1 FROM admins a LEFT JOIN roles r ON r.id = a.role_id WHERE a.id = ? AND a.status = 'active' AND (a.is_root = 1 OR r.sign_in = 'interactive')",
  )
  .bind(id)
  .fetch_optional(conn)
  .await?;
  Ok(found.is_some())
}

pub async fn set_owner(
  conn: &mut SqliteConnection,
  id: &str,
  owner_id: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query("UPDATE admins SET owner_id=? WHERE id=?")
    .bind(owner_id)
    .bind(id)
    .execute(conn)
    .await?;
  Ok(())
}

/// A role's id and whether its holders sign in with tokens only.
pub async fn role_sign_in(
  pool: &SqlitePool,
  role_id: &str,
) -> Result<Option<bool>, sqlx::Error> {
  sqlx::query_scalar("SELECT sign_in = 'token_only' FROM roles WHERE id=?")
    .bind(role_id)
    .fetch_optional(pool)
    .await
}
