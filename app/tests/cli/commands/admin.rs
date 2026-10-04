use app::cli::commands::{
  complete_factory_reset, factory_reset_archive_path, factory_reset_confirmation_matches,
  validate_factory_reset_target,
};
use tempfile::TempDir;
#[test]
fn factory_reset_requires_the_exact_confirmation_phrase() {
  assert!(factory_reset_confirmation_matches(
    "please-wipe-out-system\n"
  ));
  assert!(factory_reset_confirmation_matches(
    "please-wipe-out-system\r\n"
  ));
  for rejected in [
    "",
    "PLEASE-WIPE-OUT-SYSTEM",
    "please wipe out system",
    "please-wipe-out-system ",
    " please-wipe-out-system",
  ] {
    assert!(
      !factory_reset_confirmation_matches(rejected),
      "{rejected:?}"
    );
  }
}

#[test]
fn factory_reset_requires_a_local_database_inside_the_data_directory() {
  let directory = TempDir::new().unwrap();
  let data_dir = directory.path().join("instance");
  let database = data_dir.join("dopbase.db");

  let missing = validate_factory_reset_target(&data_dir, &database)
    .unwrap_err()
    .to_string();
  assert!(missing.contains("only available on the Dopbase server host"));

  std::fs::create_dir(&data_dir).unwrap();
  std::fs::write(&database, b"database").unwrap();
  assert_eq!(
    validate_factory_reset_target(&data_dir, &database).unwrap(),
    data_dir.canonicalize().unwrap()
  );

  let outside_database = directory.path().join("outside.db");
  std::fs::write(&outside_database, b"database").unwrap();
  let outside = validate_factory_reset_target(&data_dir, &outside_database)
    .unwrap_err()
    .to_string();
  assert!(outside.contains("does not contain its database"));
}

#[test]
fn factory_reset_creates_a_zip_and_removes_the_data_directory() {
  use std::io::Read;

  let directory = TempDir::new().unwrap();
  let data_dir = directory.path().join("instance");
  let nested = data_dir.join("backups");
  std::fs::create_dir_all(&nested).unwrap();
  std::fs::write(data_dir.join("dopbase.db"), b"database").unwrap();
  std::fs::write(nested.join("existing.dop"), b"backup").unwrap();

  let archive = complete_factory_reset(&data_dir, false).unwrap().unwrap();
  assert!(!data_dir.exists());
  assert!(archive.exists());
  assert_eq!(archive.parent(), data_dir.parent());
  assert!(
    archive
      .file_name()
      .unwrap()
      .to_string_lossy()
      .starts_with("instance.factory-reset-")
  );
  assert_eq!(
    archive,
    factory_reset_archive_path(
      &archive.with_file_name(
        archive
          .file_name()
          .unwrap()
          .to_string_lossy()
          .trim_end_matches(".zip")
      )
    )
  );

  let file = std::fs::File::open(&archive).unwrap();
  let mut zip = zip::ZipArchive::new(file).unwrap();
  let root = "instance";
  let mut database = String::new();
  zip
    .by_name(&format!("{root}/dopbase.db"))
    .unwrap()
    .read_to_string(&mut database)
    .unwrap();
  assert_eq!(database, "database");
  assert!(zip.by_name(&format!("{root}/backups/existing.dop")).is_ok());

  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(
      std::fs::metadata(&archive).unwrap().permissions().mode() & 0o777,
      0o600
    );
  }
}

#[test]
fn factory_reset_no_backup_removes_data_without_an_archive() {
  let directory = TempDir::new().unwrap();
  let data_dir = directory.path().join("instance");
  std::fs::create_dir_all(&data_dir).unwrap();
  std::fs::write(data_dir.join("dopbase.db"), b"database").unwrap();

  assert!(complete_factory_reset(&data_dir, true).unwrap().is_none());
  assert!(!data_dir.exists());
  assert!(
    std::fs::read_dir(directory.path())
      .unwrap()
      .next()
      .is_none()
  );
}

#[cfg(unix)]
#[test]
fn factory_reset_restores_data_when_the_zip_cannot_include_a_symlink() {
  use std::os::unix::fs::symlink;

  let directory = TempDir::new().unwrap();
  let data_dir = directory.path().join("instance");
  std::fs::create_dir_all(&data_dir).unwrap();
  std::fs::write(data_dir.join("dopbase.db"), b"database").unwrap();
  symlink(directory.path(), data_dir.join("outside")).unwrap();

  let error = complete_factory_reset(&data_dir, false)
    .unwrap_err()
    .to_string();
  assert!(error.contains("ZIP backup could not be created"), "{error}");
  assert!(data_dir.exists());
  assert!(data_dir.join("outside").is_symlink());
  assert_eq!(
    std::fs::read_dir(directory.path())
      .unwrap()
      .filter_map(Result::ok)
      .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "zip"))
      .count(),
    0
  );
}

