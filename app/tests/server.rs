use app::server::InstanceLock;

#[test]
fn duplicate_instance_reports_running_server() {
  let directory = tempfile::TempDir::new().unwrap();
  let database_url = format!("sqlite://{}", directory.path().join("server.db").display());
  let first = InstanceLock::acquire(&database_url).unwrap();
  assert!(first.is_some());
  let result = InstanceLock::acquire(&database_url);
  let message = result
    .err()
    .expect("second server should be rejected")
    .to_string();
  assert!(
    message.contains("Dopbase server is already running"),
    "{message}"
  );
  drop(first);
}

#[test]
fn instance_lock_reports_held_and_released_states() {
  let directory = tempfile::TempDir::new().unwrap();
  let database_url = format!("sqlite://{}", directory.path().join("server.db").display());
  assert!(!InstanceLock::is_held(&database_url).unwrap());

  let lock = InstanceLock::acquire(&database_url).unwrap();
  assert!(InstanceLock::is_held(&database_url).unwrap());

  drop(lock);
  assert!(!InstanceLock::is_held(&database_url).unwrap());
}
