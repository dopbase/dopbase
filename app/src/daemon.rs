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

use crate::server::provision::{GeneratedSetup, SetupCommittedError, StartupMode};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::{
  config::{
    EnvironmentOverrides, ServerConfig, ServerOverrides, ensure_data_dir, resolve_data_dir,
  },
  constants::config::{DAEMON_LOG_FILENAME, DAEMON_PID_FILENAME},
};

/// File descriptor the supervised server reports readiness on.
const READY_FD: i32 = 3;
pub(crate) const ENV_DAEMON_LAUNCH: &str = "DOPBASE_INTERNAL_DAEMON_LAUNCH";

/// Server-only launch inputs, saved in the private PID file for restart.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LaunchDescriptor {
  pub version: u32,
  pub overrides: ServerOverrides,
  pub environment: EnvironmentOverrides,
  pub working_directory: PathBuf,
}

impl LaunchDescriptor {
  pub(crate) fn config(&self) -> Result<ServerConfig> {
    if self.version != 1 {
      bail!(
        "unsupported saved server launch version {}. Stop the server and start it again with `dopbase server start --background`",
        self.version
      );
    }
    if !self.working_directory.is_absolute() || !self.working_directory.is_dir() {
      bail!(
        "the original server working directory is unavailable: {}",
        self.working_directory.display()
      );
    }
    let mut config = ServerConfig::load_with_environment_at(
      &self.overrides,
      self.environment.clone(),
      &self.working_directory,
    )?;
    config.daemonized = true;
    config.daemon_launch = Some(self.clone());
    Ok(config)
  }
}
const READY_TIMEOUT: Duration = Duration::from_secs(30);
const STOP_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// One-shot readiness reporter used by the supervised server process.
///
/// When the server is started by [`start`], the parent process inherits a pipe
/// write end into the child as `READY_FD`. The child reports `ok …` or
/// `error …` exactly once so the foreground command can relay the real startup
/// error (bind failure, master-key problem, …) to the user.
pub struct Ready {
  writer: Mutex<Box<dyn Write + Send>>,
  reported: AtomicBool,
}

