use anyhow::{Context, Result};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use std::{fs, str::FromStr, time::Duration};

use crate::config::database_path;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

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
  pub async fn connect(url: &str) -> Result<Self> {
    let database = (!url.contains(":memory:"))
      .then(|| database_path(url))
      .transpose()?;
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
    let directory = tempfile::TempDir::new().unwrap();
    let database = directory.path().join("pragmas.db");
    let db = DbClient::connect(&format!("sqlite://{}", database.display()))
      .await
      .unwrap();
    let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
      .fetch_one(db.pool())
      .await
      .unwrap();
    let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
      .fetch_one(db.pool())
      .await
      .unwrap();
    let busy_timeout: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
      .fetch_one(db.pool())
      .await
      .unwrap();
    let synchronous: i64 = sqlx::query_scalar("PRAGMA synchronous")
      .fetch_one(db.pool())
      .await
      .unwrap();
    assert_eq!(journal_mode, "wal");
    assert_eq!(foreign_keys, 1);
    assert_eq!(busy_timeout, 5_000);
    assert_eq!(synchronous, 1);
    db.checkpoint().await.unwrap();
    db.close().await;
  }

  #[cfg(unix)]
  #[tokio::test]
  async fn creates_owner_only_database_file() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::TempDir::new().unwrap();
    let database = directory.path().join("private").join("dopbase.db");
    let db = DbClient::connect(&format!("sqlite://{}", database.display()))
      .await
      .unwrap();
    assert_eq!(
      fs::metadata(database).unwrap().permissions().mode() & 0o777,
      0o600
    );
    db.close().await;
  }

  #[tokio::test]
  async fn migrations_apply_rollback_and_reapply() {
    const TABLES: [&str; 8] = [
      "instance_metadata",
      "admins",
      "sessions",
      "projects",
      "environments",
      "secrets",
      "runner_tokens",
      "audit_events",
    ];
    const INDEXES: [&str; 4] = [
      "sessions_token_hash_idx",
      "runner_tokens_token_hash_idx",
      "audit_events_created_idx",
      "audit_events_action_idx",
    ];

    let directory = tempfile::TempDir::new().unwrap();
    let database = directory.path().join("migration-cycle.db");
    let db = DbClient::connect(&format!("sqlite://{}", database.display()))
      .await
      .unwrap();

    MIGRATOR.run(db.pool()).await.unwrap();
    MIGRATOR.run(db.pool()).await.unwrap();
    assert_schema_objects(&db, "table", &TABLES, true).await;
    assert_schema_objects(&db, "index", &INDEXES, true).await;

    MIGRATOR.undo(db.pool(), 0).await.unwrap();
    assert_schema_objects(&db, "table", &TABLES, false).await;
    assert_schema_objects(&db, "index", &INDEXES, false).await;

    MIGRATOR.run(db.pool()).await.unwrap();
    assert_schema_objects(&db, "table", &TABLES, true).await;
    assert_schema_objects(&db, "index", &INDEXES, true).await;
    db.close().await;
  }

  #[tokio::test]
  async fn foreign_keys_cascade_dependent_records() {
    let directory = tempfile::TempDir::new().unwrap();
    let database = directory.path().join("cascade.db");
    let db = DbClient::connect(&format!("sqlite://{}", database.display()))
      .await
      .unwrap();
    MIGRATOR.run(db.pool()).await.unwrap();

    sqlx::query("INSERT INTO admins(id,email,password_hash,created_at,updated_at) VALUES('usr_test','admin@example.com','hash','now','now')").execute(db.pool()).await.unwrap();
    sqlx::query("INSERT INTO sessions(id,admin_id,kind,token_hash,created_at,last_used_at,recent_auth_at,idle_expires_at,absolute_expires_at) VALUES('ses_test','usr_test','cli',X'01','now','now','now','later','later')").execute(db.pool()).await.unwrap();
    sqlx::query("INSERT INTO projects(id,name,created_at,updated_at) VALUES('prj_test','project','now','now')").execute(db.pool()).await.unwrap();
    sqlx::query("INSERT INTO environments(id,project_id,name,created_at,updated_at) VALUES('env_test','prj_test','production','now','now')").execute(db.pool()).await.unwrap();
    sqlx::query("INSERT INTO secrets(environment_id,key,ciphertext,value_nonce,wrapped_key,key_nonce,created_at,updated_at) VALUES('env_test','API_KEY',X'01',X'02',X'03',X'04','now','now')").execute(db.pool()).await.unwrap();
    sqlx::query("INSERT INTO runner_tokens(id,environment_id,name,token_hash,created_at) VALUES('tok_test','env_test','runner',X'05','now')").execute(db.pool()).await.unwrap();

    sqlx::query("DELETE FROM admins WHERE id = 'usr_test'")
      .execute(db.pool())
      .await
      .unwrap();
    sqlx::query("DELETE FROM projects WHERE id = 'prj_test'")
      .execute(db.pool())
      .await
      .unwrap();

    for table in ["sessions", "environments", "secrets", "runner_tokens"] {
      let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(db.pool())
        .await
        .unwrap();
      assert_eq!(count, 0, "cascade did not clear {table}");
    }
    db.close().await;
  }

  async fn assert_schema_objects(
    db: &DbClient,
    object_type: &str,
    names: &[&str],
    expected: bool,
  ) {
    for name in names {
      let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = ? AND name = ?)",
      )
      .bind(object_type)
      .bind(name)
      .fetch_one(db.pool())
      .await
      .unwrap();
      assert_eq!(
        exists, expected,
        "unexpected state for {object_type} {name}"
      );
    }
  }
}
