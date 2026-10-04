use std::{
  fs::{self, OpenOptions},
  io::{ErrorKind, Write},
  path::{Path, PathBuf},
};

#[cfg(unix)]
use std::fs::File;

use anyhow::{Context, Result, bail};

/// Atomically writes a mode-0600 file through a unique sibling temporary.
/// `replace = false` commits with `hard_link`, whose destination creation is
/// atomic and cannot overwrite a file created by a competing process.
pub fn write(
  path: &Path,
  contents: &[u8],
  replace: bool,
) -> Result<()> {
  let mode = if replace {
    ExistingFile::Replace
  } else {
    ExistingFile::Reject
  };
  write_with_mode(path, contents, mode).map(|_| ())
}

/// Create a private file atomically, leaving an existing destination untouched.
/// Returns true when created, or false when the destination already exists.
pub fn write_if_missing(
  path: &Path,
  contents: &[u8],
) -> Result<bool> {
  match fs::symlink_metadata(path) {
    Ok(_) => return Ok(false),
    Err(error) if error.kind() == ErrorKind::NotFound => {}
    Err(error) => {
      return Err(error).with_context(|| format!("failed to inspect {}", path.display()));
    }
  }
  write_with_mode(path, contents, ExistingFile::Preserve)
}

#[derive(Clone, Copy)]
enum ExistingFile {
  Replace,
  Reject,
  Preserve,
}

fn write_with_mode(
  path: &Path,
  contents: &[u8],
  mode: ExistingFile,
) -> Result<bool> {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent)
      .with_context(|| format!("failed to create parent directory for {}", path.display()))?;
  }
  let temporary =
    unique_temporary(path).with_context(|| format!("failed to write {}", path.display()))?;
  let result = (|| {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
      use std::os::unix::fs::OpenOptionsExt;
      options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    file.write_all(contents)?;
    file.sync_all()?;
    drop(file);

    if matches!(mode, ExistingFile::Replace) {
      replace_file(&temporary, path)?;
    } else {
      match fs::hard_link(&temporary, path) {
        Ok(()) => fs::remove_file(&temporary)?,
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
          if matches!(mode, ExistingFile::Preserve) {
            fs::remove_file(&temporary)?;
            return Ok(false);
          }
          bail!(
            "{} already exists. Pass --force to overwrite it",
            path.display()
          )
        }
        Err(error) => return Err(error.into()),
      }
    }
    #[cfg(unix)]
    if let Some(parent) = path
      .parent()
      .filter(|parent| !parent.as_os_str().is_empty())
    {
      File::open(parent)?.sync_all()?;
    }
    Ok(true)
  })();
  if result.is_err() {
    let _ = fs::remove_file(&temporary);
  }
  result.with_context(|| format!("failed to write {}", path.display()))
}

fn replace_file(
  source: &Path,
  destination: &Path,
) -> std::io::Result<()> {
  fs::rename(source, destination)
}

fn unique_temporary(path: &Path) -> Result<PathBuf> {
  let name = path
    .file_name()
    .and_then(|name| name.to_str())
    .unwrap_or("dopbase");
  for _ in 0..16 {
    let mut random = [0_u8; 12];
    getrandom::fill(&mut random)?;
    let suffix = random
      .iter()
      .map(|byte| format!("{byte:02x}"))
      .collect::<String>();
    let candidate = path.with_file_name(format!(".{name}.{suffix}.tmp"));
    if !candidate.exists() {
      return Ok(candidate);
    }
  }
  bail!("could not allocate a unique temporary file")
}
