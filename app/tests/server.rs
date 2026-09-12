use app::server::{InstanceLock, startup_banner};

#[test]
fn startup_banner_contains_brand_and_version() {
  let banner = startup_banner(
    "http://localhost:8840",
    "127.0.0.1:8840",
    std::path::Path::new("/Users/venobi/.dopbase"),
    false,
  );
  assert!(banner.contains("Dopbase"));
  assert!(banner.contains("Secure, Simple and Private"));
  assert!(banner.contains(concat!("Version ", env!("CARGO_PKG_VERSION"))));
  assert!(banner.contains("Public URL: http://localhost:8840"));
  assert!(banner.contains("Bind:       127.0.0.1:8840"));
  assert!(banner.contains("Admin UI:   http://localhost:8840"));
  assert!(banner.contains("API:        http://localhost:8840/api/v1"));
  assert!(banner.contains("Config:     /Users/venobi/.dopbase"));
}

#[test]
fn startup_banner_separates_the_placeholder_from_the_bind_address() {
  let banner = startup_banner(
    "http://SERVER_HOST:9000",
    "0.0.0.0:9000",
    std::path::Path::new("/srv/dopbase"),
    true,
  );
  assert!(banner.contains("Public URL: http://SERVER_HOST:9000"));
  assert!(banner.contains("Bind:       0.0.0.0:9000"));
  assert!(banner.contains("API Specs:  http://SERVER_HOST:9000/api/docs"));
}

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
