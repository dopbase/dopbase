use app::{
  cli::{
    commands::cache::{
      CacheAge,
      store::{self, CachedRuntime},
    },
    local_config::{ClientConfig, ResolvedServer, ServerSource},
  },
  models::SecretInput,
};
use chrono::{Duration, TimeZone, Utc};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, process::Command, str::FromStr};
use tempfile::TempDir;

const SERVER_URL: &str = "https://cache.example.com";
const TOKEN: &str = "dbt_cache_credential_marker";
const SECRET_KEY: &str = "PRIVATE_CACHE_KEY_MARKER";
const SECRET_VALUE: &str = "private-cache-value-marker";

fn server(directory: &TempDir) -> ResolvedServer {
  ResolvedServer {
    url: SERVER_URL.into(),
    source: ServerSource::Argument,
    config_path: directory.path().join("config.toml"),
    config: ClientConfig {
      version: 1,
      server_url: None,
      default_environment: None,
    },
  }
}

fn cache_paths(server: &ResolvedServer) -> (PathBuf, PathBuf, PathBuf) {
  let root = server.config_path.parent().unwrap();
  let directory = root.join("run-cache");
  let name = format!("{:x}", Sha256::digest(server.url.as_bytes()));
  (
    root.join("run-cache-key"),
    directory.join(&name),
    directory.join(format!("{name}.lock")),
  )
}

fn cached_runtime(
  environment: &str,
  id: &str,
  fetched_at: chrono::DateTime<Utc>,
) -> CachedRuntime {
  CachedRuntime {
    project: "billing".into(),
    environment: environment.into(),
    environment_id: id.into(),
    aliases: vec![format!("alias-{environment}")],
    fetched_at,
    entries: vec![SecretInput {
      key: SECRET_KEY.into(),
      value: SECRET_VALUE.into(),
    }],
  }
}

fn save(
  server: &ResolvedServer,
  runtime: &CachedRuntime,
) {
  store::save(
    server,
    TOKEN,
    &format!("billing/{}", runtime.environment),
    runtime,
  )
  .unwrap();
}

fn command(directory: &TempDir) -> Command {
  let mut command = Command::new(env!("CARGO_BIN_EXE_dopbase"));
  command
    .env("DOPBASE_TOKEN", TOKEN)
    .args(["--data-dir", directory.path().to_str().unwrap()])
    .args(["--server", SERVER_URL]);
  command
}

#[test]
fn cache_age_accepts_supported_units_and_rejects_invalid_values() {
  for (value, seconds) in [("9s", 9), ("3m", 180), ("2h", 7200), ("14d", 1_209_600)] {
    assert_eq!(CacheAge::from_str(value).unwrap().seconds(), seconds);
  }
  for value in ["", "14", "0d", "-1d", "1w", "1.5d", "一天"] {
    assert!(CacheAge::from_str(value).is_err(), "{value}");
  }
}

#[test]
fn missing_cache_lists_as_empty_without_requiring_authentication() {
  let directory = TempDir::new().unwrap();
  let output = command(&directory)
    .env("DOPBASE_TOKEN", "")
    .args(["--json", "cache", "list"])
    .output()
    .unwrap();
  assert!(output.status.success(), "{output:?}");
  let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
  assert_eq!(value["entry_count"], 0);
  assert!(!directory.path().join("run-cache").exists());
}

#[test]
fn inspection_lists_metadata_without_exposing_cached_secrets() {
  let directory = TempDir::new().unwrap();
  let server = server(&directory);
  let now = Utc.with_ymd_and_hms(2026, 9, 12, 12, 0, 0).unwrap();
  save(
    &server,
    &cached_runtime("production", "env_prod", now - Duration::days(20)),
  );
  save(
    &server,
    &cached_runtime("staging", "env_stage", now - Duration::hours(3)),
  );

  let snapshot = store::inspect(&server, Some(TOKEN), now).unwrap();
  assert_eq!(snapshot.server_url, SERVER_URL);
  assert_eq!(snapshot.entries.len(), 2);
  assert_eq!(snapshot.entries[0].environment, "production");
  assert_eq!(snapshot.entries[0].age, "20d");
  assert!(snapshot.entries[0].aliases.contains(&"env_prod".into()));

  let output = command(&directory)
    .args(["--json", "cache", "list"])
    .output()
    .unwrap();
  assert!(output.status.success(), "{output:?}");
  let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
  assert_eq!(value["server_url"], SERVER_URL);
  assert_eq!(value["entry_count"], 2);
  let rendered = String::from_utf8(output.stdout).unwrap();
  for private in [TOKEN, SECRET_KEY, SECRET_VALUE, "ciphertext", "nonce"] {
    assert!(!rendered.contains(private), "output exposed {private}");
  }
}

