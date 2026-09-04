use std::{
  fs::{self, File, OpenOptions},
  io::{BufRead, BufReader, BufWriter, Write},
  path::{Path, PathBuf},
  sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
  },
  time::Duration,
};

use anyhow::{Context, Result, bail};
use fs2::FileExt;
use serde::{Deserialize, Serialize};

use crate::{
  config::{ServerConfig, ensure_data_dir, resolve_data_dir},
  constants::config::{DAEMON_LOG_FILENAME, DAEMON_PID_FILENAME},
};

/// File descriptor the supervised server reports readiness on.
const READY_FD: i32 = 3;
const READY_TIMEOUT: Duration = Duration::from_secs(30);
const STOP_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// One-shot readiness reporter used by the supervised server process.
///
/// When the server is started by [`start`], the parent process inherits a pipe
/// write end into the child as `READY_FD`; the child reports `ok …` or
/// `error …` exactly once so the foreground command can relay the real startup
/// error (bind failure, master-key problem, …) to the user.
pub struct Ready {
  writer: Mutex<Box<dyn Write + Send>>,
  reported: AtomicBool,
}

impl Ready {
  /// Attach to the readiness pipe inherited from the parent command.
  /// Returns `None` when this platform does not supervise servers.
  pub fn attached() -> Option<Self> {
    #[cfg(unix)]
    {
      use std::os::unix::io::FromRawFd;
      Some(Self::for_writer(Box::new(BufWriter::new(unsafe {
        File::from_raw_fd(READY_FD)
      }))))
    }
    #[cfg(not(unix))]
    {
      None
    }
  }

  /// Build a reporter over an arbitrary writer (used by tests).
  pub fn for_writer(writer: Box<dyn Write + Send>) -> Self {
    Self {
      writer: Mutex::new(writer),
      reported: AtomicBool::new(false),
    }
  }

  /// Report a successful startup, optionally relaying the one-time setup token.
  pub fn ok(
    &self,
    pid: u32,
    setup_token: Option<&str>,
  ) {
    let mut line = format!("ok {pid}");
    if let Some(token) = setup_token {
      line.push_str(&format!(" setup-token {token}"));
    }
    self.report(line);
  }

  /// Report a startup failure; `message` must be a single line, so any
  /// newlines in the error chain are flattened.
  pub fn fail(
    &self,
    message: &str,
  ) {
    self.report(format!("error {}", message.replace(['\r', '\n'], " ")));
  }

  fn report(
    &self,
    line: String,
  ) {
    if self.reported.swap(true, Ordering::SeqCst) {
      return;
    }
    let mut writer = self
      .writer
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner());
    let _ = writer
      .write_all(line.as_bytes())
      .and_then(|_| writer.write_all(b"\n"))
      .and_then(|_| writer.flush());
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PidFile {
  pub pid: u32,
  pub started_at: String,
  pub version: String,
  pub bind_address: String,
  #[serde(default)]
  pub public_url: Option<String>,
}

impl PidFile {
  pub fn resolved_public_url(&self) -> Option<String> {
    self.public_url.clone().or_else(|| {
      self
        .bind_address
        .parse::<std::net::SocketAddr>()
        .ok()
        .filter(|address| address.ip().is_loopback())
        .map(|address| format!("http://localhost:{}", address.port()))
    })
  }
}

#[derive(Clone, Debug)]
pub enum ManagedDaemonState {
  Absent,
  Running(PidFile),
  Stale,
}

#[derive(Clone, Debug)]
pub struct Stopped {
  pub pid: u32,
  pub forced: bool,
}

pub fn pid_file_path(data_dir: &Path) -> PathBuf {
  data_dir.join(DAEMON_PID_FILENAME)
}

pub fn log_file_path(data_dir: &Path) -> PathBuf {
  data_dir.join(DAEMON_LOG_FILENAME)
}

pub fn read_pid_file(path: &Path) -> Result<PidFile> {
  let contents =
    fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
  serde_json::from_str(&contents)
    .with_context(|| format!("{} is not a valid Dopbase PID file", path.display()))
}

pub fn write_pid_file(
  path: &Path,
  pid: u32,
  bind_address: &str,
  public_url: &str,
) -> Result<File> {
  let payload = PidFile {
    pid,
    started_at: chrono::Utc::now().to_rfc3339(),
    version: env!("CARGO_PKG_VERSION").to_string(),
    bind_address: bind_address.to_string(),
    public_url: Some(public_url.to_string()),
  };
  let json = serde_json::to_string_pretty(&payload)?;
  crate::utils::private_file::write(path, json.as_bytes(), true)?;
  let file = OpenOptions::new().read(true).write(true).open(path)?;
  file
    .try_lock_exclusive()
    .context("failed to claim the daemon PID file")?;
  Ok(file)
}

pub fn remove_pid_file(path: &Path) -> Result<()> {
  match fs::remove_file(path) {
    Ok(()) => Ok(()),
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
    Err(error) => Err(error).with_context(|| format!("failed to remove {}", path.display())),
  }
}

pub fn inspect(data_dir: &Path) -> Result<ManagedDaemonState> {
  let path = pid_file_path(data_dir);
  if !path.exists() {
    return Ok(ManagedDaemonState::Absent);
  }
  let pid_file = read_pid_file(&path)?;
  let ownership = OpenOptions::new().read(true).write(true).open(&path)?;
  match ownership.try_lock_exclusive() {
    Ok(()) => {
      ownership.unlock()?;
      Ok(ManagedDaemonState::Stale)
    }
    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
      Ok(ManagedDaemonState::Running(pid_file))
    }
    Err(error) => Err(error).context("failed to inspect daemon PID-file ownership"),
  }
}

