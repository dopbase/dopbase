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
  assert!(
    terminal
      .transcript
      .contains(&format!("Instance: {}", fixture.data_dir.display())),
    "{}",
    terminal.transcript
  );
  assert!(!terminal.transcript.contains("pause"), "server was stopped");
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
  let audits: i64 = sqlx::query_scalar(
    "SELECT COUNT(*) FROM audit_events WHERE action='admin.password_reset' AND json_extract(metadata,'$.serverRunning')=0",
  )
  .fetch_one(db.pool())
  .await
  .unwrap();
  assert_eq!(audits, 1);
  assert!(
    app::services::audit_chain::verify(db.pool())
      .await
      .unwrap()
      .ok
  );
  db.close().await;
}

#[cfg(unix)]
mod reset_password {
  use super::super::support::{EMAIL, Fixture, PASSWORD, command, terminal::Terminal};
  use serde_json::json;

  const NEW_PASSWORD: &str = "replacement-password-123";

  fn reset(data_dir: &std::path::Path) -> Terminal {
    let mut cmd = command(data_dir);
    cmd.args(["admin", "reset-password", EMAIL]);
    Terminal::new(cmd)
  }

  async fn login(
    fixture: &Fixture,
    password: &str,
  ) -> u16 {
    reqwest::Client::new()
      .post(format!("{}/api/v1/auth/login", fixture.url))
      .json(&json!({"email":EMAIL,"password":password,"sessionKind":"cli"}))
      .send()
      .await
      .unwrap()
      .status()
      .as_u16()
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn it_works_while_the_server_is_running() {
    let fixture = Fixture::new().await;
    // The fixture serves in-process without the instance lock, so take it
    // here to look like a running server.
    let _lock = app::server::InstanceLock::acquire(&app::config::sqlite_url(
      &fixture.data_dir.join("dopbase.db"),
    ))
    .unwrap();
    let mut terminal = reset(&fixture.data_dir);
    terminal.reply("New password:", &format!("{NEW_PASSWORD}\r"));
    terminal.reply("Confirm new password:", &format!("{NEW_PASSWORD}\r"));
    assert!(terminal.finish().success(), "{}", terminal.transcript);
    assert!(terminal.transcript.contains("pause still runs out"));

    assert_eq!(login(&fixture, PASSWORD).await, 401);
    assert_eq!(login(&fixture, NEW_PASSWORD).await, 200);
    let revoked = reqwest::Client::new()
      .get(format!("{}/api/v1/projects", fixture.url))
      .bearer_auth(&fixture.token)
      .send()
      .await
      .unwrap()
      .status();
    assert_eq!(revoked, 401, "the old session stopped working");

    let pool = fixture.state.db.pool();
    let running: i64 = sqlx::query_scalar(
      "SELECT COUNT(*) FROM audit_events WHERE action='admin.password_reset' AND json_extract(metadata,'$.serverRunning')=1",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(running, 1);
    assert!(app::services::audit_chain::verify(pool).await.unwrap().ok);
  }

  #[tokio::test]
  async fn it_creates_nothing_where_there_is_no_instance() {
    let directory = tempfile::TempDir::new().unwrap();
    let missing = directory.path().join("missing");
    let empty = directory.path().join("empty");
    std::fs::create_dir(&empty).unwrap();
    for data_dir in [&missing, &empty] {
      let mut terminal = reset(data_dir);
      assert_eq!(terminal.finish().code(), Some(1));
      assert!(
        terminal.transcript.contains("there's no Dopbase instance"),
        "{}",
        terminal.transcript
      );
    }
    assert!(!missing.exists());
    assert_eq!(std::fs::read_dir(&empty).unwrap().count(), 0);
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn it_refuses_a_database_a_newer_version_upgraded() {
    let fixture = Fixture::new().await;
    let pool = fixture.state.db.pool();
    sqlx::query("INSERT INTO _sqlx_migrations(version,description,installed_on,success,checksum,execution_time) VALUES(99991231000000,'from the future',CURRENT_TIMESTAMP,1,x'00',0)")
      .execute(pool)
      .await
      .unwrap();
    let before: String = sqlx::query_scalar("SELECT password_hash FROM admins WHERE email=?")
      .bind(EMAIL)
      .fetch_one(pool)
      .await
      .unwrap();
    fixture.stop().await;

    let mut terminal = reset(&fixture.data_dir);
    assert_eq!(terminal.finish().code(), Some(1));
    assert!(
      terminal.transcript.contains("a newer version of Dopbase"),
      "{}",
      terminal.transcript
    );
    let db = app::services::db::DbClient::connect(&app::config::sqlite_url(
      &fixture.data_dir.join("dopbase.db"),
    ))
    .await
    .unwrap();
    let after: String = sqlx::query_scalar("SELECT password_hash FROM admins WHERE email=?")
      .bind(EMAIL)
      .fetch_one(db.pool())
      .await
      .unwrap();
    db.close().await;
    assert_eq!(before, after);
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn it_never_writes_a_new_master_key() {
    let fixture = Fixture::new().await;
    fixture.stop().await;
    let key = fixture.data_dir.join("master.key");
    std::fs::remove_file(&key).unwrap();
    let mut terminal = reset(&fixture.data_dir);
    assert_eq!(terminal.finish().code(), Some(1));
    assert!(!key.exists(), "{}", terminal.transcript);
  }
}

/// `passcode`, `reset-password` and `factory-reset` only change the local
/// instance, so they refuse when the CLI is connected somewhere else.
#[cfg(unix)]
mod connected_server {
  use super::super::support::{EMAIL, Fixture, command, output, terminal::Terminal};

  const ELSEWHERE: &str = "http://localhost:8841";
  const PASSCODE_REFUSED: &str = "please connect to local instance to perform passcode action";

  fn connect(
    fixture: &Fixture,
    url: &str,
  ) {
    std::fs::write(
      fixture.data_dir.join("config.toml"),
      format!("version = 1\nserver_url = \"{url}\"\n"),
    )
    .unwrap();
  }

  async fn passcode(
    fixture: &Fixture,
    env: &[(&str, &str)],
  ) -> std::process::Output {
    let mut cmd = command(&fixture.data_dir);
    cmd.args(["admin", "passcode", EMAIL]);
    for (name, value) in env {
      cmd.env(name, value);
    }
    output(cmd, None).await
  }

  fn interactive(
    fixture: &Fixture,
    arguments: &[&str],
  ) -> Terminal {
    let mut cmd = command(&fixture.data_dir);
    cmd.arg("admin").args(arguments);
    let mut terminal = Terminal::new(cmd);
    assert_eq!(terminal.finish().code(), Some(1));
    terminal
  }

  async fn audit_rows(fixture: &Fixture) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM audit_events")
      .fetch_one(fixture.state.db.pool())
      .await
      .unwrap()
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn all_three_refuse_before_touching_anything() {
    let fixture = Fixture::new().await;
    let _lock = app::server::InstanceLock::acquire(&app::config::sqlite_url(
      &fixture.data_dir.join("dopbase.db"),
    ))
    .unwrap();
    connect(&fixture, ELSEWHERE);
    let before = audit_rows(&fixture).await;

    let refused = passcode(&fixture, &[]).await;
    assert!(!refused.status.success());
    let message = String::from_utf8_lossy(&refused.stderr);
    assert!(message.contains(PASSCODE_REFUSED), "{message}");

    for (arguments, refused) in [
      (
        &["reset-password", EMAIL][..],
        "please connect to local instance to perform reset-password action",
      ),
      (
        &["factory-reset"][..],
        "please connect to local instance to perform factory-reset action",
      ),
    ] {
      let terminal = interactive(&fixture, arguments);
      assert!(
        terminal.transcript.contains(refused),
        "{}",
        terminal.transcript
      );
      assert!(!terminal.transcript.contains("New password"));
      assert!(!terminal.transcript.contains("Type please-wipe-out-system"));
    }
    assert!(fixture.data_dir.join("dopbase.db").is_file());
    assert_eq!(audit_rows(&fixture).await, before);
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn dopbase_url_counts_as_a_connection_too() {
    let fixture = Fixture::new().await;
    let refused = passcode(&fixture, &[("DOPBASE_URL", "https://dopbase.example.com")]).await;
    assert!(!refused.status.success());
    let message = String::from_utf8_lossy(&refused.stderr);
    assert!(message.contains(PASSCODE_REFUSED), "{message}");
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn a_connection_to_this_instance_or_none_at_all_is_allowed() {
    let fixture = Fixture::new().await;
    let _lock = app::server::InstanceLock::acquire(&app::config::sqlite_url(
      &fixture.data_dir.join("dopbase.db"),
    ))
    .unwrap();
    // No config.toml: nothing is saved, as inside a fresh container.
    assert!(passcode(&fixture, &[]).await.status.success());
    for url in ["http://localhost:8840", "http://127.0.0.1:8840"] {
      connect(&fixture, url);
      let issued = passcode(&fixture, &[]).await;
      assert!(
        issued.status.success(),
        "{url}: {}",
        String::from_utf8_lossy(&issued.stderr)
      );
    }
  }
}

mod connection_is_local {
  use app::cli::commands::connection_is_local;

  #[test]
  fn the_public_url_matches() {
    let public = "https://dopbase.example.com";
    assert!(connection_is_local(public, public, "0.0.0.0:8840"));
    assert!(connection_is_local(
      "https://dopbase.example.com/",
      public,
      "0.0.0.0:8840"
    ));
    assert!(!connection_is_local(
      "http://dopbase.example.com",
      public,
      "0.0.0.0:8840"
    ));
    assert!(!connection_is_local(
      "https://other.example.com",
      public,
      "0.0.0.0:8840"
    ));
    assert!(!connection_is_local(
      "https://dopbase.example.com/other",
      "https://dopbase.example.com/dopbase",
      "0.0.0.0:8840"
    ));
  }

  #[test]
  fn any_loopback_name_reaches_a_loopback_public_url() {
    for connected in [
      "http://localhost:8840",
      "http://127.0.0.1:8840",
      "http://[::1]:8840",
    ] {
      assert!(
        connection_is_local(connected, "http://localhost:8840", "127.0.0.1:8840"),
        "{connected}"
      );
    }
  }

  #[test]
  fn loopback_on_the_bind_port_matches_whatever_the_public_url() {
    assert!(connection_is_local(
      "http://localhost:8841",
      "https://dopbase.example.com",
      "0.0.0.0:8841"
    ));
    assert!(connection_is_local(
      "http://127.0.0.1:8841",
      "http://SERVER_HOST:8841",
      "0.0.0.0:8841"
    ));
  }

  #[test]
  fn another_port_or_host_does_not_match() {
    // The case that started this: connected to a container on 8841 while
    // the local instance is on 8840.
    assert!(!connection_is_local(
      "http://localhost:8841",
      "http://localhost:8840",
      "127.0.0.1:8840"
    ));
    assert!(!connection_is_local(
      "http://192.168.1.20:8840",
      "http://localhost:8840",
      "0.0.0.0:8840"
    ));
    assert!(!connection_is_local(
      "https://localhost:8840",
      "http://localhost:8840",
      "127.0.0.1:8840"
    ));
  }
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

#[cfg(unix)]
mod passcode {
  use super::super::support::{EMAIL, Fixture, command, failure, output, success};
  use serde_json::{Value, json};

  /// Runs `dopbase admin passcode` against the fixture's data directory,
  /// the way an operator on the server machine would.
  async fn run(
    fixture: &Fixture,
    arguments: &[&str],
    env: &[(&str, &str)],
  ) -> std::process::Output {
    let mut cmd = command(&fixture.data_dir);
    cmd.args(["admin", "passcode"]).args(arguments);
    for (name, value) in env {
      cmd.env(name, value);
    }
    output(cmd, None).await
  }

  /// The fixture serves in-process and does not take the instance lock, so
  /// take it here to look like a running server.
  fn running(fixture: &Fixture) -> Option<app::server::InstanceLock> {
    app::server::InstanceLock::acquire(&app::config::sqlite_url(
      &fixture.data_dir.join("dopbase.db"),
    ))
    .unwrap()
  }

  async fn launch(
    fixture: &Fixture,
    identity: &str,
    passcode: &str,
  ) -> reqwest::Response {
    reqwest::Client::new()
      .post(format!("{}/api/v1/auth/launch", fixture.url))
      .json(&json!({"identity":identity,"passcode":passcode}))
      .send()
      .await
      .unwrap()
  }

  async fn rows(fixture: &Fixture) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM passcodes")
      .fetch_one(fixture.state.db.pool())
      .await
      .unwrap()
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn a_printed_passcode_signs_the_account_in_once() {
    let fixture = Fixture::new().await;
    let _lock = running(&fixture);
    let issued: Value = run_json(&fixture, &[EMAIL.to_uppercase().as_str()]).await;
    assert_eq!(issued["email"], EMAIL);
    assert_eq!(issued["ttlSeconds"], 60);
    let passcode = issued["passcode"].as_str().unwrap();
    assert!(
      app::utils::passcode::normalize(passcode).is_some() && passcode.as_bytes()[4] == b'-',
      "{passcode}"
    );
    let url = issued["url"].as_str().unwrap();
    assert!(
      url.ends_with(&format!(
        "/launch#identity=admin%40dopbase.com&passcode={passcode}"
      )),
      "{url}"
    );
    let expires_at = chrono::DateTime::parse_from_rfc3339(issued["expiresAt"].as_str().unwrap())
      .unwrap()
      .with_timezone(&chrono::Utc);
    let left = (expires_at - chrono::Utc::now()).num_seconds();
    assert!((50..=60).contains(&left), "{left}");

    let response = launch(&fixture, EMAIL, passcode).await;
    assert_eq!(response.status(), 200);
    assert!(
      response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("dopbase_session=")
    );
    assert_eq!(launch(&fixture, EMAIL, passcode).await.status(), 401);

    let issued_events: i64 = sqlx::query_scalar(
      "SELECT COUNT(*) FROM audit_events WHERE action='passcode.issued' AND actor_label='server-cli'",
    )
    .fetch_one(fixture.state.db.pool())
    .await
    .unwrap();
    assert_eq!(issued_events, 1);
    let report = app::services::audit_chain::verify(fixture.state.db.pool())
      .await
      .unwrap();
    assert!(report.ok, "chain broke: {:?}", report.broken);
  }

  async fn run_json(
    fixture: &Fixture,
    arguments: &[&str],
  ) -> Value {
    let mut cmd = command(&fixture.data_dir);
    cmd.args(["--json", "admin", "passcode"]).args(arguments);
    success(&output(cmd, None).await)
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn human_output_shows_the_code_expiry_and_link() {
    let fixture = Fixture::new().await;
    let _lock = running(&fixture);
    let result = run(&fixture, &[EMAIL, "--ttl", "120"], &[]).await;
    assert!(result.status.success(), "{result:?}");
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(
      stdout.contains(&format!("Launch passcode for {EMAIL}")),
      "{stdout}"
    );
    assert!(stdout.contains("Passcode:"), "{stdout}");
    assert!(stdout.contains("in 120 seconds"), "{stdout}");
    assert!(
      stdout.contains("/launch#identity=admin%40dopbase.com&passcode="),
      "{stdout}"
    );
    assert!(stdout.contains("It works once."), "{stdout}");
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn running_it_again_replaces_the_code_and_leaves_expired_ones_alone() {
    let fixture = Fixture::new().await;
    let _lock = running(&fixture);
    sqlx::query("INSERT INTO admins(id,email,password_hash,role,role_id,is_root,created_at,updated_at) VALUES('usr_stale','stale@dopbase.com','','member','rol_member',0,'t','t')")
      .execute(fixture.state.db.pool())
      .await
      .unwrap();
    sqlx::query("INSERT INTO passcodes(id,purpose,admin_id,code_hash,created_at,expires_at) VALUES('psc_stale','launch','usr_stale',x'00','2020-01-01T00:00:00+00:00','2020-01-01T00:01:00+00:00')")
      .execute(fixture.state.db.pool())
      .await
      .unwrap();
    let first = run_json(&fixture, &[EMAIL]).await;
    let second = run_json(&fixture, &[EMAIL]).await;
    assert_ne!(first["passcode"], second["passcode"]);
    assert_eq!(
      rows(&fixture).await,
      2,
      "one live code plus the untouched stale one"
    );
    assert_eq!(
      launch(&fixture, EMAIL, first["passcode"].as_str().unwrap())
        .await
        .status(),
      401
    );
    assert_eq!(
      launch(&fixture, EMAIL, second["passcode"].as_str().unwrap())
        .await
        .status(),
      200
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn it_needs_a_running_server_with_the_admin_ui() {
    let fixture = Fixture::new().await;
    failure(&run(&fixture, &[EMAIL], &[]).await, "isn't running");
    let _lock = running(&fixture);
    failure(
      &run(&fixture, &[EMAIL], &[("DOPBASE_WEB_UI", "false")]).await,
      "web_ui = false",
    );
    assert_eq!(rows(&fixture).await, 0);
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn it_refuses_accounts_that_cannot_sign_in() {
    let fixture = Fixture::new().await;
    let _lock = running(&fixture);
    for (id, email, role_id, status) in [
      ("usr_agent", "agent@dopbase.com", "rol_ai_agent", "active"),
      ("usr_gone", "gone@dopbase.com", "rol_member", "disabled"),
      ("usr_member", "member@dopbase.com", "rol_member", "active"),
    ] {
      sqlx::query("INSERT INTO admins(id,email,password_hash,role,role_id,is_root,status,created_at,updated_at) VALUES(?,?,'','member',?,0,?,'t','t')")
        .bind(id)
        .bind(email)
        .bind(role_id)
        .bind(status)
        .execute(fixture.state.db.pool())
        .await
        .unwrap();
    }
    failure(
      &run(&fixture, &["nobody@dopbase.com"], &[]).await,
      "no account exists",
    );
    failure(
      &run(&fixture, &["agent@dopbase.com"], &[]).await,
      "tokens only",
    );
    failure(
      &run(&fixture, &["gone@dopbase.com"], &[]).await,
      "turned off",
    );

    sqlx::query("INSERT INTO sso_providers(id,preset,display_name,issuer,client_id,client_secret_ciphertext,client_secret_nonce,enabled,require_sso,created_at,updated_at) VALUES('sso_x','generic','Okta','https://idp.example.com','client',x'00',x'00',1,1,'t','t')")
      .execute(fixture.state.db.pool())
      .await
      .unwrap();
    let sso = [("DOPBASE_SSO", "true")];
    failure(
      &run(&fixture, &["member@dopbase.com"], &sso).await,
      "only root can sign in with a passcode",
    );
    assert!(run(&fixture, &[EMAIL], &sso).await.status.success());
    assert_eq!(rows(&fixture).await, 1, "only root got a code");
  }

  #[tokio::test]
  async fn it_only_runs_on_the_server_machine() {
    use clap::Parser;
    let remote = app::cli::args::Cli::try_parse_from([
      "dopbase",
      "--server",
      "https://dopbase.example.com",
      "admin",
      "passcode",
      EMAIL,
    ])
    .unwrap();
    assert_eq!(
      app::cli::commands::execute(remote)
        .await
        .unwrap_err()
        .to_string(),
      "--server cannot be used with local `dopbase admin` commands"
    );
  }
}