impl Ready {
  /// Attach to the readiness pipe inherited from the parent command.
  /// Returns `None` when no inherited write pipe is available.
  pub fn attached() -> Option<Self> {
    #[cfg(unix)]
    {
      use nix::libc;
      use std::{mem::MaybeUninit, os::unix::io::FromRawFd};
      // Only the parent's inherited write pipe belongs to this reporter.
      // Tokio may own descriptor 3 when --supervised is invoked directly.
      let descriptor_flags = unsafe { libc::fcntl(READY_FD, libc::F_GETFD) };
      if descriptor_flags < 0 || descriptor_flags & libc::FD_CLOEXEC != 0 {
        return None;
      }
      let mut metadata = MaybeUninit::<libc::stat>::uninit();
      if unsafe { libc::fstat(READY_FD, metadata.as_mut_ptr()) } != 0 {
        return None;
      }
      // fstat initialized metadata on success.
      let metadata = unsafe { metadata.assume_init() };
      let access_flags = unsafe { libc::fcntl(READY_FD, libc::F_GETFL) };
      if metadata.st_mode & libc::S_IFMT != libc::S_IFIFO
        || access_flags < 0
        || access_flags & libc::O_ACCMODE != libc::O_WRONLY
      {
        return None;
      }
      // The inherited pipe is open and has no existing Rust owner in the child.
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

  pub(crate) fn ok_with_setup(
    &self,
    pid: u32,
    setup: &GeneratedSetup,
  ) -> Result<()> {
    let payload = Zeroizing::new(serde_json::to_string(setup)?);
    let line = Zeroizing::new(format!("ok {pid} setup-json {}", payload.as_str()));
    self.report_checked(&line)
  }

  pub(crate) fn fail_error(
    &self,
    error: &anyhow::Error,
  ) {
    if let Some(committed) = error.downcast_ref::<SetupCommittedError>() {
      let payload = serde_json::json!({
        "email": committed.email, "data_dir": committed.data_dir,
        "message": committed.reason.to_string(),
      });
      self.report(format!("error-setup {payload}"));
    } else {
      self.fail(&format!("{error:#}"));
    }
  }

  /// Report a startup failure. `message` must be a single line, so any
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
    let _ = self.report_checked(&line);
  }

  fn report_checked(
    &self,
    line: &str,
  ) -> Result<()> {
    if self.reported.swap(true, Ordering::SeqCst) {
      return Ok(());
    }
    let mut writer = self
      .writer
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner());
    writer.write_all(line.as_bytes())?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
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
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub launch: Option<Box<LaunchDescriptor>>,
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
  write_pid_file_with_launch(path, pid, bind_address, public_url, None)
}

pub(crate) fn write_pid_file_with_launch(
  path: &Path,
  pid: u32,
  bind_address: &str,
  public_url: &str,
  launch: Option<LaunchDescriptor>,
) -> Result<File> {
  let payload = PidFile {
    pid,
    started_at: chrono::Utc::now().to_rfc3339(),
    version: env!("CARGO_PKG_VERSION").to_string(),
    bind_address: bind_address.to_string(),
    public_url: Some(public_url.to_string()),
    launch: launch.map(Box::new),
  };
  let json = serde_json::to_string_pretty(&payload)?;
  crate::utils::private_file::write(path, json.as_bytes(), true)?;
  let file = OpenOptions::new().read(true).write(true).open(path)?;
  file
    .try_lock()
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
  match ownership.try_lock() {
    Ok(()) => {
      ownership.unlock()?;
      Ok(ManagedDaemonState::Stale)
    }
    Err(std::fs::TryLockError::WouldBlock) => Ok(ManagedDaemonState::Running(pid_file)),
    Err(std::fs::TryLockError::Error(error)) => {
      Err(error).context("failed to inspect daemon PID-file ownership")
    }
  }
}

/// Information about a freshly spawned background server.
pub struct Started {
  pub pid: u32,
  pub log_path: PathBuf,
  pub pid_file: PathBuf,
  pub setup_token: Option<String>,
  pub(crate) setup: Option<GeneratedSetup>,
  child: std::process::Child,
}

/// Start a supervised background server and report readiness.
pub(crate) async fn start(
  config: ServerConfig,
  json_output: bool,
) -> Result<i32> {
  let mut started = start_managed(&config).await?;
  if let Err(error) = report_started(&config, &started, json_output, None) {
    if let Some(setup) = &started.setup {
      // This command is still the child's parent. Reap it here rather than
      // waiting for a detached-process exit check, which would see a zombie.
      let _ = started.child.kill();
      let stopped = started.child.wait();
      let _ = remove_pid_file(&started.pid_file);
      let reason = match stopped {
        Ok(_) => error.context("failed to deliver setup output"),
        Err(stop_error) => error.context(format!(
          "failed to deliver setup output; failed to reap the server: {stop_error}"
        )),
      };
      return Err(setup.committed_error(reason));
    }
    return Err(error);
  }
  Ok(0)
}

pub(crate) async fn restart(
  config: ServerConfig,
  expected: &PidFile,
  timeout: Duration,
  json_output: bool,
) -> Result<i32> {
  crate::server::require_initialized(&config).await?;
  let stopped = stop_managed_for(Some(&config.data_dir), timeout, Some(expected)).await?;
  let started = start_managed(&config).await.map_err(|error| {
    error.context(format!(
      "the server stopped, but restart failed. Check {}",
      log_file_path(&config.data_dir).display()
    ))
  })?;
  report_started(&config, &started, json_output, Some(&stopped))?;
  Ok(0)
}

#[cfg(not(unix))]
async fn start_managed(_config: &ServerConfig) -> Result<Started> {
  bail!("--background is only supported on macOS and Linux");
}

#[cfg(unix)]
async fn start_managed(config: &ServerConfig) -> Result<Started> {
  let root_email = match crate::server::provision::startup_mode(config).await? {
    StartupMode::Generated(email) => Some(email),
    _ => None,
  };
  let data_dir = &config.data_dir;
  ensure_data_dir(data_dir)?;
  let pid_path = pid_file_path(data_dir);
  match inspect(data_dir)? {
    ManagedDaemonState::Running(existing) => bail!(
      "Dopbase server is already running (pid {}, data directory {}). Stop it with `dopbase server stop` before starting another one",
      existing.pid,
      data_dir.display()
    ),
    ManagedDaemonState::Stale => remove_pid_file(&pid_path)?,
    ManagedDaemonState::Absent => {}
  }
  // Foreground and background servers share the database lock.
  let lock = crate::server::InstanceLock::acquire(&config.database_url)?;
  drop(lock);
  spawn(
    data_dir,
    config.daemon_launch.as_ref(),
    root_email.as_deref(),
  )
  .await
}

fn report_started(
  config: &ServerConfig,
  started: &Started,
  json_output: bool,
  stopped: Option<&Stopped>,
) -> Result<()> {
  if !json_output {
    anstream::eprintln!(
      "\n{}\n",
      crate::server::startup_banner(
        &config.public_url,
        &config.bind_address,
        &config.data_dir,
        config.docs_enabled,
        config.web_ui_enabled,
      )
    );
  }
  if let Some(warning) = config.public_url_warning() {
    anstream::eprintln!("{}\n", crate::server::startup_warning(&warning));
  }
  if let Some(token) = &started.setup_token {
    eprintln!(
      "{}",
      crate::server::setup_token_message(&config.public_url, token)
    );
  }

  let stop_command = "dopbase server stop".to_owned();
  let restart_command = "dopbase server restart".to_owned();
  if json_output {
    let mut value = serde_json::json!({
      "started": true, "version": env!("CARGO_PKG_VERSION"), "pid": started.pid,
      "log_file": started.log_path, "pid_file": started.pid_file,
      "public_url": config.public_url, "bind_address": config.bind_address,
      "stop_command": stop_command, "restart_command": restart_command,
    });
    if let Some(stopped) = stopped {
      value["restarted"] = true.into();
      value["previous_pid"] = stopped.pid.into();
      value["forced"] = stopped.forced.into();
    }
    #[derive(Serialize)]
    struct StartOutput<'a> {
      #[serde(flatten)]
      fields: serde_json::Value,
      #[serde(skip_serializing_if = "Option::is_none")]
      setup: Option<&'a GeneratedSetup>,
    }
    let output = Zeroizing::new(
      serde_json::to_string_pretty(&StartOutput {
        fields: value,
        setup: started.setup.as_ref(),
      })?
        + "\n",
    );
    crate::cli::output::print_raw(&output)?;
  } else {
    if let Some(setup) = &started.setup {
      crate::cli::output::print_raw(&setup.human_output())?;
    }
    let action = if stopped.is_some() {
      "restarted"
    } else {
      "started"
    };
    println!("Server {action} in the background.");
    println!(
      "{}",
      crate::cli::output::render_fields(&[
        ("PID:", started.pid.to_string()),
        ("Log:", started.log_path.display().to_string()),
        ("Stop with:", stop_command),
        ("Restart with:", restart_command),
      ])
    );
    if stopped.is_some_and(|stopped| stopped.forced) {
      eprintln!("The previous server required a forced shutdown.");
    }
  }
  Ok(())
}