/// Information about a freshly spawned background server.
pub struct Started {
  pub pid: u32,
  pub log_path: PathBuf,
  pub pid_file: PathBuf,
  pub setup_token: Option<String>,
}

/// Start `dopbase serve` as a detached background server.
///
/// The foreground command validates the configuration, refuses to run when a
/// daemon is already active for this data directory, spawns the binary again
/// with `--supervised`, and waits for the readiness report before returning.
#[cfg(not(unix))]
pub(crate) async fn start(
  _config: ServerConfig,
  _flags: &[String],
  _json_output: bool,
) -> Result<i32> {
  bail!("--background is only supported on macOS and Linux");
}

#[cfg(unix)]
pub(crate) async fn start(
  config: ServerConfig,
  flags: &[String],
  json_output: bool,
) -> Result<i32> {
  use nix::{sys::signal::kill, unistd::Pid};

  let data_dir = config.data_dir.clone();
  ensure_data_dir(&data_dir)?;
  let pid_path = pid_file_path(&data_dir);
  if let Ok(existing) = read_pid_file(&pid_path) {
    if kill(Pid::from_raw(existing.pid as i32), None).is_ok() {
      bail!(
        "Dopbase server is already running (pid {}, data directory {}). Stop it before starting another one",
        existing.pid,
        data_dir.display()
      );
    }
    let _ = remove_pid_file(&pid_path);
  }
  // A foreground server does not write a PID file, so also check the shared
  // database lock before spawning a detached child.
  let _lock = crate::server::InstanceLock::acquire(&config.database_url)?;
  drop(_lock);
  let started = spawn(&data_dir, flags).await?;
  if !json_output {
    eprintln!(
      "\n{}\n",
      crate::server::startup_banner(&config.public_url, &data_dir, config.docs_enabled,)
    );
  }
  if let Some(token) = &started.setup_token {
    eprintln!("\nDopbase setup token (shown once):\n{token}\n");
  }
  let stop_command = format!("dopbase --data-dir {} stop", data_dir.display());
  if json_output {
    print_value(
      true,
      &serde_json::json!({
          "started": true,
          "version": env!("CARGO_PKG_VERSION"),
          "pid": started.pid,
          "log_file": started.log_path.display().to_string(),
          "pid_file": started.pid_file.display().to_string(),
          "stop_command": stop_command,
      }),
    );
  } else {
    println!(
      "Server started in the background.\nPID:        {}\nLog:        {}\nStop with:  {}",
      started.pid,
      started.log_path.display(),
      stop_command
    );
  }
  Ok(0)
}

