use app::{constants::config::executable_environment_names, daemon};
use serde_json::Value;
use std::{
  fs,
  path::Path,
  process::{Command, Output},
};
use tempfile::TempDir;

fn command(data_dir: &Path) -> Command {
  let mut command = Command::new(env!("CARGO_BIN_EXE_dopbase"));
  for name in executable_environment_names() {
    command.env_remove(name);
  }
  command.env_remove("DOPBASE_INTERNAL_DAEMON_LAUNCH");
  command.env("NO_COLOR", "1");
  command.arg("--data-dir").arg(data_dir);
  command
}

fn success(output: Output) -> Value {
  assert!(
    output.status.success(),
    "command failed: {}",
    String::from_utf8_lossy(&output.stderr)
  );
  serde_json::from_slice(&output.stdout).unwrap()
}

fn failure(
  output: Output,
  message: &str,
) {
  assert_eq!(output.status.code(), Some(1));
  assert!(output.stdout.is_empty());
  assert!(
    String::from_utf8_lossy(&output.stderr).contains(message),
    "{}",
    String::from_utf8_lossy(&output.stderr)
  );
}

fn migration_notice(
  output: Output,
  replacement: &str,
) {
  assert_eq!(output.status.code(), Some(1));
  assert!(output.stdout.is_empty());
  let notice = String::from_utf8(output.stderr).unwrap();
  assert!(notice.starts_with("Info:"), "{notice}");
  assert!(notice.contains("has been replaced"), "{notice}");
  assert!(notice.contains(replacement), "{notice}");
  assert!(!notice.contains("Error:"));
}

#[test]
fn replaced_commands_report_migration_without_creating_state() {
  let directory = TempDir::new().unwrap();
  let data_dir = directory.path().join("missing");
  for (args, replacement) in [
    (
      vec!["up", "--port", "9000", "--docs"],
      "dopbase server start --background",
    ),
    (vec!["down", "--timeout", "30"], "dopbase server stop"),
  ] {
    migration_notice(
      command(&data_dir)
        .arg("server")
        .args(&args)
        .env("DOPBASE_PORT", "invalid")
        .output()
        .unwrap(),
      replacement,
    );
    let output = command(&data_dir)
      .args(["--json", "server"])
      .args(&args)
      .output()
      .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["success"], false);
    assert_eq!(value["info"]["code"], "COMMAND_REPLACED");
    assert_eq!(value["info"]["replacement"], replacement);
    assert!(value.get("error").is_none());
    assert!(
      value["info"]["message"]
        .as_str()
        .unwrap()
        .contains(replacement)
    );
    let colored = command(&data_dir)
      .arg("server")
      .args(&args)
      .env_remove("NO_COLOR")
      .env("CLICOLOR_FORCE", "1")
      .output()
      .unwrap();
    assert_eq!(colored.status.code(), Some(1));
    assert!(colored.stdout.is_empty());
    let notice = String::from_utf8(colored.stderr).unwrap();
    assert!(notice.contains("\x1b[34mInfo:"), "{notice:?}");
    assert!(!notice.contains("\x1b[31m"));
    assert!(!data_dir.exists());
  }
}

#[test]
fn help_lists_new_commands_and_explains_hidden_replaced_commands() {
  let directory = TempDir::new().unwrap();
  let output = command(directory.path())
    .args(["server", "--help"])
    .output()
    .unwrap();
  assert!(output.status.success());
  let help = String::from_utf8(output.stdout).unwrap();
  for command in ["start", "stop", "restart", "status", "logs"] {
    assert!(help.contains(command));
  }
  assert!(!help.contains("server up"));
  assert!(!help.contains("server down"));
  assert!(
    !help
      .lines()
      .any(|line| line.starts_with("  up ") || line.starts_with("  down "))
  );
  for (old, new) in [("up", "start --background"), ("down", "stop")] {
    let output = command(directory.path())
      .args(["server", old, "--help"])
      .output()
      .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("has been replaced"));
    assert!(help.contains(&format!("dopbase server {new}")));
  }
}

