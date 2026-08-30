use std::fs;

use app::services::db::{DbClient, MIGRATOR};

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
  sqlx::query(
    "INSERT INTO projects(id,name,created_at,updated_at) VALUES('prj_test','project','now','now')",
  )
  .execute(db.pool())
  .await
  .unwrap();
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
    let exists: bool =
      sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = ? AND name = ?)")
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