/// Spawn the detached server process and wait for its readiness report.
#[cfg(unix)]
async fn spawn(
  data_dir: &Path,
  launch: Option<&LaunchDescriptor>,
  root_email: Option<&str>,
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
    .args(["server", "start", "--supervised"])
    .stdin(Stdio::null())
    .stdout(Stdio::from(log))
    .stderr(Stdio::from(log_error));
  command.env_remove(ENV_DAEMON_LAUNCH);
  command.env_remove(crate::constants::config::ENV_ROOT_EMAIL);
  if let Some(email) = root_email {
    command.env(crate::constants::config::ENV_ROOT_EMAIL, email);
  }
  if let Some(launch) = launch {
    command.current_dir(&launch.working_directory);
    command.env(ENV_DAEMON_LAUNCH, serde_json::to_string(launch)?);
  }
  // Remove live server overrides; the child uses the captured launch inputs.
  // Other process settings such as PATH, HOME, and RUST_LOG pass through.
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
        // Re-open the readiness pipe as READY_FD. dup2 also clears the
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
  let mut child = command
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
    Ok(_) => {
      let _ = child.kill();
      let _ = child.wait();
      bail!(
        "the background server exited before becoming ready. See {}",
        log_path.display()
      );
    }
    Err(_) => {
      let _ = child.kill();
      let _ = child.wait();
      bail!(
        "the background server did not become ready within {}s. See {}",
        READY_TIMEOUT.as_secs(),
        log_path.display()
      );
    }
  };

  let line = Zeroizing::new(line);
  let (status, setup_json) = match line.split_once(" setup-json ") {
    Some((status, payload)) => (status, Some(payload)),
    None => (line.as_str(), None),
  };
  let setup: Option<GeneratedSetup> = setup_json.map(serde_json::from_str).transpose()?;
  let mut parts = status.split_whitespace();
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
    Some("error-setup") => {
      let _ = child.kill();
      let _ = child.wait();
      let value: serde_json::Value =
        serde_json::from_str(status.strip_prefix("error-setup ").unwrap_or(""))?;
      return Err(
        SetupCommittedError {
          email: value["email"].as_str().unwrap_or_default().to_owned(),
          data_dir: serde_json::from_value(value["data_dir"].clone())?,
          reason: anyhow::anyhow!(
            "{}",
            value["message"]
              .as_str()
              .unwrap_or("background setup failed")
          ),
        }
        .into(),
      );
    }
    Some("error") => {
      let message = line.trim().strip_prefix("error ").unwrap_or(line.trim());
      let _ = child.kill();
      let _ = child.wait();
      bail!("{message}. See {}", log_path.display());
    }
    _ => bail!(
      "the background server reported an unexpected readiness state. See {}",
      log_path.display()
    ),
  }

  Ok(Started {
    pid: reported_pid.unwrap_or(spawned_pid),
    log_path,
    pid_file: pid_file_path(data_dir),
    setup_token,
    setup,
    child,
  })
}

