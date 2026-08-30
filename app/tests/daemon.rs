use std::{
  fs,
  io::Write,
  path::Path,
  sync::{Arc, Mutex},
  time::Duration,
};

use app::constants::config::{DAEMON_LOG_FILENAME, DAEMON_PID_FILENAME};
use app::daemon::{
  Ready, log_file_path, pid_file_path, read_pid_file, remove_pid_file, stop, write_pid_file,
};

#[derive(Clone, Default)]
struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

impl Write for SharedBuffer {
  fn write(
    &mut self,
    buffer: &[u8],
  ) -> std::io::Result<usize> {
    self.0.lock().unwrap().write(buffer)
  }
  fn flush(&mut self) -> std::io::Result<()> {
    Ok(())
  }
}

#[test]
fn paths_derive_from_the_data_directory() {
  let data_dir = Path::new("/tmp/dopbase-data");
  assert_eq!(pid_file_path(data_dir), data_dir.join(DAEMON_PID_FILENAME));
  assert_eq!(log_file_path(data_dir), data_dir.join(DAEMON_LOG_FILENAME));
}

#[test]
fn pid_file_round_trips() {
  let directory = tempfile::TempDir::new().unwrap();
  let path = directory.path().join("dopbase.pid");
  write_pid_file(&path, 4242, "127.0.0.1:8840").unwrap();
  let pid_file = read_pid_file(&path).unwrap();
  assert_eq!(pid_file.pid, 4242);
  assert_eq!(pid_file.bind_address, "127.0.0.1:8840");
  assert_eq!(pid_file.version, env!("CARGO_PKG_VERSION"));
  remove_pid_file(&path).unwrap();
  assert!(!path.exists());
  remove_pid_file(&path).unwrap(); // idempotent
}

#[test]
fn corrupt_pid_file_is_rejected() {
  let directory = tempfile::TempDir::new().unwrap();
  let path = directory.path().join("dopbase.pid");
  fs::write(&path, "not json").unwrap();
  assert!(read_pid_file(&path).is_err());
}

#[test]
fn ready_reports_exactly_once() {
  let buffer = SharedBuffer::default();
  let ready = Ready::for_writer(Box::new(buffer.clone()));
  ready.ok(7, None);
  ready.fail("ignored after ok");
  let contents = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
  assert_eq!(contents, "ok 7\n");
}

#[test]
fn ready_relays_setup_token_and_flattens_errors() {
  let buffer = SharedBuffer::default();
  Ready::for_writer(Box::new(buffer.clone())).ok(9, Some("setup_abc"));
  Ready::for_writer(Box::new(buffer.clone())).fail("bind failed\ncaused by: boom");
  let contents = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
  assert_eq!(
    contents,
    "ok 9 setup-token setup_abc\nerror bind failed caused by: boom\n"
  );
}

#[cfg(unix)]
#[tokio::test]
async fn stop_reports_stale_pid_files() {
  use std::process::Command;
  let directory = tempfile::TempDir::new().unwrap();
  // Record the PID of a process that has already exited.
  let stale_pid: u32 = {
    let mut child = Command::new("sh").arg("-c").arg("exit 0").spawn().unwrap();
    let pid = child.id();
    let _ = child.wait();
    pid
  };
  let path = pid_file_path(directory.path());
  write_pid_file(&path, stale_pid, "127.0.0.1:8840").unwrap();
  let result = stop(Some(directory.path()), Duration::from_secs(1), false).await;
  let message = result.unwrap_err().to_string();
  assert!(message.contains("no running Dopbase daemon"), "{message}");
  assert!(!path.exists());
}

#[cfg(unix)]
#[tokio::test]
async fn stop_without_pid_file_fails() {
  let directory = tempfile::TempDir::new().unwrap();
  let result = stop(Some(directory.path()), Duration::from_secs(1), false).await;
  let message = result.unwrap_err().to_string();
  assert!(message.contains("no running Dopbase daemon"), "{message}");
}