/// Spawn the detached server process and wait for its readiness report.
#[cfg(unix)]
async fn spawn(
  data_dir: &Path,
  flags: &[String],
) -> Result<Started> {
  use std::{
    io::pipe,
    os::unix::io::AsRawFd,
    process::{Command, Stdio},
  };

  let log_path = log_file_path(data_dir);
  let log = OpenOptions::new()
    .create(true)
    .append(true)
    .open(&log_path)
    .with_context(|| format!("failed to open daemon log {}", log_path.display()))?;
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(&log_path, fs::Permissions::from_mode(0o600));
  }
  let log_error = log
    .try_clone()
    .context("failed to duplicate the daemon log handle")?;

  let (reader, writer) = pipe().context("failed to create the readiness pipe")?;
  let binary = std::env::current_exe().context("failed to locate the dopbase binary")?;
  let mut command = Command::new(binary);
  command
    .args(flags)
    .stdin(Stdio::null())
    .stdout(Stdio::from(log))
    .stderr(Stdio::from(log_error));
  // Strip DOPBASE_* variables so the child resolves its configuration from
  // the explicit flags above; everything else (PATH, HOME, RUST_LOG, …)
  // passes through.
  for variable in crate::constants::config::daemon_environment_names() {
    command.env_remove(variable);
  }
  {
    use std::os::{
      fd::{BorrowedFd, FromRawFd},
      unix::process::CommandExt,
    };
    command.process_group(0);
    let fd = writer.as_raw_fd();
    unsafe {
      command.pre_exec(move || {
        let _ = nix::unistd::setsid();
        // Re-open the readiness pipe as READY_FD; dup2 also clears the
        // close-on-exec flag std sets on the pipe, so the descriptor
        // survives exec. The returned owner of READY_FD is forgotten —
        // the child keeps it open for the readiness report.
        let target = std::os::fd::OwnedFd::from_raw_fd(READY_FD);
        let duped = nix::unistd::dup2_raw(BorrowedFd::borrow_raw(fd), target)
          .map_err(|error| std::io::Error::from_raw_os_error(error as i32))?;
        std::mem::forget(duped);
        Ok(())
      });
    }
  }
  let child = command
    .spawn()
    .context("failed to start the background server")?;
  drop(writer);
  let spawned_pid = child.id();

  let read = tokio::task::spawn_blocking(move || {
    let mut line = String::new();
    BufReader::new(reader).read_line(&mut line).map(|_| line)
  });
  let line = match tokio::time::timeout(READY_TIMEOUT, read).await {
    Ok(Ok(Ok(line))) if !line.trim().is_empty() => line,
    Ok(_) => bail!(
      "the background server exited before becoming ready; see {}",
      log_path.display()
    ),
    Err(_) => bail!(
      "the background server did not become ready within {}s; see {}",
      READY_TIMEOUT.as_secs(),
      log_path.display()
    ),
  };

  let mut parts = line.split_whitespace();
  let mut setup_token = None;
  let reported_pid;
  match parts.next() {
    Some("ok") => {
      reported_pid = parts.next().and_then(|value| value.parse::<u32>().ok());
      let mut key = parts.next();
      while let Some(name) = key {
        if name == "setup-token" {
          setup_token = parts.next().map(str::to_string);
        }
        key = parts.next();
      }
    }
    Some("error") => {
      let message = line.trim().strip_prefix("error ").unwrap_or(line.trim());
      bail!("{message}; see {}", log_path.display());
    }
    _ => bail!(
      "the background server reported an unexpected readiness state; see {}",
      log_path.display()
    ),
  }

  Ok(Started {
    pid: reported_pid.unwrap_or(spawned_pid),
    log_path,
    pid_file: pid_file_path(data_dir),
    setup_token,
  })
}

/// Stop the background server for a data directory.
///
/// Reads the PID file, sends `SIGTERM` (which the server's graceful shutdown
/// handles), waits for the process to exit, and escalates to `SIGKILL` after
/// the grace period. Stale PID files are reported and removed.
#[cfg(not(unix))]
pub async fn stop_managed(
  _data_dir: Option<&Path>,
  _grace: Duration,
) -> Result<Stopped> {
  bail!("stopping the background server is only supported on macOS and Linux");
}

