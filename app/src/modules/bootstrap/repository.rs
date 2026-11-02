use sqlx::{Sqlite, Transaction};
pub async fn admin_count(pool: &sqlx::SqlitePool) -> Result<i64, sqlx::Error> {
  sqlx::query_scalar("SELECT COUNT(*) FROM admins")
    .fetch_one(pool)
    .await
}
pub async fn insert_admin(
  tx: &mut Transaction<'_, Sqlite>,
  id: &str,
  email: &str,
  password_hash: &str,
  now: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query("INSERT INTO admins(id,email,password_hash,created_at,updated_at,role,is_root) VALUES(?,?,?,?,?,'root',1)")
    .bind(id)
    .bind(email)
    .bind(password_hash)
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map(|_| ())
}
