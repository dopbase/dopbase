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
    sqlx::query(
        "INSERT INTO admins(id,email,password_hash,created_at,updated_at) VALUES(?,?,?,?,?)",
    )
    .bind(id)
    .bind(email)
    .bind(password_hash)
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map(|_| ())
}
#[allow(clippy::too_many_arguments)]
pub async fn insert_session(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
    admin_id: &str,
    token_hash: &[u8],
    csrf_hash: &[u8],
    now: &str,
    idle: &str,
    absolute: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO sessions(id,admin_id,kind,token_hash,csrf_hash,created_at,last_used_at,recent_auth_at,idle_expires_at,absolute_expires_at) VALUES(?,?,'browser',?,?,?,?,?,?,?)").bind(id).bind(admin_id).bind(token_hash).bind(csrf_hash).bind(now).bind(now).bind(now).bind(idle).bind(absolute).execute(&mut **tx).await.map(|_|())
}
