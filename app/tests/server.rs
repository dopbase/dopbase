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

#[tokio::test]
async fn failed_bind_does_not_create_configuration_references() {
  let directory = tempfile::TempDir::new().unwrap();
  let occupied = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
  let config = app::config::ServerConfig::load_with_environment(
    &app::config::ServerOverrides {
      data_dir: Some(directory.path().to_path_buf()),
      port: Some(occupied.local_addr().unwrap().port()),
      ..Default::default()
    },
    app::config::EnvironmentOverrides::default(),
  )
  .unwrap();
  let error = app::server::serve(config).await.unwrap_err();
  assert!(format!("{error:#}").contains("failed to bind"));
  assert!(!directory.path().join("server.toml").exists());
  assert!(!directory.path().join("config.toml").exists());
}

#[tokio::test]
async fn reference_write_failure_stops_startup_with_the_affected_path() {
  let directory = tempfile::TempDir::new().unwrap();
  let blocked = directory.path().join("blocked");
  std::fs::write(&blocked, b"keep").unwrap();
  let config_path = blocked.join("server.toml");
  let config = app::config::ServerConfig::load_with_environment(
    &app::config::ServerOverrides {
      data_dir: Some(directory.path().to_path_buf()),
      config_path: Some(config_path.clone()),
      port: Some(0),
      ..Default::default()
    },
    app::config::EnvironmentOverrides::default(),
  )
  .unwrap();
  let error = app::server::serve(config).await.unwrap_err();
  assert!(format!("{error:#}").contains(&config_path.display().to_string()));
  assert!(!directory.path().join("config.toml").exists());
  assert_eq!(std::fs::read(blocked).unwrap(), b"keep");
}

#[cfg(unix)]
#[test]
fn start_and_up_create_references_in_the_selected_locations() {
  use std::{
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
  };

  fn command(data_dir: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_dopbase"));
    for name in app::constants::config::executable_environment_names() {
      command.env_remove(name);
    }
    command
      .arg("--data-dir")
      .arg(data_dir)
      .stdout(Stdio::null())
      .stderr(Stdio::null());
    command
  }

  struct RunningServer {
    child: Option<Child>,
    background: bool,
    data_dir: PathBuf,
  }
  impl Drop for RunningServer {
    fn drop(&mut self) {
      if let Some(child) = &mut self.child {
        let _ = child.kill();
        let _ = child.wait();
      }
      if self.background {
        let _ = command(&self.data_dir).args(["server", "down"]).status();
      }
    }
  }

  for mode in ["start", "up"] {
    let directory = tempfile::TempDir::new().unwrap();
    let data_dir = directory.path().join("data");
    let config_path = if mode == "up" {
      directory.path().join("custom/server.toml")
    } else {
      data_dir.join("server.toml")
    };
    let mut running = RunningServer {
      child: None,
      background: mode == "up",
      data_dir: data_dir.clone(),
    };
    let mut launch = command(&data_dir);
    launch.args(["server", mode, "--port", "0"]);
    if mode == "up" {
      launch.arg("--config").arg(&config_path);
      assert!(
        launch.status().unwrap().success(),
        "background startup failed"
      );
    } else {
      running.child = Some(launch.spawn().unwrap());
      let deadline = Instant::now() + Duration::from_secs(10);
      while !data_dir.join("config.toml").exists() {
        assert!(
          running
            .child
            .as_mut()
            .unwrap()
            .try_wait()
            .unwrap()
            .is_none(),
          "foreground server exited before creating references"
        );
        assert!(
          Instant::now() < deadline,
          "startup did not create references"
        );
        std::thread::sleep(Duration::from_millis(20));
      }
    }
    assert!(
      std::fs::read_to_string(&config_path)
        .unwrap()
        .starts_with("# Dopbase server configuration\n")
    );
    assert!(
      std::fs::read_to_string(data_dir.join("config.toml"))
        .unwrap()
        .starts_with("# Dopbase client configuration\n")
    );
    if mode == "up" {
      assert!(!data_dir.join("server.toml").exists());
    }
    drop(running);
  }
}
