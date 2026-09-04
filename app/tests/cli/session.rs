use app::cli::{
  local_config::{ClientConfig, ResolvedServer, ServerSource},
  session,
};
use std::fs;
use tempfile::TempDir;

const TOKEN: &str = "dbs_test_marker_that_must_stay_encrypted";
const EMAIL: &str = "admin@example.com";

fn server(
  directory: &TempDir,
  url: &str,
) -> ResolvedServer {
  ResolvedServer {
    url: url.into(),
    source: ServerSource::Argument,
    config_path: directory.path().join("config.toml"),
    config: ClientConfig {
      version: 1,
      server_url: None,
      default_environment: None,
    },
  }
}

#[test]
fn encrypted_session_round_trips_without_plaintext() {
  let directory = TempDir::new().unwrap();
  let server = server(&directory, "https://dopbase.example.com");

  session::save(&server, TOKEN, Some(EMAIL)).unwrap();

  let stored = fs::read(directory.path().join("session")).unwrap();
  assert!(
    !stored
      .windows(TOKEN.len())
      .any(|value| value == TOKEN.as_bytes())
  );
  assert!(
    !stored
      .windows(server.url.len())
      .any(|value| value == server.url.as_bytes())
  );
  assert!(
    !stored
      .windows(EMAIL.len())
      .any(|value| value == EMAIL.as_bytes())
  );
  let loaded = session::load(&server).unwrap().unwrap();
  assert_eq!(loaded.token, TOKEN);
  assert_eq!(loaded.email.as_deref(), Some(EMAIL));
  assert_eq!(
    fs::read(directory.path().join("session-key"))
      .unwrap()
      .len(),
    32
  );
}

#[test]
fn each_save_uses_a_fresh_nonce_and_replaces_the_token() {
  let directory = TempDir::new().unwrap();
  let server = server(&directory, "http://localhost:8840");

  session::save(&server, "dbs_first", Some(EMAIL)).unwrap();
  let first = fs::read(directory.path().join("session")).unwrap();
  session::save(&server, "dbs_second", Some(EMAIL)).unwrap();
  let second = fs::read(directory.path().join("session")).unwrap();

  assert_ne!(first, second);
  assert_eq!(session::load(&server).unwrap().unwrap().token, "dbs_second");
}

#[test]
fn session_is_scoped_to_its_normalized_server() {
  let directory = TempDir::new().unwrap();
  let saved = server(&directory, "https://one.example.com");
  let other = server(&directory, "https://two.example.com");
  session::save(&saved, TOKEN, Some(EMAIL)).unwrap();

  assert!(session::load(&other).unwrap().is_none());
  assert_eq!(session::load(&saved).unwrap().unwrap().token, TOKEN);
}

#[test]
fn session_without_cached_email_remains_compatible() {
  let directory = TempDir::new().unwrap();
  let server = server(&directory, "https://dopbase.example.com");
  session::save(&server, TOKEN, None).unwrap();

  let loaded = session::load(&server).unwrap().unwrap();
  assert_eq!(loaded.token, TOKEN);
  assert!(loaded.email.is_none());
}

#[test]
fn tampered_session_fails_closed_without_exposing_contents() {
  let directory = TempDir::new().unwrap();
  let server = server(&directory, "https://dopbase.example.com");
  session::save(&server, TOKEN, Some(EMAIL)).unwrap();
  let path = directory.path().join("session");
  let mut stored = fs::read(&path).unwrap();
  let last = stored.len() - 1;
  stored[last] ^= 1;
  fs::write(&path, stored).unwrap();

  let message = session::load(&server).err().unwrap().to_string();
  assert!(message.contains("could not decrypt"), "{message}");
  assert!(!message.contains(TOKEN));
}

#[test]
fn missing_key_fails_closed_without_exposing_contents() {
  let directory = TempDir::new().unwrap();
  let server = server(&directory, "https://dopbase.example.com");
  session::save(&server, TOKEN, Some(EMAIL)).unwrap();
  fs::remove_file(directory.path().join("session-key")).unwrap();

  let message = session::load(&server).err().unwrap().to_string();
  assert!(message.contains("session key is unavailable"), "{message}");
  assert!(!message.contains(TOKEN));
}

#[test]
fn unsupported_session_version_fails_closed() {
  let directory = TempDir::new().unwrap();
  let server = server(&directory, "https://dopbase.example.com");
  session::save(&server, TOKEN, Some(EMAIL)).unwrap();
  let path = directory.path().join("session");
  let mut stored = fs::read(&path).unwrap();
  stored[8] = 2;
  fs::write(&path, stored).unwrap();

  let message = session::load(&server).err().unwrap().to_string();
  assert!(message.contains("unsupported version"), "{message}");
  assert!(!message.contains(TOKEN));
}

#[test]
fn remove_is_idempotent_and_deletes_session_material() {
  let directory = TempDir::new().unwrap();
  let server = server(&directory, "https://dopbase.example.com");
  session::save(&server, TOKEN, Some(EMAIL)).unwrap();

  assert!(session::remove(&server).unwrap());
  assert!(!session::remove(&server).unwrap());

  assert!(!directory.path().join("session").exists());
  assert!(!directory.path().join("session-key").exists());
}

#[cfg(unix)]
#[test]
fn session_files_are_private_on_unix() {
  use std::os::unix::fs::PermissionsExt;

  let directory = TempDir::new().unwrap();
  let server = server(&directory, "https://dopbase.example.com");
  session::save(&server, TOKEN, Some(EMAIL)).unwrap();

  assert_eq!(
    fs::metadata(directory.path()).unwrap().permissions().mode() & 0o777,
    0o700
  );

  assert_eq!(
    fs::metadata(directory.path().join("session"))
      .unwrap()
      .permissions()
      .mode()
      & 0o777,
    0o600
  );
  assert_eq!(
    fs::metadata(directory.path().join("session-key"))
      .unwrap()
      .permissions()
      .mode()
      & 0o777,
    0o600
  );
}
