use sqlx::{SqliteConnection, SqlitePool};

#[derive(sqlx::FromRow)]
pub struct RoleRow {
  pub id: String,
  pub name: String,
  pub description: String,
  pub builtin: bool,
  pub max_level: String,
  pub sign_in: String,
  pub requires_owner: bool,
  pub token_scope: String,
  pub token_max_level: String,
  pub token_default_ttl_days: i64,
  pub token_max_ttl_days: i64,
  pub rank: i64,
  pub members: i64,
  pub created_at: String,
  pub updated_at: String,
}

const SELECT: &str = "SELECT r.id, r.name, r.description, r.builtin = 1 AS builtin, r.max_level, r.sign_in, r.requires_owner = 1 AS requires_owner, r.token_scope, r.token_max_level, r.token_default_ttl_days, r.token_max_ttl_days, r.rank, (SELECT COUNT(*) FROM admins a WHERE a.role_id = r.id) AS members, r.created_at, r.updated_at FROM roles r";

pub async fn list(pool: &SqlitePool) -> Result<Vec<RoleRow>, sqlx::Error> {
  sqlx::query_as(&format!("{SELECT} ORDER BY r.rank, r.name"))
    .fetch_all(pool)
    .await
}

pub async fn find(
  conn: &mut SqliteConnection,
  id: &str,
) -> Result<Option<RoleRow>, sqlx::Error> {
  sqlx::query_as(&format!("{SELECT} WHERE r.id = ?"))
    .bind(id)
    .fetch_optional(conn)
    .await
}

pub async fn capabilities(
  conn: &mut SqliteConnection,
  id: &str,
) -> Result<Vec<String>, sqlx::Error> {
  sqlx::query_scalar(
    "SELECT capability FROM role_capabilities WHERE role_id = ? ORDER BY capability",
  )
  .bind(id)
  .fetch_all(conn)
  .await
}

/// The settings a custom role is saved with.
pub struct RoleSettings<'a> {
  pub name: &'a str,
  pub description: &'a str,
  pub max_level: &'a str,
  pub sign_in: &'a str,
  pub requires_owner: bool,
  pub token_scope: &'a str,
  pub token_max_level: &'a str,
  pub token_default_ttl_days: i64,
  pub token_max_ttl_days: i64,
  pub rank: i64,
}

/// Fails with a unique violation when the name is taken.
pub async fn insert(
  conn: &mut SqliteConnection,
  id: &str,
  role: &RoleSettings<'_>,
  now: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query("INSERT INTO roles(id,name,description,builtin,max_level,sign_in,requires_owner,token_scope,token_max_level,token_default_ttl_days,token_max_ttl_days,rank,created_at,updated_at) VALUES(?,?,?,0,?,?,?,?,?,?,?,?,?,?)")
    .bind(id)
    .bind(role.name)
    .bind(role.description)
    .bind(role.max_level)
    .bind(role.sign_in)
    .bind(role.requires_owner)
    .bind(role.token_scope)
    .bind(role.token_max_level)
    .bind(role.token_default_ttl_days)
    .bind(role.token_max_ttl_days)
    .bind(role.rank)
    .bind(now)
    .bind(now)
    .execute(conn)
    .await?;
  Ok(())
}

/// Fails with a unique violation when the name is taken. Sign-in and owner
/// rules never change.
pub async fn update(
  conn: &mut SqliteConnection,
  id: &str,
  role: &RoleSettings<'_>,
  now: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query("UPDATE roles SET name=?,description=?,max_level=?,token_scope=?,token_max_level=?,token_default_ttl_days=?,token_max_ttl_days=?,rank=?,updated_at=? WHERE id=? AND builtin=0")
    .bind(role.name)
    .bind(role.description)
    .bind(role.max_level)
    .bind(role.token_scope)
    .bind(role.token_max_level)
    .bind(role.token_default_ttl_days)
    .bind(role.token_max_ttl_days)
    .bind(role.rank)
    .bind(now)
    .bind(id)
    .execute(conn)
    .await?;
  Ok(())
}

pub async fn set_capabilities(
  conn: &mut SqliteConnection,
  id: &str,
  capabilities: &[&str],
) -> Result<(), sqlx::Error> {
  sqlx::query("DELETE FROM role_capabilities WHERE role_id = ?")
    .bind(id)
    .execute(&mut *conn)
    .await?;
  for capability in capabilities {
    sqlx::query("INSERT INTO role_capabilities(role_id,capability) VALUES(?,?)")
      .bind(id)
      .bind(capability)
      .execute(&mut *conn)
      .await?;
  }
  Ok(())
}

pub async fn delete(
  conn: &mut SqliteConnection,
  id: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query("DELETE FROM roles WHERE id = ? AND builtin = 0")
    .bind(id)
    .execute(conn)
    .await?;
  Ok(())
}
