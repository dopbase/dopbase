use tempfile::TempDir;
#[tokio::test]
async fn backup_rejects_when_server_is_offline() {
  use clap::Parser;

  let dir = TempDir::new().unwrap();
  let cli = app::cli::args::Cli::try_parse_from([
    "dopbase",
    "--data-dir",
    dir.path().to_str().unwrap(),
    "--server",
    "http://127.0.0.1:1",
    "backup",
  ])
  .unwrap();

  let err = app::cli::commands::execute(cli).await.unwrap_err();
  let msg = err.to_string();
  assert!(
    msg.contains("Cannot perform backup: Dopbase server at http://127.0.0.1:1 is not connected or offline (live status required)"),
    "unexpected message: {msg}"
  );
}

#[tokio::test(flavor = "multi_thread")]
async fn backup_download_writes_a_private_archive_without_overwriting_files() {
  use super::support::{Fixture, failure};
  let fixture = Fixture::new().await;
  fixture.environment().await;
  let path = fixture.directory.path().join("snapshot.dop");
  let created = fixture
    .json(&["backup", "snapshot", "--output", path.to_str().unwrap()])
    .await;
  assert_eq!(created["key"], "snapshot.dop");
  assert_eq!(created["localPath"], path.to_str().unwrap());
  let contents = std::fs::read(&path).unwrap();
  assert!(contents.starts_with(b"DOPBASE_BK1\0"));
  assert_eq!(created["size"].as_u64().unwrap(), contents.len() as u64);
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(
      std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
      0o600
    );
  }
  failure(
    &fixture
      .run(&["backup", "second", "--output", path.to_str().unwrap()])
      .await,
    "already exists",
  );
  assert_eq!(std::fs::read(&path).unwrap(), contents);
}
