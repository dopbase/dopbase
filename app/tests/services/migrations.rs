use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use app::config::sqlite_url;
use app::services::db::{DbClient, MIGRATOR};
use chrono::NaiveDateTime;
use regex::Regex;

/// A timestamped migration a later branch would add after the current set.
const PROBE_MIGRATION: &str = "20261024091600_upgrade_probe";
const PROBE_VERSION: i64 = 20_261_024_091_600;
/// Highest version recorded before migrations were renamed.
const LEGACY_HEAD: i64 = 18;

fn migrations_dir() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations")
}

fn migration_files() -> Vec<String> {
  let mut names: Vec<String> = fs::read_dir(migrations_dir())
    .unwrap()
    .map(|entry| entry.unwrap().file_name().into_string().unwrap())
    .filter(|name| name.ends_with(".sql"))
    .collect();
  names.sort();
  names
}

fn timestamped_ups() -> Vec<String> {
  let mut names: Vec<String> = migration_files()
    .into_iter()
    .filter(|name| name.ends_with(".up.sql"))
    .collect();
  names.sort_by_key(|name| migration_version(name));
  names
}

fn migration_version(name: &str) -> i64 {
  name
    .split('_')
    .next()
    .and_then(|version| version.parse().ok())
    .unwrap_or_else(|| panic!("{name}: version is not a number"))
}

#[test]
fn database_migration_files_follow_the_naming_convention() {
  let timestamped = Regex::new(r"^([0-9]{14})_([a-z0-9_]+)\.(up|down)\.sql$").unwrap();

  for name in migration_files() {
    let captures = timestamped
      .captures(&name)
      .unwrap_or_else(|| panic!("{name}: expected YYYYMMDDHHMMSS_<name>.up.sql or .down.sql"));
    let version: i64 = captures[1].parse().unwrap();
    assert!(version > 0, "{name}: version must be positive");
    NaiveDateTime::parse_from_str(&captures[1], "%Y%m%d%H%M%S")
      .unwrap_or_else(|_| panic!("{name}: version is not a valid UTC timestamp"));
  }
}

#[test]
fn every_migration_file_has_a_matching_pair() {
  let directory = migrations_dir();
  let mut pairs = BTreeSet::new();
  for name in migration_files() {
    let pair = name
      .strip_suffix(".up.sql")
      .or_else(|| name.strip_suffix(".down.sql"))
      .unwrap_or_else(|| panic!("{name}: expected an .up.sql or .down.sql suffix"));
    pairs.insert(pair.to_owned());
  }

  for pair in pairs {
    for suffix in ["up", "down"] {
      let file = format!("{pair}.{suffix}.sql");
      assert!(directory.join(&file).is_file(), "{file} is missing");
    }
  }
}

#[tokio::test]
async fn applies_an_older_pending_migration_after_a_newer_one() {
  let directory = tempfile::TempDir::new().unwrap();
  let migrations = directory.path().join("migrations");
  fs::create_dir(&migrations).unwrap();
  write_migration(
    &migrations,
    "20260101000000_later",
    "CREATE TABLE later (id INTEGER PRIMARY KEY);",
    "DROP TABLE IF EXISTS later;",
  );

  let database_dir = tempfile::TempDir::new().unwrap();
  let database = database_dir.path().join("out-of-order.db");
  let db = DbClient::connect(&sqlite_url(&database)).await.unwrap();

  let later = sqlx::migrate::Migrator::new(migrations.as_path())
    .await
    .unwrap();
  later.run(db.pool()).await.unwrap();
  assert_table(&db, "later", true).await;

  // A branch written earlier merges after the newer migration was applied.
  write_migration(
    &migrations,
    "20250101000000_earlier",
    "CREATE TABLE earlier (id INTEGER PRIMARY KEY);",
    "DROP TABLE IF EXISTS earlier;",
  );

  let both = sqlx::migrate::Migrator::new(migrations.as_path())
    .await
    .unwrap();
  both.run(db.pool()).await.unwrap();

  assert_table(&db, "earlier", true).await;
  assert_table(&db, "later", true).await;
  let versions: Vec<i64> =
    sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
      .fetch_all(db.pool())
      .await
      .unwrap();
  assert_eq!(versions, [20250101000000, 20260101000000]);
  db.close().await;
}

#[tokio::test]
async fn applies_a_new_migration_to_an_existing_database() {
  let database_dir = tempfile::TempDir::new().unwrap();
  let database = database_dir.path().join("existing.db");
  let db = DbClient::connect(&sqlite_url(&database)).await.unwrap();
  MIGRATOR.run(db.pool()).await.unwrap();

  // Mirror the migrations a build with a new timestamped migration would
  // embed, then apply it to a database that already has the current versions.
  let directory = tempfile::TempDir::new().unwrap();
  let migrations = directory.path().join("migrations");
  fs::create_dir(&migrations).unwrap();
  for name in migration_files() {
    fs::copy(migrations_dir().join(&name), migrations.join(&name)).unwrap();
  }
  write_migration(
    &migrations,
    PROBE_MIGRATION,
    "CREATE TABLE upgrade_probe (id INTEGER PRIMARY KEY);",
    "DROP TABLE IF EXISTS upgrade_probe;",
  );

  let upgraded = sqlx::migrate::Migrator::new(migrations.as_path())
    .await
    .unwrap();
  upgraded.run(db.pool()).await.unwrap();
  assert_table(&db, "upgrade_probe", true).await;

  upgraded.undo(db.pool(), PROBE_VERSION - 1).await.unwrap();
  assert_table(&db, "upgrade_probe", false).await;

  // The embedded migrator still accepts the current checksums and finds
  // nothing left to apply.
  MIGRATOR.run(db.pool()).await.unwrap();
  db.close().await;
}