#[test]
fn restart_requires_a_running_managed_server_and_rejects_launch_options() {
  let directory = TempDir::new().unwrap();
  failure(
    command(directory.path())
      .args(["server", "restart"])
      .output()
      .unwrap(),
    "background server is stopped",
  );
  for option in ["--background", "-b", "--port", "--docs"] {
    assert!(
      !command(directory.path())
        .args(["server", "restart", option])
        .output()
        .unwrap()
        .status
        .success()
    );
  }
  let path = daemon::pid_file_path(directory.path());
  let lock =
    daemon::write_pid_file(&path, 4242, "127.0.0.1:8840", "http://localhost:8840").unwrap();
  drop(lock);
  let before = fs::read(&path).unwrap();
  failure(
    command(directory.path())
      .args(["server", "restart"])
      .output()
      .unwrap(),
    "background server is stopped",
  );
  assert_eq!(fs::read(&path).unwrap(), before);
  fs::write(&path, "invalid json").unwrap();
  failure(
    command(directory.path())
      .args(["server", "restart"])
      .output()
      .unwrap(),
    "not a valid Dopbase PID file",
  );
  assert_eq!(fs::read_to_string(&path).unwrap(), "invalid json");
}

#[test]
fn stop_and_restart_direct_foreground_users_to_ctrl_c() {
  let directory = TempDir::new().unwrap();
  let url = app::config::sqlite_url(&directory.path().join("dopbase.db"));
  let _lock = app::server::InstanceLock::acquire(&url).unwrap();
  for action in ["stop", "restart"] {
    failure(
      command(directory.path())
        .args(["server", action])
        .output()
        .unwrap(),
      "Stop it with Ctrl+C",
    );
    assert!(app::server::InstanceLock::is_held(&url).unwrap());
  }
}

#[test]
fn restart_refuses_older_running_metadata_before_stopping() {
  let directory = TempDir::new().unwrap();
  let path = daemon::pid_file_path(directory.path());
  let _lock = daemon::write_pid_file(
    &path,
    std::process::id(),
    "127.0.0.1:8840",
    "http://localhost:8840",
  )
  .unwrap();
  let before = fs::read(&path).unwrap();
  failure(
    command(directory.path())
      .args(["server", "restart"])
      .output()
      .unwrap(),
    "no saved launch settings",
  );
  assert_eq!(fs::read(&path).unwrap(), before);
  assert!(matches!(
    daemon::inspect(directory.path()).unwrap(),
    daemon::ManagedDaemonState::Running(_)
  ));
}

#[cfg(unix)]
mod background {
  use super::*;
  use std::{net::TcpListener, os::unix::fs::PermissionsExt};