/// Stop the background server for a data directory.
///
/// Reads the PID file, sends `SIGTERM` (which the server's graceful shutdown
/// handles), waits for the process to exit, and escalates to `SIGKILL` after
/// the grace period. Stale PID files are reported and removed.
pub async fn stop_managed(
  data_dir: Option<&Path>,
  grace: Duration,
) -> Result<Stopped> {
  stop_managed_for(data_dir, grace, None).await
}

#[cfg(not(unix))]
async fn stop_managed_for(
  _data_dir: Option<&Path>,
  _grace: Duration,
  _expected: Option<&PidFile>,
) -> Result<Stopped> {
  bail!("stopping the background server is only supported on macOS and Linux");
}

#[cfg(unix)]
async fn stop_managed_for(
  data_dir: Option<&Path>,
  grace: Duration,
  expected: Option<&PidFile>,
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
        "removed the unreadable PID file {}. If a daemon is still running, stop it manually",
        path.display()
      )));
    }
  };
  if expected.is_some_and(|expected| {
    expected.pid != pid_file.pid || expected.started_at != pid_file.started_at
  }) {
    bail!("the running server changed while preparing restart. Run `dopbase server restart` again");
  }
  // The daemon holds an exclusive advisory lock on its PID file for its
  // entire lifetime. An unlocked file is stale even if its PID has since
  // been reused by an unrelated live process, so it must never be signalled.
  let ownership = OpenOptions::new().read(true).write(true).open(&path)?;
  match ownership.try_lock() {
    Ok(()) => {
      let _ = ownership.unlock();
      let _ = remove_pid_file(&path);
      bail!(
        "no running Dopbase daemon owns {}. Refusing to signal pid {} and removing the stale PID file",
        path.display(),
        pid_file.pid,
      );
    }
    Err(std::fs::TryLockError::WouldBlock) => {}
    Err(std::fs::TryLockError::Error(error)) => {
      return Err(error).context("failed to verify daemon PID-file ownership");
    }
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
      "the daemon (pid {}) did not stop. Check {}",
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
  if json_output {
    print_value(
      true,
      &serde_json::json!({"stopped": true, "pid": stopped.pid, "forced": stopped.forced}),
    );
  } else if stopped.forced {
    println!("Dopbase server stopped forcefully.");
  } else {
    println!("Dopbase server stopped successfully.");
  }
  Ok(0)
}

pub async fn logs(
  data_dir: Option<&Path>,
  line_count: usize,
  clean: bool,
  watch: bool,
  json_output: bool,
) -> Result<i32> {
  let data_dir = resolve_data_dir(data_dir)?;
  let path = log_file_path(&data_dir);
  if clean {
    OpenOptions::new()
      .create(true)
      .write(true)
      .truncate(true)
      .open(&path)
      .with_context(|| {
        format!(
          "failed to clear background server log at {}",
          path.display()
        )
      })?;
    #[cfg(unix)]
    {
      use std::os::unix::fs::PermissionsExt;
      fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).with_context(|| {
        format!(
          "failed to secure background server log at {}",
          path.display()
        )
      })?;
    }
    if json_output {
      print_value(
        true,
        &serde_json::json!({"log_file": path, "cleaned": true}),
      );
      return Ok(0);
    }
    if !watch {
      println!("Background server log cleared.");
      return Ok(0);
    }
    println!("Background server log cleared. Waiting for new output.");
  }

  let mut contents = if clean {
    String::new()
  } else {
    fs::read_to_string(&path)
      .with_context(|| format!("no background server log found at {}", path.display()))?
  };
  let lines = tail_lines(&contents, line_count);
  if json_output {
    print_value(true, &serde_json::json!({"log_file": path, "lines": lines}));
    return Ok(0);
  }
  for line in &lines {
    println!("{line}");
  }
  if !watch {
    return Ok(0);
  }

  let mut offset = contents.len();
  loop {
    tokio::select! {
      _ = tokio::signal::ctrl_c() => return Ok(0),
      _ = tokio::time::sleep(Duration::from_millis(250)) => {}
    }
    contents =
      fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    if contents.len() < offset {
      offset = 0;
    }
    if contents.len() > offset {
      print!("{}", &contents[offset..]);
      std::io::stdout().flush()?;
      offset = contents.len();
    }
  }
}

pub fn tail_lines(
  contents: &str,
  line_count: usize,
) -> Vec<&str> {
  let lines = contents.lines().collect::<Vec<_>>();
  let start = lines.len().saturating_sub(line_count);
  lines[start..].to_vec()
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