#[cfg(unix)]
pub async fn stop_managed(
  data_dir: Option<&Path>,
  grace: Duration,
) -> Result<Stopped> {
  use nix::{
    errno::Errno,
    sys::signal::{Signal, kill},
    unistd::Pid,
  };

  let data_dir = resolve_data_dir(data_dir)?;
  let path = pid_file_path(&data_dir);
  if !path.exists() {
    bail!("no running Dopbase daemon (no {})", path.display());
  }
  let pid_file = match read_pid_file(&path) {
    Ok(pid_file) => pid_file,
    Err(error) => {
      let _ = remove_pid_file(&path);
      return Err(error.context(format!(
        "removed the unreadable PID file {}; if a daemon is still running, stop it manually",
        path.display()
      )));
    }
  };
  // The daemon holds an exclusive advisory lock on its PID file for its
  // entire lifetime. An unlocked file is stale even if its PID has since
  // been reused by an unrelated live process, so it must never be signalled.
  let ownership = OpenOptions::new().read(true).write(true).open(&path)?;
  match ownership.try_lock_exclusive() {
    Ok(()) => {
      let _ = ownership.unlock();
      let _ = remove_pid_file(&path);
      bail!(
        "no running Dopbase daemon owns {}; refusing to signal pid {} and removing the stale PID file",
        path.display(),
        pid_file.pid,
      );
    }
    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
    Err(error) => return Err(error).context("failed to verify daemon PID-file ownership"),
  }
  let pid = Pid::from_raw(pid_file.pid as i32);
  match kill(pid, None) {
    Err(Errno::ESRCH) => {
      let _ = remove_pid_file(&path);
      bail!(
        "no running Dopbase daemon (stale {} for pid {})",
        path.display(),
        pid_file.pid
      );
    }
    Err(error) => bail!(
      "failed to inspect the daemon process (pid {}): {error}",
      pid_file.pid
    ),
    Ok(()) => {}
  }
  kill(pid, Signal::SIGTERM)
    .with_context(|| format!("failed to signal the daemon process (pid {})", pid_file.pid))?;

  let mut forced = false;
  let mut stopped = wait_for_exit(pid, grace + Duration::from_secs(5)).await;
  if !stopped {
    kill(pid, Signal::SIGKILL)
      .with_context(|| format!("failed to force-stop the daemon (pid {})", pid_file.pid))?;
    forced = true;
    stopped = wait_for_exit(pid, Duration::from_secs(5)).await;
  }
  if !stopped {
    bail!(
      "the daemon (pid {}) did not stop; check {}",
      pid_file.pid,
      path.display()
    );
  }
  let _ = remove_pid_file(&path);
  Ok(Stopped {
    pid: pid_file.pid,
    forced,
  })
}

pub async fn stop(
  data_dir: Option<&Path>,
  grace: Duration,
  json_output: bool,
) -> Result<i32> {
  let stopped = stop_managed(data_dir, grace).await?;
  print_value(
    json_output,
    &serde_json::json!({"stopped": true, "pid": stopped.pid, "forced": stopped.forced}),
  );
  Ok(0)
}

#[cfg(unix)]
async fn wait_for_exit(
  pid: nix::unistd::Pid,
  timeout: Duration,
) -> bool {
  use nix::sys::signal::kill;
  let deadline = tokio::time::Instant::now() + timeout;
  loop {
    if kill(pid, None).is_err() {
      return true;
    }
    if tokio::time::Instant::now() >= deadline {
      return false;
    }
    tokio::time::sleep(STOP_POLL_INTERVAL).await;
  }
}

fn print_value(
  json_output: bool,
  value: &serde_json::Value,
) {
  if json_output {
    println!(
      "{}",
      serde_json::to_string_pretty(value).unwrap_or_else(|_| "null".into())
    );
  } else if let Some(value) = value.as_str() {
    println!("{value}");
  } else {
    println!(
      "{}",
      serde_json::to_string_pretty(value).unwrap_or_else(|_| "Done.".into())
    );
  }
}