#[test]
fn age_cleanup_is_strict_at_the_boundary_and_preserves_concurrent_refreshes() {
  let directory = TempDir::new().unwrap();
  let server = server(&directory);
  let now = Utc.with_ymd_and_hms(2026, 9, 12, 12, 0, 0).unwrap();
  save(
    &server,
    &cached_runtime(
      "old",
      "env_old",
      now - Duration::days(14) - Duration::seconds(1),
    ),
  );
  save(
    &server,
    &cached_runtime("boundary", "env_boundary", now - Duration::days(14)),
  );
  save(
    &server,
    &cached_runtime("fresh", "env_fresh", now - Duration::days(1)),
  );

  let snapshot = store::inspect(&server, Some(TOKEN), now).unwrap();
  let candidates = snapshot
    .entries
    .iter()
    .filter(|entry| entry.is_older_than(now, 14 * 24 * 60 * 60))
    .map(|entry| entry.candidate())
    .collect::<Vec<_>>();
  assert_eq!(candidates.len(), 1);

  save(
    &server,
    &cached_runtime("old", "env_old", now + Duration::seconds(1)),
  );
  let removal = store::remove(&server, TOKEN, &candidates, now).unwrap();
  assert!(removal.removed.is_empty());
  assert_eq!(removal.remaining_count, 3);
}

#[test]
fn dry_run_makes_no_changes_and_default_cleanup_removes_only_old_entries() {
  let directory = TempDir::new().unwrap();
  let server = server(&directory);
  let now = Utc::now();
  save(
    &server,
    &cached_runtime("old", "env_old", now - Duration::days(20)),
  );
  save(
    &server,
    &cached_runtime("fresh", "env_fresh", now - Duration::days(1)),
  );
  let (_, cache_path, lock_path) = cache_paths(&server);
  let before_cache = fs::read(&cache_path).unwrap();
  let before_lock = fs::read(&lock_path).unwrap();

  let dry = command(&directory)
    .args(["--json", "cache", "clean", "--dry-run"])
    .output()
    .unwrap();
  assert!(dry.status.success(), "{dry:?}");
  let value: serde_json::Value = serde_json::from_slice(&dry.stdout).unwrap();
  assert_eq!(value["dry_run"], true);
  assert_eq!(value["older_than"], "14d");
  assert_eq!(value["matched_count"], 1);
  assert_eq!(value["removed_count"], 0);
  assert_eq!(fs::read(&cache_path).unwrap(), before_cache);
  assert_eq!(fs::read(&lock_path).unwrap(), before_lock);

  let clean = command(&directory)
    .args(["--json", "cache", "clean", "--yes"])
    .output()
    .unwrap();
  assert!(clean.status.success(), "{clean:?}");
  let value: serde_json::Value = serde_json::from_slice(&clean.stdout).unwrap();
  assert_eq!(value["removed_count"], 1);
  assert_eq!(value["remaining_count"], 1);
  let snapshot = store::inspect(&server, Some(TOKEN), Utc::now()).unwrap();
  assert_eq!(snapshot.entries.len(), 1);
  assert_eq!(snapshot.entries[0].environment_id, "env_fresh");
  assert_eq!(
    fs::read_dir(cache_path.parent().unwrap())
      .unwrap()
      .filter_map(Result::ok)
      .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp"))
      .count(),
    0
  );
}

#[test]
fn destructive_cleanup_requires_confirmation_and_all_removes_the_cache_document() {
  let directory = TempDir::new().unwrap();
  let server = server(&directory);
  save(&server, &cached_runtime("fresh", "env_fresh", Utc::now()));
  let (key_path, cache_path, lock_path) = cache_paths(&server);

  let refused = command(&directory)
    .args(["cache", "clean", "--all"])
    .output()
    .unwrap();
  assert!(!refused.status.success());
  assert!(String::from_utf8_lossy(&refused.stderr).contains("Pass --yes"));
  assert!(cache_path.exists());

  let removed = command(&directory)
    .args(["--json", "cache", "clean", "--all", "--yes"])
    .output()
    .unwrap();
  assert!(removed.status.success(), "{removed:?}");
  let value: serde_json::Value = serde_json::from_slice(&removed.stdout).unwrap();
  assert_eq!(value["all"], true);
  assert!(value["older_than"].is_null());
  assert_eq!(value["removed_count"], 1);
  assert!(!cache_path.exists());
  assert!(key_path.exists());
  assert!(lock_path.exists());

  let empty = command(&directory)
    .args(["--json", "cache", "list"])
    .output()
    .unwrap();
  assert!(empty.status.success(), "{empty:?}");
  let value: serde_json::Value = serde_json::from_slice(&empty.stdout).unwrap();
  assert_eq!(value["entry_count"], 0);
}

#[test]
fn corrupt_cache_is_reported_with_recovery_steps_and_never_changed() {
  let directory = TempDir::new().unwrap();
  let server = server(&directory);
  save(
    &server,
    &cached_runtime("production", "env_prod", Utc::now() - Duration::days(30)),
  );
  let (_, cache_path, _) = cache_paths(&server);
  let mut damaged = fs::read(&cache_path).unwrap();
  let last = damaged.len() - 1;
  damaged[last] ^= 1;
  fs::write(&cache_path, &damaged).unwrap();

  let output = command(&directory)
    .args(["cache", "clean", "--all", "--yes"])
    .output()
    .unwrap();
  assert!(!output.status.success());
  let error = String::from_utf8_lossy(&output.stderr);
  assert!(error.contains(cache_path.to_str().unwrap()), "{error}");
  assert!(error.contains("The file was not changed"), "{error}");
  assert!(
    error.contains("move it out of the run-cache directory"),
    "{error}"
  );
  assert_eq!(fs::read(cache_path).unwrap(), damaged);
}
