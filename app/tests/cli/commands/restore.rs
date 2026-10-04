use tempfile::TempDir;
#[tokio::test]
async fn restore_rejects_when_server_is_offline() {
  use clap::Parser;

  let dir = TempDir::new().unwrap();
  let backup_file = dir.path().join("test_backup.dop");
  std::fs::write(&backup_file, b"dummy content").unwrap();

  let cli = app::cli::args::Cli::try_parse_from([
    "dopbase",
    "--data-dir",
    dir.path().to_str().unwrap(),
    "--server",
    "http://127.0.0.1:1",
    "restore",
    backup_file.to_str().unwrap(),
    "--yes",
  ])
  .unwrap();

  let err = app::cli::commands::execute(cli).await.unwrap_err();
  let msg = err.to_string();
  assert!(
    msg.contains("Cannot perform restore: Dopbase server at http://127.0.0.1:1 is not connected or offline (live status required)"),
    "unexpected message: {msg}"
  );
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn restore_commands_upload_to_initialized_and_bootstrap_instances() {
  use super::support::{Fixture, PASSWORD, failure, terminal::Terminal};
  let source = Fixture::new().await;
  source.environment().await;
  let path = source.directory.path().join("snapshot.dop");
  source
    .json(&["backup", "cli-restore", "--output", path.to_str().unwrap()])
    .await;
  failure(
    &source.run(&["restore", "missing.dop", "--yes"]).await,
    "Backup file not found",
  );
  let invalid = source.directory.path().join("invalid.txt");
  std::fs::write(&invalid, b"preserve").unwrap();
  failure(
    &source
      .run(&["restore", invalid.to_str().unwrap(), "--yes"])
      .await,
    ".dop extension",
  );
  failure(
    &source.run(&["restore", path.to_str().unwrap()]).await,
    "Pass --yes",
  );
  source.json(&["project", "create", "after-snapshot"]).await;
  let mut command = source.command();
  command.args(["--json", "restore", path.to_str().unwrap(), "--yes"]);
  let mut terminal = Terminal::new(command);
  terminal.reply("Password:", &format!("{PASSWORD}\r"));
  assert!(terminal.finish().success());
  assert!(terminal.transcript.contains("\"restored\": true"));
  assert!(!terminal.transcript.contains(PASSWORD));
  let projects = source.json(&["project", "list"]).await;
  assert_eq!(projects.as_array().unwrap().len(), 1);
  assert_eq!(projects[0]["name"], "fixture");
  let target = Fixture::uninitialized().await;
  failure(
    &target
      .run(&["restore", path.to_str().unwrap(), "--yes"])
      .await,
    "--setup-token is required",
  );
  let token = target.state.setup.read().await.token.clone().unwrap();
  let key = source.data_dir.join("master.key");
  let restored = target
    .json(&[
      "restore",
      path.to_str().unwrap(),
      "--key",
      key.to_str().unwrap(),
      "--setup-token",
      &token,
      "--yes",
    ])
    .await;
  assert_eq!(restored["restored"], true);
  let initialized: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects WHERE name='fixture'")
    .fetch_one(target.state.db.pool())
    .await
    .unwrap();
  assert_eq!(initialized, 1);
  assert!(target.state.setup.read().await.token.is_none());
}
