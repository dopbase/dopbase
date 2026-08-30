use sqlx::SqlitePool;
pub async fn admin_by_email(
  pool: &SqlitePool,
  email: &str,
) -> Result<Option<(String, String, String)>, sqlx::Error> {
  sqlx::query_as("SELECT id,email,password_hash FROM admins WHERE email=? COLLATE NOCASE")
    .bind(email)
    .fetch_optional(pool)
    .await
}
pub async fn password_hash(
  pool: &SqlitePool,
  id: &str,
) -> Result<Option<String>, sqlx::Error> {
  sqlx::query_scalar("SELECT password_hash FROM admins WHERE id=?")
    .bind(id)
    .fetch_optional(pool)
    .await
}
