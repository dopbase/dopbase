use app::{
  cli::commands::init::{
    EnvCleanup, classify_entries, remove_env_if_unchanged, render_detection_summary,
    validate_interactive_target,
  },
  models::SecretInput,
};
use std::{collections::HashSet, process::Command};
use tempfile::TempDir;

fn entries(keys: &[&str]) -> Vec<SecretInput> {
  keys
    .iter()
    .map(|key| SecretInput {
      key: (*key).into(),
      value: "private-test-value".into(),
    })
    .collect()
}

#[test]
fn interactive_init_classifies_sensitive_names_without_reading_values() {
  let detected = entries(&[
    "API_KEY",
    "stripe_secret",
    "AUTH_TOKEN",
    "DATABASE_URL",
    "AWS_ACCESS_KEY_ID",
    "client_password",
    "CREDENTIALS",
    "PRIVATE_KEY_PATH",
    "DB_URL",
    "APP_PORT",
    "LOG_LEVEL",
    "PUBLIC_ORIGIN",
  ]);

  assert_eq!(classify_entries(&detected), (9, 3));
  assert_eq!(
    render_detection_summary(12, 9, 3),
    "12 variables detected\n9 appear sensitive\n3 appear non-sensitive"
  );
  assert_eq!(
    render_detection_summary(1, 1, 0),
    "1 variable detected\n1 appears sensitive\n0 appear non-sensitive"
  );
}

#[test]
fn interactive_init_validates_targets_and_rejects_existing_projects() {
  let existing = HashSet::from(["payment-service".to_owned()]);

  assert_eq!(
    validate_interactive_target("billing/local", &existing).unwrap(),
    ("billing".to_owned(), "local".to_owned())
  );
  assert_eq!(
    validate_interactive_target("payment", &existing).unwrap_err(),
    "Environment target must use PROJECT/ENVIRONMENT, for example payment-service/local."
  );
  assert_eq!(
    validate_interactive_target("payment-service/local", &existing).unwrap_err(),
    "Project payment-service already exists. Enter a different project name."
  );
}

#[test]
fn interactive_init_deletes_only_the_file_that_was_imported() {
  let directory = TempDir::new().unwrap();
  let path = directory.path().join(".env");
  let imported = b"API_KEY=original\n";

  std::fs::write(&path, imported).unwrap();
  assert_eq!(
    remove_env_if_unchanged(&path, imported).unwrap(),
    EnvCleanup::Deleted
  );
  assert!(!path.exists());
  assert_eq!(
    remove_env_if_unchanged(&path, imported).unwrap(),
    EnvCleanup::Missing
  );

  std::fs::write(&path, b"API_KEY=changed\n").unwrap();
  assert_eq!(
    remove_env_if_unchanged(&path, imported).unwrap(),
    EnvCleanup::Changed
  );
  assert_eq!(std::fs::read(&path).unwrap(), b"API_KEY=changed\n");
}

#[test]
fn zero_argument_init_explains_non_interactive_usage_before_connecting() {
  let directory = TempDir::new().unwrap();
  let output = Command::new(env!("CARGO_BIN_EXE_dopbase"))
    .args(["--data-dir", directory.path().to_str().unwrap(), "init"])
    .output()
    .unwrap();

  assert!(!output.status.success(), "{output:?}");
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(
    stderr.contains("interactive init requires a terminal"),
    "{stderr}"
  );
  assert!(stderr.contains("--from"), "{stderr}");
}