  struct Server {
    directory: TempDir,
  }
  impl Server {
    fn new() -> Self {
      Self {
        directory: TempDir::new().unwrap(),
      }
    }
    fn data(&self) -> std::path::PathBuf {
      self.directory.path().join("data")
    }
    fn command(&self) -> Command {
      command(&self.data())
    }
    fn pid(&self) -> daemon::PidFile {
      daemon::read_pid_file(&daemon::pid_file_path(&self.data())).unwrap()
    }
    fn config(
      &self,
      contents: &str,
    ) {
      fs::write(self.directory.path().join("custom.toml"), contents).unwrap();
    }
    fn start(
      &self,
      extra: &[&str],
    ) -> Value {
      success(
        self
          .command()
          .current_dir(self.directory.path())
          .args(["--json", "server", "start", "-b", "--config", "custom.toml"])
          .args(extra)
          .output()
          .unwrap(),
      )
    }
  }
  impl Drop for Server {
    fn drop(&mut self) {
      if daemon::pid_file_path(&self.data()).exists() {
        let output = self
          .command()
          .args(["server", "stop", "--timeout", "1"])
          .output()
          .unwrap();
        if !output.status.success()
          && let Ok(pid) = daemon::read_pid_file(&daemon::pid_file_path(&self.data()))
          && matches!(
            daemon::inspect(&self.data()),
            Ok(daemon::ManagedDaemonState::Running(_))
          )
        {
          let _ = nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(pid.pid as i32),
            nix::sys::signal::Signal::SIGKILL,
          );
        }
      }
    }
  }
  fn port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
      .unwrap()
      .local_addr()
      .unwrap()
      .port()
  }

  #[tokio::test]
  async fn restart_preserves_overrides_and_relative_paths_while_rereading_configuration() {
    let server = Server::new();
    let cli_port = port();
    server.config("docs = false\nport = 1\n[master_key]\npath = 'custom.key'\n");
    let output = success(
      server
        .command()
        .current_dir(server.directory.path())
        .env("DOPBASE_PORT", "0")
        .env("DOPBASE_HOST", "127.0.0.1")
        .env("DOPBASE_SHUTDOWN_GRACE_SECONDS", "2")
        .env("DOPBASE_TOKEN", "private-token-marker")
        .env("UNRELATED_SECRET", "unrelated-marker")
        .args([
          "--json",
          "server",
          "start",
          "--background",
          "--config",
          "custom.toml",
          "--port",
          &cli_port.to_string(),
        ])
        .output()
        .unwrap(),
    );
    assert_eq!(output["stop_command"], "dopbase server stop");
    let before = server.pid();
    assert_eq!(before.bind_address, format!("127.0.0.1:{cli_port}"));
    let path = daemon::pid_file_path(&server.data());
    assert_eq!(
      fs::metadata(&path).unwrap().permissions().mode() & 0o777,
      0o600
    );
    let metadata = fs::read_to_string(&path).unwrap();
    for secret in [
      "private-token-marker",
      "unrelated-marker",
      "DOPBASE_TOKEN",
      "UNRELATED_SECRET",
    ] {
      assert!(!metadata.contains(secret));
    }
    let saved = before.launch.as_ref().unwrap();
    assert_eq!(saved.environment.port.as_deref(), Some("0"));
    assert_eq!(saved.overrides.port, Some(cli_port));
    assert_eq!(
      saved.working_directory,
      server.directory.path().canonicalize().unwrap()
    );
    let key = fs::read(server.directory.path().join("custom.key")).unwrap();
    assert!(!metadata.contains(std::str::from_utf8(&key).unwrap_or("not-a-key")));
    let client = reqwest::Client::new();
    let endpoint = format!("http://127.0.0.1:{cli_port}");
    assert!(
      client
        .get(format!("{endpoint}/api/v1/health"))
        .send()
        .await
        .unwrap()
        .status()
        .is_success()
    );
    assert!(
      !client
        .get(format!("{endpoint}/api/v1/openapi.json"))
        .send()
        .await
        .unwrap()
        .status()
        .is_success()
    );

    // Replaced commands must leave a live server and its saved metadata untouched.
    for args in [vec!["up", "--port", "0"], vec!["down", "--timeout", "0"]] {
      migration_notice(
        server.command().arg("server").args(args).output().unwrap(),
        "has been replaced",
      );
      assert_eq!(server.pid().pid, before.pid);
      assert_eq!(fs::read_to_string(&path).unwrap(), metadata);
    }
    failure(
      server
        .command()
        .args(["server", "start", "-b"])
        .output()
        .unwrap(),
      "already running",
    );

    server.config("docs = true\nport = 1\n[master_key]\npath = 'custom.key'\n");
    let other = TempDir::new().unwrap();
    let restart = success(
      server
        .command()
        .current_dir(other.path())
        .env("DOPBASE_PORT", "invalid")
        .env("DOPBASE_HOST", "invalid")
        .args(["--json", "server", "restart"])
        .output()
        .unwrap(),
    );
    assert_eq!(restart["restarted"], true);
    assert_eq!(restart["previous_pid"], before.pid);
    assert_ne!(restart["pid"], before.pid);
    assert_eq!(server.pid().bind_address, before.bind_address);
    // Restart closes existing HTTP connections; use a fresh connection to the replacement.
    let client = reqwest::Client::new();
    assert!(
      client
        .get(format!("{endpoint}/api/v1/openapi.json"))
        .send()
        .await
        .unwrap()
        .status()
        .is_success()
    );
    assert_eq!(
      fs::read(server.directory.path().join("custom.key")).unwrap(),
      key
    );
    let stopped = success(
      server
        .command()
        .args(["--json", "server", "stop"])
        .output()
        .unwrap(),
    );
    assert_eq!(stopped["stopped"], true);
    assert!(!path.exists());
  }

  #[tokio::test]
  async fn restart_validation_and_replacement_failures_have_distinct_outcomes() {
    let server = Server::new();
    let original_port = port();
    server.config(&format!("port = {original_port}\n"));
    server.start(&[]);
    let original = server.pid().pid;
    let client = reqwest::Client::new();
    let endpoint = format!("http://127.0.0.1:{original_port}/api/v1/health");
    let pid_path = daemon::pid_file_path(&server.data());
    let original_metadata = fs::read(&pid_path).unwrap();
    for (field, value, message) in [
      (
        "version",
        Value::from(2),
        "unsupported saved server launch version",
      ),
      (
        "working_directory",
        Value::from(server.directory.path().join("missing").to_str().unwrap()),
        "original server working directory is unavailable",
      ),
    ] {
      let mut metadata: Value = serde_json::from_slice(&original_metadata).unwrap();
      metadata["launch"][field] = value;
      fs::write(&pid_path, serde_json::to_vec(&metadata).unwrap()).unwrap();
      failure(
        server
          .command()
          .args(["server", "restart"])
          .output()
          .unwrap(),
        message,
      );
      assert_eq!(server.pid().pid, original);
      fs::write(&pid_path, &original_metadata).unwrap();
    }
    server.config("port = 'invalid'\n");
    failure(
      server
        .command()
        .args(["server", "restart"])
        .output()
        .unwrap(),
      "failed to parse",
    );
    assert_eq!(server.pid().pid, original);
    assert!(
      client
        .get(&endpoint)
        .send()
        .await
        .unwrap()
        .status()
        .is_success()
    );
    server.config(&format!("port = {original_port}\n"));
    let occupied = TcpListener::bind("127.0.0.1:0").unwrap();
    server.config(&format!(
      "port = {}\n",
      occupied.local_addr().unwrap().port()
    ));
    failure(
      server
        .command()
        .args(["server", "restart"])
        .output()
        .unwrap(),
      "server stopped, but restart failed",
    );
    assert!(matches!(
      daemon::inspect(&server.data()).unwrap(),
      daemon::ManagedDaemonState::Absent
    ));
    assert!(client.get(&endpoint).send().await.is_err());
  }

  #[test]
  fn restart_preserves_environment_overrides_without_cli_equivalents() {
    let server = Server::new();
    let selected = port();
    server.config("port = 1\ndocs = false\n");
    success(
      server
        .command()
        .current_dir(server.directory.path())
        .env("DOPBASE_PORT", selected.to_string())
        .env("DOPBASE_DOCS", "true")
        .args(["--json", "server", "start", "-b", "--config", "custom.toml"])
        .output()
        .unwrap(),
    );
    assert_eq!(server.pid().bind_address, format!("127.0.0.1:{selected}"));
    let before = server.pid().pid;
    let output = success(
      server
        .command()
        .env("DOPBASE_PORT", "1")
        .env("DOPBASE_DOCS", "false")
        .args(["--json", "server", "restart"])
        .output()
        .unwrap(),
    );
    assert_eq!(output["previous_pid"], before);
    assert_eq!(server.pid().bind_address, format!("127.0.0.1:{selected}"));
    assert_eq!(
      server.pid().launch.unwrap().environment.docs.as_deref(),
      Some("true")
    );
    let pid_path = daemon::pid_file_path(&server.data());
    let mut old_metadata: Value = serde_json::from_slice(&fs::read(&pid_path).unwrap()).unwrap();
    old_metadata.as_object_mut().unwrap().remove("launch");
    fs::write(&pid_path, serde_json::to_vec(&old_metadata).unwrap()).unwrap();
    failure(
      server
        .command()
        .args(["server", "restart"])
        .output()
        .unwrap(),
      "no saved launch settings",
    );
    let stopped = success(
      server
        .command()
        .args(["--json", "server", "stop"])
        .output()
        .unwrap(),
    );
    assert_eq!(stopped["stopped"], true);
    assert!(!pid_path.exists());
  }
}