#[tokio::test]
async fn factory_reset_rejects_remote_and_json_modes_before_touching_local_state() {
  use clap::Parser;

  let remote = app::cli::args::Cli::try_parse_from([
    "dopbase",
    "--server",
    "https://dopbase.example.com",
    "admin",
    "factory-reset",
  ])
  .unwrap();
  assert_eq!(
    app::cli::commands::execute(remote)
      .await
      .unwrap_err()
      .to_string(),
    "--server cannot be used with local `dopbase admin` commands"
  );

  let json =
    app::cli::args::Cli::try_parse_from(["dopbase", "--json", "admin", "factory-reset"]).unwrap();
  assert_eq!(
    app::cli::commands::execute(json)
      .await
      .unwrap_err()
      .to_string(),
    "--json cannot be used with `dopbase admin factory-reset`"
  );
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn offline_password_reset_updates_the_account_and_revokes_sessions() {
  use super::support::{EMAIL, Fixture, command, terminal::Terminal};
  let fixture = Fixture::new().await;
  fixture.stop().await;
  let mut cmd = command(&fixture.data_dir);
  cmd.args(["admin", "reset-password", EMAIL]);
  let mut terminal = Terminal::new(cmd);
  let password = "replacement-password-123";
  terminal.reply("New password:", &format!("{password}\r"));
  terminal.reply("Confirm new password:", &format!("{password}\r"));
  assert!(terminal.finish().success());
  assert!(!terminal.transcript.contains(password));
  let db = app::services::db::DbClient::connect(&app::config::sqlite_url(
    &fixture.data_dir.join("dopbase.db"),
  ))
  .await
  .unwrap();
  let hash: String = sqlx::query_scalar("SELECT password_hash FROM admins WHERE email=?")
    .bind(EMAIL)
    .fetch_one(db.pool())
    .await
    .unwrap();
  assert!(
    argon2::PasswordVerifier::verify_password(
      &argon2::Argon2::default(),
      password.as_bytes(),
      &argon2::PasswordHash::new(&hash).unwrap()
    )
    .is_ok()
  );
  let active: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE revoked_at IS NULL")
    .fetch_one(db.pool())
    .await
    .unwrap();
  assert_eq!(active, 0);
  let audits: i64 =
    sqlx::query_scalar("SELECT COUNT(*) FROM audit_events WHERE action='admin.password_reset'")
      .fetch_one(db.pool())
      .await
      .unwrap();
  assert_eq!(audits, 1);
  db.close().await;
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn factory_reset_command_requires_a_stopped_instance_and_root_confirmation() {
  use super::support::{Fixture, PASSWORD, command, terminal::Terminal};
  let fixture = Fixture::new().await;
  fixture.stop().await;
  let key = std::fs::read(fixture.data_dir.join("master.key")).unwrap();
  let url = app::config::sqlite_url(&fixture.data_dir.join("dopbase.db"));
  let lock = app::server::InstanceLock::acquire(&url).unwrap();
  let mut cmd = command(&fixture.data_dir);
  cmd.args(["admin", "factory-reset"]);
  let mut terminal = Terminal::new(cmd);
  assert_eq!(terminal.finish().code(), Some(1));
  assert!(terminal.transcript.contains("fully stopped"));
  drop(lock);
  for (phrase, password, expected) in [
    ("wrong phrase\r", None, 1),
    ("\x03", None, 130),
    ("please-wipe-out-system\r", Some("wrong-password"), 1),
    ("please-wipe-out-system\r", Some(PASSWORD), 0),
  ] {
    let mut cmd = command(&fixture.data_dir);
    cmd.args(["admin", "factory-reset"]);
    let mut terminal = Terminal::new(cmd);
    terminal.reply("Type please-wipe-out-system", phrase);
    if let Some(password) = password {
      terminal.reply("Root password", &format!("{password}\r"));
      assert!(!terminal.transcript.contains(password));
    }
    assert_eq!(terminal.finish().code(), Some(expected));
    if expected == 0 {
      assert!(!fixture.data_dir.exists());
      assert!(
        std::fs::read_dir(fixture.directory.path())
          .unwrap()
          .any(|entry| entry
            .unwrap()
            .path()
            .extension()
            .is_some_and(|extension| extension == "zip"))
      );
    } else {
      assert_eq!(
        std::fs::read(fixture.data_dir.join("master.key")).unwrap(),
        key
      );
      let db = app::services::db::DbClient::connect(&url).await.unwrap();
      let accounts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admins")
        .fetch_one(db.pool())
        .await
        .unwrap();
      assert_eq!(accounts, 1);
      db.close().await;
    }
  }
}
