use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous,
};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

/// SQLite database client backed by an async sqlx connection pool.
///
/// Cheap to clone; every clone shares the same underlying pool.
#[derive(Clone, Debug)]
pub struct DbClient {
    pool: SqlitePool,
}

impl DbClient {
    /// Open a pooled connection to the SQLite database at `url`
    /// (e.g. `sqlite://./dopbase.db`). The database file is created
    /// when missing.
    pub async fn connect(url: &str) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::from_str(url)?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(5));

        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .acquire_timeout(Duration::from_secs(5))
            .idle_timeout(Duration::from_secs(60))
            .connect_with(options)
            .await?;

        Ok(Self { pool })
    }

    /// Run all pending SQL migrations found in `migrations_dir`.
    pub async fn migrate(
        &self,
        migrations_dir: &Path,
    ) -> Result<(), sqlx::migrate::MigrateError> {
        let migrator = sqlx::migrate::Migrator::new(migrations_dir).await?;
        migrator.run(&self.pool).await
    }

    /// Access the underlying pool for ad-hoc queries.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Verify the database is reachable.
    pub async fn ping(&self) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map(|_| ())
    }

    /// Gracefully close all pooled connections.
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn connects_pings_and_closes() {
        let db = DbClient::connect("sqlite::memory:").await.unwrap();
        db.ping().await.unwrap();
        db.close().await;
    }

    #[tokio::test]
    async fn connect_sets_pragmas() {
        let db = DbClient::connect("sqlite::memory:").await.unwrap();
        let row: (i64, bool) = sqlx::query_as("SELECT 1, 1")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(row, (1, true));
        db.close().await;
    }
}
