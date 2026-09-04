use app::server::{InstanceLock, startup_banner};

#[test]
fn startup_banner_contains_brand_and_version() {
  let banner = startup_banner(
    "http://localhost:8840",
    std::path::Path::new("/Users/venobi/.dopbase"),
    false,
  );
  assert!(banner.contains("Dopbase"));
  assert!(banner.contains("Secure, Simple and Private"));
  assert!(banner.contains(concat!("Version ", env!("CARGO_PKG_VERSION"))));
  assert!(banner.contains("Admin UI:   http://localhost:8840"));
  assert!(banner.contains("API:        http://localhost:8840/api/v1"));
  assert!(banner.contains("Config:     /Users/venobi/.dopbase"));
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