#[tokio::test]
async fn rewrites_versions_recorded_before_the_timestamped_rename() {
  let database_dir = tempfile::TempDir::new().unwrap();
  let database = database_dir.path().join("legacy.db");
  let db = DbClient::connect(&sqlite_url(&database)).await.unwrap();

  // Rebuild what a 0.1.9 database recorded: the same SQL under four-digit
  // versions. The migrator matches rows by checksum, so the descriptions here
  // do not matter.
  let directory = tempfile::TempDir::new().unwrap();
  let legacy = directory.path().join("legacy");
  fs::create_dir(&legacy).unwrap();
  for (index, name) in timestamped_ups()
    .iter()
    .take(LEGACY_HEAD as usize)
    .enumerate()
  {
    let version = index + 1;
    fs::copy(
      migrations_dir().join(name),
      legacy.join(format!("{version:04}_legacy.up.sql")),
    )
    .unwrap();
    fs::write(legacy.join(format!("{version:04}_legacy.down.sql")), "").unwrap();
  }
  let migrator = sqlx::migrate::Migrator::new(legacy.as_path())
    .await
    .unwrap();
  migrator.run(db.pool()).await.unwrap();
  let versions: Vec<i64> =
    sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
      .fetch_all(db.pool())
      .await
      .unwrap();
  assert_eq!(versions, (1..=LEGACY_HEAD).collect::<Vec<_>>());

  // Startup runs this pass on every launch; a second one must be a no-op.
  db.migrate().await.unwrap();
  db.migrate().await.unwrap();

  let rows: Vec<(i64, String)> =
    sqlx::query_as("SELECT version, description FROM _sqlx_migrations ORDER BY version")
      .fetch_all(db.pool())
      .await
      .unwrap();
  assert_eq!(rows.len(), timestamped_ups().len());
  assert!(rows.iter().all(|(version, _)| *version > LEGACY_HEAD));
  assert_eq!(
    rows
      .first()
      .map(|(version, description)| (*version, description.as_str())),
    Some((20260829031127, "instance metadata"))
  );
  assert!(
    rows
      .iter()
      .any(|(version, description)| *version == 20261024010048 && description == "secret versions"),
    "the last 0.1.9 migration keeps its checksum"
  );
  db.close().await;
}

fn write_migration(
  directory: &Path,
  stem: &str,
  up: &str,
  down: &str,
) {
  fs::write(directory.join(format!("{stem}.up.sql")), up).unwrap();
  fs::write(directory.join(format!("{stem}.down.sql")), down).unwrap();
}

async fn assert_table(
  db: &DbClient,
  name: &str,
  expected: bool,
) {
  let exists: bool = sqlx::query_scalar(
    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?)",
  )
  .bind(name)
  .fetch_one(db.pool())
  .await
  .unwrap();
  assert_eq!(exists, expected, "unexpected state for table {name}");
}

/// Highest four-digit version a fork database recorded before 0.1.10.
const FORK_LEGACY_HEAD: i64 = 17;

#[tokio::test]
async fn rewrites_fork_workspace_legacy_versions() {
  let database_dir = tempfile::TempDir::new().unwrap();
  let database = database_dir.path().join("fork-legacy.db");
  let db = DbClient::connect(&sqlite_url(&database)).await.unwrap();

  let directory = tempfile::TempDir::new().unwrap();
  let legacy = directory.path().join("legacy");
  fs::create_dir(&legacy).unwrap();
  let ups = timestamped_ups();
  for version in 1..=13 {
    fs::copy(
      migrations_dir().join(&ups[version - 1]),
      legacy.join(format!("{version:04}_legacy.up.sql")),
    )
    .unwrap();
    fs::write(legacy.join(format!("{version:04}_legacy.down.sql")), "").unwrap();
  }
  let fork_only = [
    "20261024132209_workspaces.up.sql",
    "20261024132210_default_workspace.up.sql",
    "20261024132211_project_names_per_workspace.up.sql",
    "20261024132212_workspace_tokens.up.sql",
  ];
  for (index, name) in fork_only.iter().enumerate() {
    let version = 14 + index;
    fs::copy(
      migrations_dir().join(name),
      legacy.join(format!("{version:04}_legacy.up.sql")),
    )
    .unwrap();
    fs::write(legacy.join(format!("{version:04}_legacy.down.sql")), "").unwrap();
  }

  let migrator = sqlx::migrate::Migrator::new(legacy.as_path())
    .await
    .unwrap();
  migrator.run(db.pool()).await.unwrap();

  db.migrate().await.unwrap();
  db.migrate().await.unwrap();

  assert_table(&db, "workspaces", true).await;
  assert_table(&db, "workspace_tokens", true).await;
  assert_table(&db, "sso_providers", true).await;

  let versions: Vec<i64> =
    sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
      .fetch_all(db.pool())
      .await
      .unwrap();
  assert!(versions.contains(&20261024132212));
  assert!(versions.iter().all(|version| *version > FORK_LEGACY_HEAD));
  db.close().await;
}
