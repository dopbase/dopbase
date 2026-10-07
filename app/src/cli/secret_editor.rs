use anyhow::{Context, Result, bail};
use std::{
  env,
  fs::{self, File, OpenOptions},
  io::{Read, Write},
  path::{Path, PathBuf},
  process::Stdio,
};
use tempfile::TempDir;
use tokio::process::{Child, Command};
use zeroize::Zeroizing;

pub const MAX_EDITOR_BYTES: u64 = 8 * 1024 * 1024;

pub struct Editor {
  executable: PathBuf,
  arguments: Vec<String>,
  profile: Option<String>,
}

impl Editor {
  pub fn resolve(explicit: Option<&str>) -> Result<Self> {
    if let Some(command) = explicit
      .map(str::to_owned)
      .or_else(|| {
        env::var("VISUAL")
          .ok()
          .filter(|value| !value.trim().is_empty())
      })
      .or_else(|| {
        env::var("EDITOR")
          .ok()
          .filter(|value| !value.trim().is_empty())
      })
    {
      return Self::from_command(&command);
    }
    for name in ["nvim", "vim", "hx", "nano"] {
      if let Ok(editor) = Self::from_command(name) {
        return Ok(editor);
      }
    }
    bail!("no terminal editor found. Set VISUAL or EDITOR, or pass --editor");
  }

  pub fn from_command(command: &str) -> Result<Self> {
    let mut words = shlex::split(command)
      .context("editor command has invalid quoting")?
      .into_iter();
    let name = words
      .next()
      .filter(|word| !word.is_empty())
      .context("editor command is empty")?;
    let arguments: Vec<_> = words.collect();
    let path = Path::new(&name);
    let candidates: Vec<PathBuf> = if path.components().count() > 1 {
      vec![env::current_dir()?.join(path)]
    } else {
      env::split_paths(&env::var_os("PATH").unwrap_or_default())
        .map(|root| root.join(path))
        .collect()
    };
    let executable = candidates
      .into_iter()
      .find(|path| {
        let Ok(metadata) = fs::metadata(path) else {
          return false;
        };
        #[cfg(unix)]
        {
          use std::os::unix::fs::PermissionsExt;
          metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
        }
        #[cfg(not(unix))]
        {
          metadata.is_file()
        }
      })
      .context("editor executable was not found or is not executable")?;
    // Identify the invoked name before canonicalizing (editors can be symlinks).
    let profile = if arguments.is_empty() {
      path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| matches!(*name, "vim" | "nvim" | "hx" | "helix" | "nano"))
        .map(str::to_owned)
    } else {
      None
    };
    let executable = fs::canonicalize(executable)?;
    // macOS ships `nano` as a symlink to Pico, whose flags differ from nano.
    let profile = if profile.as_deref() == Some("nano")
      && executable.file_name().is_some_and(|name| name == "pico")
    {
      None
    } else {
      profile
    };
    Ok(Self {
      executable,
      arguments,
      profile,
    })
  }

  pub fn protected(&self) -> bool {
    self.profile.is_some()
  }

  pub fn command(
    &self,
    session: &Session,
  ) -> Result<Command> {
    let mut command = Command::new(&self.executable);
    command
      .current_dir(session.path())
      .env_clear()
      .kill_on_drop(true);
    for name in [
      "PATH",
      "TERM",
      "COLORTERM",
      "LANG",
      "LC_ALL",
      "LC_CTYPE",
      "TZ",
    ] {
      if let Some(value) = env::var_os(name) {
        command.env(name, value);
      }
    }
    // Keep editor configuration, state, and any incidental files in the session.
    for (name, directory) in [
      ("XDG_CONFIG_HOME", "config"),
      ("XDG_CACHE_HOME", "cache"),
      ("XDG_DATA_HOME", "data"),
      ("XDG_STATE_HOME", "state"),
    ] {
      let path = session.path().join(directory);
      fs::create_dir_all(&path)?;
      secure_directory(&path)?;
      command.env(name, path);
    }
    command.env("TMPDIR", session.path());
    match self.profile.as_deref() {
      Some("vim" | "nvim") => {
        command.args([
          "-N",
          "-u",
          "NONE",
          "-i",
          "NONE",
          "-n",
          "--noplugin",
          "--cmd",
          "set noswapfile nobackup nowritebackup noundofile nomodeline noexrc",
        ]);
        if self.profile.as_deref() == Some("vim") {
          command.args(["-U", "NONE"]);
        } else {
          command.env("NVIM_LOG_FILE", session.path().join("nvim.log"));
        }
        command.arg("--");
      }
      Some("nano") => {
        command.args(["-I", "-R", "-w", "--"]);
      }
      Some("hx" | "helix") => {
        let config = session.path().join("editor.toml");
        crate::utils::private_file::write(
          &config,
          b"[editor]\nauto-format = false\n[editor.lsp]\nenable = false\n",
          true,
        )?;
        command
          .arg("-c")
          .arg(config)
          .arg("--log")
          .arg(session.path().join("helix.log"));
      }
      _ => {
        command.args(&self.arguments);
      }
    }
    command
      .arg(session.file_path())
      .stdin(Stdio::inherit())
      .stdout(Stdio::inherit())
      .stderr(Stdio::inherit());
    #[cfg(unix)]
    unsafe {
      command.pre_exec(|| {
        nix::libc::umask(0o077);
        Ok(())
      });
    }
    Ok(command)
  }
}

