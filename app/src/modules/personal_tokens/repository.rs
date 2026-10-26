use super::model::PersonalTokenResponse;
use crate::{modules::grants::model::ScopeType, services::authx::Level};
use sqlx::{SqliteConnection, SqlitePool};

const SELECT: &str = "SELECT t.id, t.user_id, t.name, t.scope_type, t.scope_id, \
  COALESCE(CASE t.scope_type WHEN 'project' THEN (SELECT name FROM projects WHERE id = t.scope_id) WHEN 'environment' THEN (SELECT p.name || '/' || e.name FROM environments e JOIN projects p ON p.id = e.project_id WHERE e.id = t.scope_id) END, '') AS scope_label, \
  t.level, t.created_by, t.created_at, t.expires_at, t.last_used_at, t.revoked_at FROM personal_tokens t";

#[derive(sqlx::FromRow)]
struct TokenRow {
  id: String,
  user_id: String,
  name: String,
  scope_type: String,
  scope_id: String,
  scope_label: String,
  level: String,
  created_by: Option<String>,
  created_at: String,
  expires_at: Option<String>,
  last_used_at: Option<String>,
  revoked_at: Option<String>,
}

impl TokenRow {
  /// Audit tokens are left out until they can be listed as such.
  fn response(self) -> Option<PersonalTokenResponse> {
    Some(PersonalTokenResponse {
      id: self.id,
      user_id: self.user_id,
      name: self.name,
      scope_type: ScopeType::parse(&self.scope_type)?,
      scope_id: self.scope_id,
      scope_label: self.scope_label,
      level: Level::parse(&self.level)?,
      created_by: self.created_by,
      created_at: self.created_at,
      expires_at: self.expires_at,
      last_used_at: self.last_used_at,
      revoked_at: self.revoked_at,
    })
  }
}

pub async fn list(
  pool: &SqlitePool,
  user_id: &str,
) -> Result<Vec<PersonalTokenResponse>, sqlx::Error> {
  let rows: Vec<TokenRow> = sqlx::query_as(&format!(
    "{SELECT} WHERE t.user_id = ? ORDER BY t.created_at DESC"
  ))
  .bind(user_id)
  .fetch_all(pool)
  .await?;
  Ok(rows.into_iter().filter_map(TokenRow::response).collect())
}

pub async fn find(
  conn: &mut SqliteConnection,
  user_id: &str,
  id: &str,
) -> Result<Option<PersonalTokenResponse>, sqlx::Error> {
  let row: Option<TokenRow> = sqlx::query_as(&format!("{SELECT} WHERE t.user_id = ? AND t.id = ?"))
    .bind(user_id)
    .bind(id)
    .fetch_optional(conn)
    .await?;
  Ok(row.and_then(TokenRow::response))
}

pub struct NewToken<'a> {
  pub id: &'a str,
  pub user_id: &'a str,
  pub name: &'a str,
  pub token_hash: Vec<u8>,
  pub scope_type: ScopeType,
  pub scope_id: &'a str,
  pub level: Level,
  pub created_by: &'a str,
  pub created_at: &'a str,
  pub expires_at: &'a str,
}

/// Fails with a unique violation when the account has a token by that name.
pub async fn insert(
  conn: &mut SqliteConnection,
  token: NewToken<'_>,
) -> Result<(), sqlx::Error> {
  sqlx::query("INSERT INTO personal_tokens(id,user_id,name,token_hash,scope_type,scope_id,level,created_by,created_at,expires_at) VALUES(?,?,?,?,?,?,?,?,?,?)")
    .bind(token.id)
    .bind(token.user_id)
    .bind(token.name)
    .bind(token.token_hash)
    .bind(token.scope_type.as_str())
    .bind(token.scope_id)
    .bind(token.level.as_str())
    .bind(token.created_by)
    .bind(token.created_at)
    .bind(token.expires_at)
    .execute(conn)
    .await?;
  Ok(())
}

/// Returns false when the token was already revoked.
pub async fn revoke(
  conn: &mut SqliteConnection,
  user_id: &str,
  id: &str,
  now: &str,
) -> Result<bool, sqlx::Error> {
  Ok(
    sqlx::query("UPDATE personal_tokens SET revoked_at = ? WHERE user_id = ? AND id = ? AND revoked_at IS NULL")
      .bind(now)
      .bind(user_id)
      .bind(id)
      .execute(conn)
      .await?
      .rows_affected()
      == 1,
  )
}

/// The account a token would belong to, and its role's token policy.
#[derive(sqlx::FromRow)]
pub struct Holder {
  pub email: String,
  pub is_root: bool,
  pub owner_id: Option<String>,
  pub sign_in: Option<String>,
  pub token_scope: Option<String>,
  pub token_max_level: Option<String>,
  pub token_default_ttl_days: Option<i64>,
  pub token_max_ttl_days: Option<i64>,
}

pub async fn holder(
  pool: &SqlitePool,
  user_id: &str,
) -> Result<Option<Holder>, sqlx::Error> {
  sqlx::query_as("SELECT a.email, a.is_root = 1 AS is_root, a.owner_id, r.sign_in, r.token_scope, r.token_max_level, r.token_default_ttl_days, r.token_max_ttl_days FROM admins a LEFT JOIN roles r ON r.id = a.role_id WHERE a.id = ?")
    .bind(user_id)
    .fetch_optional(pool)
    .await
}
