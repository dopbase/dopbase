use sqlx::{SqliteConnection, SqlitePool};

use crate::{
  constants::tokens::GRANT_ID_PREFIX, modules::grants::model::GroupRef, services::token,
};

const USER_COLUMNS: &str = "a.id,a.email,a.role,a.role_id,CASE WHEN a.is_root = 1 THEN 'Root' ELSE COALESCE(r.name, '') END AS role_name,a.created_at,a.updated_at,a.password_hash<>'' AS has_password,EXISTS(SELECT 1 FROM admin_identities i WHERE i.admin_id=a.id) AS sso_linked,a.owner_id,(SELECT o.email FROM admins o WHERE o.id = a.owner_id) AS owner_email,COALESCE(r.sign_in = 'token_only', 0) AS agent,(a.status = 'active' AND (a.is_root = 1 OR (r.sign_in = 'interactive' AND EXISTS(SELECT 1 FROM role_capabilities c WHERE c.role_id = a.role_id AND c.capability = 'agents.view')))) AS can_own,a.status,a.last_sign_in_at";

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
  pub owner_email: Option<String>,
  /// The account's role signs in with tokens only: an AI agent.
  pub agent: bool,
  /// An active person who could own an AI agent, as [`can_own`] judges it.
  pub can_own: bool,
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

/// Every account's id and email, by email.
pub async fn identities(pool: &SqlitePool) -> Result<Vec<(String, String)>, sqlx::Error> {
  sqlx::query_as("SELECT id, email FROM admins ORDER BY email")
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

/// Active people, the accounts a people limit counts: not root, and not AI
/// agents.
const ACTIVE_PEOPLE: &str = "SELECT COUNT(*) FROM admins p LEFT JOIN roles pr ON pr.id = p.role_id WHERE p.is_root = 0 AND p.status = 'active' AND COALESCE(pr.sign_in = 'token_only', 0) = 0";

pub async fn active_people(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
  sqlx::query_scalar(ACTIVE_PEOPLE).fetch_one(pool).await
}

/// Sets `status` and returns false when the account already had it, or
/// when `limit` is set and that many people are already active. Root is
/// never disabled.
pub async fn set_status(
  conn: &mut SqliteConnection,
  id: &str,
  status: &str,
  limit: Option<u32>,
  now: &str,
) -> Result<bool, sqlx::Error> {
  // The count sits in the same statement as the write, so two requests at
  // once can't both take the last spot.
  let query = format!(
    "UPDATE admins SET status=?,updated_at=? WHERE id=? AND is_root=0 AND status<>? AND (? IS NULL OR ({ACTIVE_PEOPLE}) < ?)"
  );
  Ok(
    sqlx::query(&query)
      .bind(status)
      .bind(now)
      .bind(id)
      .bind(status)
      .bind(limit)
      .bind(limit)
      .execute(conn)
      .await?
      .rows_affected()
      == 1,
  )
}

/// `active` or `disabled`, or `None` when the account doesn't exist.
pub async fn status(
  conn: &mut SqliteConnection,
  id: &str,
) -> Result<Option<String>, sqlx::Error> {
  sqlx::query_scalar("SELECT status FROM admins WHERE id=?")
    .bind(id)
    .fetch_optional(conn)
    .await
}

/// What changing an account needs to know about it.
#[derive(sqlx::FromRow)]
pub struct Account {
  pub email: String,
  pub is_root: bool,
  pub role_id: Option<String>,
  /// Its role signs in with tokens only: an AI agent.
  pub agent: bool,
  pub owner_id: Option<String>,
  /// `active` or `disabled`.
  pub status: String,
}

pub async fn account(
  pool: &SqlitePool,
  id: &str,
) -> Result<Option<Account>, sqlx::Error> {
  sqlx::query_as("SELECT a.email, a.is_root = 1 AS is_root, a.role_id, COALESCE(r.sign_in = 'token_only', 0) AS agent, a.owner_id, a.status FROM admins a LEFT JOIN roles r ON r.id = a.role_id WHERE a.id=?")
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Who disabled the account most recently, from the audit log. `None` when
/// it was never disabled or the record has been pruned.
pub async fn last_disabled_by(
  pool: &SqlitePool,
  id: &str,
) -> Result<Option<String>, sqlx::Error> {
  let actor: Option<Option<String>> = sqlx::query_scalar(
    "SELECT actor_id FROM audit_events WHERE action = 'user.disabled' AND resource_type = 'admin' AND resource_id = ? ORDER BY seq DESC LIMIT 1",
  )
  .bind(id)
  .fetch_optional(pool)
  .await?;
  Ok(actor.flatten())
}

/// `Some(owner)` when `id` is an AI agent, `None` for a person or a
/// missing account.
pub async fn agent(
  pool: &SqlitePool,
  id: &str,
) -> Result<Option<Option<String>>, sqlx::Error> {
  sqlx::query_scalar(
    "SELECT a.owner_id FROM admins a JOIN roles r ON r.id = a.role_id WHERE a.id = ? AND r.sign_in = 'token_only'",
  )
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
  /// The people limit to respect, for a person. `None` for AI agents and
  /// when there is no limit.
  pub limit: Option<u32>,
}

/// Returns false, adding nobody, when `user.limit` is set and that many
/// people are already active. Fails with a unique violation when the email
/// is taken.
pub async fn insert(
  conn: &mut SqliteConnection,
  user: NewUser<'_>,
) -> Result<bool, sqlx::Error> {
  // The count sits in the same statement as the write, so two requests at
  // once can't both take the last spot.
  let query = format!(
    "INSERT INTO admins(id,email,password_hash,role,role_id,owner_id,created_by,created_at,updated_at) SELECT ?,?,?,?,?,?,?,?,? WHERE ? IS NULL OR ({ACTIVE_PEOPLE}) < ?"
  );
  Ok(
    sqlx::query(&query)
      .bind(user.id)
      .bind(user.email)
      .bind(user.password_hash)
      .bind(user.role)
      .bind(user.role_id)
      .bind(user.owner_id)
      .bind(user.created_by)
      .bind(user.now)
      .bind(user.now)
      .bind(user.limit)
      .bind(user.limit)
      .execute(conn)
      .await?
      .rows_affected()
      == 1,
  )
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

/// Cancels the CLI logins the account approved that the CLI hasn't picked
/// up yet, so none of them turns into a session.
pub async fn cancel_device_approvals(
  conn: &mut SqliteConnection,
  id: &str,
  now: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query("UPDATE cli_device_requests SET status='cancelled',decided_at=? WHERE approved_by=? AND status='approved'")
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

/// Whether `id` can own an AI agent: an active person whose role signs in
/// and holds agents.view, so they can see and run it.
pub async fn can_own(
  conn: &mut SqliteConnection,
  id: &str,
) -> Result<bool, sqlx::Error> {
  let found: Option<i64> = sqlx::query_scalar(
    "SELECT 1 FROM admins a LEFT JOIN roles r ON r.id = a.role_id WHERE a.id = ? AND a.status = 'active' AND (a.is_root = 1 OR (r.sign_in = 'interactive' AND EXISTS(SELECT 1 FROM role_capabilities c WHERE c.role_id = a.role_id AND c.capability = 'agents.view')))",
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

/// Whether a role signs in with tokens only, and whether it is built in.
pub async fn role_sign_in(
  pool: &SqlitePool,
  role_id: &str,
) -> Result<Option<(bool, bool)>, sqlx::Error> {
  sqlx::query_as("SELECT sign_in = 'token_only', builtin = 1 FROM roles WHERE id=?")
    .bind(role_id)
    .fetch_optional(pool)
    .await
}
