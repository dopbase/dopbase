use std::{
  fs,
  io::Write,
  path::Path,
  sync::{Arc, Mutex},
  time::Duration,
};

use app::constants::config::{DAEMON_LOG_FILENAME, DAEMON_PID_FILENAME};
use app::daemon::{
  ManagedDaemonState, Ready, inspect, log_file_path, pid_file_path, read_pid_file, remove_pid_file,
  stop, write_pid_file,
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
  let _lock = write_pid_file(&path, 4242, "127.0.0.1:8840", "http://localhost:8840").unwrap();
  let pid_file = read_pid_file(&path).unwrap();
  assert_eq!(pid_file.pid, 4242);
  assert_eq!(pid_file.bind_address, "127.0.0.1:8840");
  assert_eq!(
    pid_file.resolved_public_url().as_deref(),
    Some("http://localhost:8840")
  );
  assert_eq!(pid_file.version, env!("CARGO_PKG_VERSION"));
  remove_pid_file(&path).unwrap();
  assert!(!path.exists());
  remove_pid_file(&path).unwrap(); // idempotent
}

#[test]
fn old_pid_files_derive_a_loopback_public_url() {
  let pid_file: app::daemon::PidFile = serde_json::from_value(serde_json::json!({
    "pid": 4242,
    "started_at": "2026-01-01T00:00:00Z",
    "version": "0.0.12",
    "bind_address": "127.0.0.1:9123"
  }))
  .unwrap();

  assert_eq!(
    pid_file.resolved_public_url().as_deref(),
    Some("http://localhost:9123")
  );
}

#[test]
fn inspect_distinguishes_absent_running_and_stale_daemons() {
  let directory = tempfile::TempDir::new().unwrap();
  assert!(matches!(
    inspect(directory.path()).unwrap(),
    ManagedDaemonState::Absent
  ));

  let path = pid_file_path(directory.path());
  let lock = write_pid_file(&path, 4242, "127.0.0.1:8840", "http://localhost:8840").unwrap();
  let running = inspect(directory.path()).unwrap();
  assert!(matches!(running, ManagedDaemonState::Running(pid) if pid.pid == 4242));

  drop(lock);
  assert!(matches!(
    inspect(directory.path()).unwrap(),
    ManagedDaemonState::Stale
  ));
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
  let lock = write_pid_file(&path, stale_pid, "127.0.0.1:8840", "http://localhost:8840").unwrap();
  drop(lock);
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

#[cfg(unix)]
#[tokio::test]
async fn stop_never_signals_a_live_process_from_an_unowned_stale_pid_file() {
  use std::process::Command;
  let directory = tempfile::TempDir::new().unwrap();
  let mut unrelated = Command::new("sleep").arg("30").spawn().unwrap();
  let path = pid_file_path(directory.path());
  let lock = write_pid_file(
    &path,
    unrelated.id(),
    "127.0.0.1:8840",
    "http://localhost:8840",
  )
  .unwrap();
  drop(lock); // Simulate the original daemon exiting without removing its file.

  let result = stop(Some(directory.path()), Duration::from_secs(1), false).await;
  let message = result.unwrap_err().to_string();
  assert!(message.contains("refusing to signal"), "{message}");
  assert!(unrelated.try_wait().unwrap().is_none());
  unrelated.kill().unwrap();
  let _ = unrelated.wait();
}
