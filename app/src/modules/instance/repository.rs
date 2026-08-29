pub async fn initialized(pool: &sqlx::SqlitePool) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM admins")
        .fetch_one(pool)
        .await
        .map(|count| count > 0)
}