pub struct Session {
  directory: Option<TempDir>,
  _lock: File,
  child: Option<Child>,
  _terminal: TerminalRestore,
}

impl Session {
  pub fn new(root: &Path) -> Result<Self> {
    fs::create_dir_all(root)?;
    #[cfg(unix)]
    {
      use std::os::unix::fs::{MetadataExt, PermissionsExt};
      let parent = fs::metadata(root.parent().context("editor session root has no parent")?)?;
      if parent.uid() != unsafe { nix::libc::geteuid() } || parent.permissions().mode() & 0o022 != 0
      {
        bail!("editor session parent must be owned by you and not writable by other users");
      }
    }
    secure_directory(root)?;
    let registry_lock = open_private(&root.join(".lock"), false)?;
    registry_lock.lock()?;
    // Creation and scavenging share a lock, so a new session cannot be swept
    // before its own lifetime lock has been acquired.
    for entry in fs::read_dir(root)? {
      let entry = entry?;
      if !entry.file_name().to_string_lossy().starts_with("session-") {
        continue;
      }
      let path = entry.path();
      if check_directory(&path).is_err() {
        continue;
      }
      let Ok(lock) = open_private(&path.join(".lock"), false) else {
        continue;
      };
      if lock.try_lock().is_ok() {
        fs::remove_dir_all(&path).context("failed to remove abandoned editor session")?;
      }
    }
    let directory = tempfile::Builder::new()
      .prefix("session-")
      .tempdir_in(root)?;
    secure_directory(directory.path())?;
    let lock = open_private(&directory.path().join(".lock"), true)?;
    lock.lock()?;
    let file = open_private(&directory.path().join("secrets.env"), true)?;
    drop(file);
    drop(registry_lock);
    Ok(Self {
      directory: Some(directory),
      _lock: lock,
      child: None,
      _terminal: TerminalRestore::capture(),
    })
  }

  pub fn path(&self) -> &Path {
    self
      .directory
      .as_ref()
      .expect("active editor session")
      .path()
  }
  pub fn file_path(&self) -> PathBuf {
    self.path().join("secrets.env")
  }

  pub fn write(
    &self,
    text: &str,
  ) -> Result<()> {
    if text.len() as u64 > MAX_EDITOR_BYTES {
      bail!("editor document exceeds 8 MiB");
    }
    let mut file = read_file(&self.file_path(), true)?;
    file.set_len(0)?;
    file.write_all(text.as_bytes())?;
    Ok(())
  }

  pub fn read(&self) -> Result<Zeroizing<String>> {
    check_directory(self.path())?;
    let file = read_file(&self.file_path(), false)?;
    let mut text = Zeroizing::new(String::new());
    file
      .take(MAX_EDITOR_BYTES + 1)
      .read_to_string(&mut text)
      .context("edited file must contain valid UTF-8")?;
    if text.len() as u64 > MAX_EDITOR_BYTES {
      bail!("editor document exceeds 8 MiB");
    }
    Ok(text)
  }

