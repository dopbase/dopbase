use anyhow::{Context, Result};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use std::{fs, str::FromStr, time::Duration};

use crate::config::database_path;

/// Embedded database migrations, exposed so tests can drive `run`/`undo`.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// TODO: when all user has migrate fully migrate will remove legacy capabilities
/// to make sure our code is clean
/// PR: https://github.com/dopbase/dopbase/pull/140
/// Highest version recorded by the sequential naming scheme that shipped
/// through 0.1.9.
const LEGACY_MIGRATION_HEAD: i64 = 18;

/// SQLite database client backed by an async sqlx connection pool.
///
/// Cheap to clone because every clone shares the same underlying pool.
#[derive(Clone, Debug)]
pub struct DbClient {
  pool: SqlitePool,
}

impl DbClient {
  /// Open a pooled connection to the SQLite database at `url`
  /// (e.g. `sqlite://./dopbase.db`). The database file is created
  /// when missing.
  pub async fn connect(url: &str) -> Result<Self> {
    let database = (!url.contains(":memory:"))
      .then(|| database_path(url))
      .transpose()?;
    #[cfg(unix)]
    let database_existed = database.as_ref().is_some_and(|path| path.exists());
    if let Some(parent) = database.as_ref().and_then(|path| path.parent()) {
      fs::create_dir_all(parent)
        .with_context(|| format!("failed to create database directory {}", parent.display()))?;
    }
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

    #[cfg(unix)]
    if !database_existed && let Some(path) = database {
      use std::os::unix::fs::PermissionsExt;
      fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
        .with_context(|| format!("failed to secure database file {}", path.display()))?;
    }

    Ok(Self { pool })
  }

  /// Run the migrations embedded in the Dopbase executable.
  pub async fn migrate(&self) -> Result<()> {
    adopt_legacy_versions(&self.pool).await?;
    MIGRATOR.run(&self.pool).await?;
    Ok(())
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

  /// Flush committed WAL pages into the main database before shutdown.
  pub async fn checkpoint(&self) -> Result<(), sqlx::Error> {
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
      .execute(&self.pool)
      .await
      .map(|_| ())
  }

  /// Gracefully close all pooled connections.
  pub async fn close(&self) {
    self.pool.close().await;
  }
}

/// Carry databases created before migrations switched to timestamped file
/// names onto the current versions.
///
/// 0.1.9 and earlier recorded four-digit versions. The SQL files were only
/// renamed, so every stale row is matched to its current migration by checksum
/// and its version and description are rewritten before sqlx validates the
/// applied set. Rows that match no current migration, or more than one, are
/// left alone so sqlx still reports them.
async fn adopt_legacy_versions(pool: &SqlitePool) -> Result<()> {
  let has_migrations: bool = sqlx::query_scalar(
    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations')",
  )
  .fetch_one(pool)
  .await?;
  if !has_migrations {
    return Ok(());
  }

  let mut transaction = pool.begin().await?;
  let applied: Vec<(i64, Vec<u8>)> = sqlx::query_as(
    "SELECT version, checksum FROM _sqlx_migrations WHERE version <= ? AND success = 1",
  )
  .bind(LEGACY_MIGRATION_HEAD)
  .fetch_all(&mut *transaction)
  .await?;

  for (legacy_version, checksum) in applied {
    let mut matches = MIGRATOR.iter().filter(|migration| {
      !migration.migration_type.is_down_migration()
        && migration.checksum.as_ref() == checksum.as_slice()
    });
    let (Some(migration), None) = (matches.next(), matches.next()) else {
      continue;
    };
    sqlx::query("UPDATE _sqlx_migrations SET version = ?, description = ? WHERE version = ?")
      .bind(migration.version)
      .bind(migration.description.as_ref())
      .bind(legacy_version)
      .execute(&mut *transaction)
      .await?;
  }
  transaction.commit().await?;
  Ok(())
}