  pub async fn run(
    &mut self,
    editor: &Editor,
  ) -> Result<()> {
    let child = editor
      .command(self)?
      .spawn()
      .context("failed to start editor")?;
    self.child = Some(child);
    let status = self
      .child
      .as_mut()
      .unwrap()
      .wait()
      .await
      .context("failed to wait for editor")?;
    self.child.take();
    if !status.success() {
      bail!("editor exited unsuccessfully; no changes were saved");
    }
    Ok(())
  }

  pub async fn close(mut self) -> Result<()> {
    if let Some(mut child) = self.child.take() {
      let _ = child.start_kill();
      child.wait().await.context("failed to stop editor")?;
    }
    let directory = self.directory.take().unwrap();
    let path = directory.path().to_owned();
    directory.close().with_context(|| {
      format!(
        "could not remove plaintext editor session at {}",
        path.display()
      )
    })
  }
}

fn open_private(
  path: &Path,
  exclusive: bool,
) -> Result<File> {
  let mut options = OpenOptions::new();
  options.read(true).write(true);
  if exclusive {
    options.create_new(true);
  } else {
    options.create(true);
  }
  #[cfg(unix)]
  {
    use std::os::unix::fs::OpenOptionsExt;
    options
      .mode(0o600)
      .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK);
  }
  let file = options.open(path)?;
  check_file(&file)?;
  Ok(file)
}

fn read_file(
  path: &Path,
  write: bool,
) -> Result<File> {
  let mut options = OpenOptions::new();
  options.read(true).write(write);
  #[cfg(unix)]
  {
    use std::os::unix::fs::OpenOptionsExt;
    options.custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK);
  }
  let file = options
    .open(path)
    .context("could not safely open the edited file")?;
  check_file(&file)?;
  if file.metadata()?.len() > MAX_EDITOR_BYTES {
    bail!("editor document exceeds 8 MiB");
  }
  Ok(file)
}

fn check_file(file: &File) -> Result<()> {
  let metadata = file.metadata()?;
  if !metadata.is_file() {
    bail!("editor file must be a regular file");
  }
  #[cfg(unix)]
  {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    if metadata.uid() != unsafe { nix::libc::geteuid() }
      || metadata.permissions().mode() & 0o077 != 0
      || metadata.nlink() != 1
    {
      bail!("editor file must be owned by you, private, and have no hard links");
    }
  }
  Ok(())
}

fn secure_directory(path: &Path) -> Result<()> {
  let metadata = fs::symlink_metadata(path)?;
  if !metadata.is_dir() {
    bail!("editor directory must not be a symlink");
  }
  #[cfg(unix)]
  {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    if metadata.uid() != unsafe { nix::libc::geteuid() } {
      bail!("editor directory must be owned by you");
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
  }
  check_directory(path)
}

fn check_directory(path: &Path) -> Result<()> {
  let metadata = fs::symlink_metadata(path)?;
  if !metadata.is_dir() {
    bail!("editor directory must not be a symlink");
  }
  #[cfg(unix)]
  {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    if metadata.uid() != unsafe { nix::libc::geteuid() }
      || metadata.permissions().mode() & 0o077 != 0
    {
      bail!("editor directory must be owned by you and private");
    }
  }
  Ok(())
}

struct TerminalRestore {
  #[cfg(unix)]
  original: Option<nix::libc::termios>,
}
impl TerminalRestore {
  fn capture() -> Self {
    #[cfg(unix)]
    {
      let mut original = std::mem::MaybeUninit::uninit();
      let original = unsafe {
        if nix::libc::tcgetattr(0, original.as_mut_ptr()) == 0 {
          Some(original.assume_init())
        } else {
          None
        }
      };
      Self { original }
    }
    #[cfg(not(unix))]
    {
      Self {}
    }
  }
}
impl Drop for TerminalRestore {
  fn drop(&mut self) {
    #[cfg(unix)]
    if let Some(original) = &self.original {
      unsafe {
        nix::libc::tcsetattr(0, nix::libc::TCSANOW, original);
      }
    }
  }
}

pub(crate) fn disable_core_dumps() -> Result<()> {
  #[cfg(unix)]
  {
    let limit = nix::libc::rlimit {
      rlim_cur: 0,
      rlim_max: 0,
    };
    if unsafe { nix::libc::setrlimit(nix::libc::RLIMIT_CORE, &limit) } != 0 {
      return Err(std::io::Error::last_os_error())
        .context("failed to disable core dumps for secret editing");
    }
  }
  Ok(())
}
