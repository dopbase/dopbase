use super::AdminCommand;
use crate::cli::{output, prompt};
use crate::config::{ServerConfig, ServerOverrides, database_path, ensure_data_dir};
use anyhow::{Context, Result, bail};
use serde_json::json;
use std::{
  fs::{self, File, OpenOptions},
  io::{self, IsTerminal},
  path::{Path, PathBuf},
};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

pub(crate) async fn execute(
  command: AdminCommand,
  data_dir: Option<PathBuf>,
  json_output: bool,
) -> Result<i32> {
  match command {
    AdminCommand::ResetPassword {
      email,
      config,
      master_key_file,
    } => reset_password(email, data_dir, config, master_key_file).await?,
    AdminCommand::FactoryReset { config, no_backup } => {
      factory_reset_offline(data_dir, config, no_backup, json_output).await?
    }
  }
  Ok(0)
}

const FACTORY_RESET_CONFIRMATION: &str = "please-wipe-out-system";

pub fn factory_reset_confirmation_matches(value: &str) -> bool {
  value.trim_end_matches(&['\r', '\n'][..]) == FACTORY_RESET_CONFIRMATION
}

async fn factory_reset_offline(
  data_dir: Option<PathBuf>,
  config: Option<PathBuf>,
  no_backup: bool,
  json_output: bool,
) -> Result<()> {
  if json_output {
    bail!("--json cannot be used with `dopbase admin factory-reset`");
  }
  if !io::stdin().is_terminal() {
    bail!("factory reset requires an interactive terminal");
  }

  let config = ServerConfig::load(&ServerOverrides {
    data_dir,
    config_path: config,
    ..Default::default()
  })?;
  let database = database_path(&config.database_url)?;
  let reset_target = validate_factory_reset_target(&config.data_dir, &database)?;
  let _lock = crate::server::InstanceLock::acquire(&config.database_url)
    .map_err(|error| {
      anyhow::anyhow!(
        "Dopbase must be fully stopped before factory reset. Stop every foreground or background server using this instance.\n{error}"
      )
    })?
    .context("factory reset requires a file-backed SQLite database")?;
  let db = crate::services::db::DbClient::connect(&config.database_url).await?;

  let root: Option<(String, String)> =
    sqlx::query_as("SELECT email,password_hash FROM admins WHERE role='root'")
      .fetch_optional(db.pool())
      .await?;
  let (root_email, root_password_hash) =
    root.context("this instance does not have a Dopbase root account")?;

  output::print_warning(
    "Factory reset removes the entire Dopbase data directory from its active location.",
  );
  eprintln!("Directory: {}", reset_target.display());
  eprintln!(
    "Every database, backup, log, configuration, master-key, and CLI-state file inside this directory will be removed."
  );
  if no_backup {
    output::print_warning(
      "No backup will be created. The data directory will be permanently deleted.",
    );
  } else {
    eprintln!("A ZIP backup will be saved next to the data directory before it is removed.");
  }
  eprintln!("The next server start will create a fresh installation.");
  if no_backup {
    eprintln!("Keep an external backup of anything you need before continuing.");
  }
  eprintln!("You must initialize Dopbase again after continuing.\n");
  let confirmation = prompt::text(
    &format!("Type {FACTORY_RESET_CONFIRMATION} to continue:"),
    crate::cli::CliCancelled::Confirmation,
  )?;
  if !factory_reset_confirmation_matches(&confirmation) {
    bail!("factory reset cancelled: confirmation text did not match");
  }

  let password = prompt::password(
    &format!("Root password for {root_email}:"),
    false,
    crate::cli::CliCancelled::PasswordConfirmation,
  )?;
  if !crate::modules::common::verify_password(&password, &root_password_hash) {
    bail!("the root password is incorrect");
  }

  db.close().await;
  let backup = complete_factory_reset(&reset_target, no_backup)?;
  output::print_success("Factory reset complete.");
  if let Some(backup) = backup {
    output::print_fields(&[("Backup:", backup.display().to_string())]);
  } else {
    output::print_fields(&[("Backup:", "skipped (--no-backup)".into())]);
  }
  output::print_text("Start Dopbase to create a fresh installation and complete first-run setup.");
  Ok(())
}

pub fn complete_factory_reset(
  data_dir: &Path,
  no_backup: bool,
) -> Result<Option<PathBuf>> {
  let quarantine = factory_reset_quarantine_path(data_dir)?;
  let archive = factory_reset_archive_path(&quarantine);
  let archive_root = data_dir
    .file_name()
    .and_then(|name| name.to_str())
    .context("Dopbase data directory does not have a valid UTF-8 name")?;
  let partial_archive = archive.with_file_name(format!(
    ".{}.partial",
    archive
      .file_name()
      .and_then(|name| name.to_str())
      .unwrap_or("factory-reset.zip")
  ));
  if !no_backup && (archive.exists() || partial_archive.exists()) {
    bail!(
      "factory reset backup path already exists: {}",
      archive.display()
    );
  }

  fs::rename(data_dir, &quarantine).with_context(|| {
    format!(
      "failed to move Dopbase data directory {} to {}",
      data_dir.display(),
      quarantine.display()
    )
  })?;

  if no_backup {
    fs::remove_dir_all(&quarantine).with_context(|| {
      format!(
        "factory reset removed the active data directory, but could not delete {}",
        quarantine.display()
      )
    })?;
    return Ok(None);
  }

  if let Err(error) = archive_directory(&quarantine, &partial_archive, &archive, archive_root) {
    let _ = fs::remove_file(&partial_archive);
    if let Err(restore_error) = fs::rename(&quarantine, data_dir) {
      return Err(error).context(format!(
        "the backup failed and the original data directory could not be restored: {restore_error}"
      ));
    }
    return Err(error)
      .context("factory reset was cancelled because the ZIP backup could not be created");
  }

  fs::remove_dir_all(&quarantine).with_context(|| {
    format!(
      "the ZIP backup was created at {}, but the old data remains at {}",
      archive.display(),
      quarantine.display()
    )
  })?;
  Ok(Some(archive))
}

pub fn factory_reset_archive_path(quarantine: &Path) -> PathBuf {
  let mut name = quarantine
    .file_name()
    .map(|name| name.to_os_string())
    .unwrap_or_else(|| "dopbase.factory-reset".into());
  name.push(".zip");
  quarantine.with_file_name(name)
}

fn archive_directory(
  source: &Path,
  partial_archive: &Path,
  archive: &Path,
  archive_root: &str,
) -> Result<()> {
  let mut open_options = OpenOptions::new();
  open_options.write(true).create_new(true);
  #[cfg(unix)]
  {
    use std::os::unix::fs::OpenOptionsExt;
    open_options.mode(0o600);
  }
  let file = open_options
    .open(partial_archive)
    .with_context(|| format!("failed to create {}", partial_archive.display()))?;
  let mut zip = ZipWriter::new(file);
  add_zip_directory(&mut zip, source, archive_root)?;
  let file = zip
    .finish()
    .context("failed to finish factory reset ZIP backup")?;
  file
    .sync_all()
    .context("failed to sync factory reset ZIP backup")?;
  fs::rename(partial_archive, archive).with_context(|| {
    format!(
      "failed to publish factory reset ZIP backup {}",
      archive.display()
    )
  })?;
  Ok(())
}

fn add_zip_directory(
  zip: &mut ZipWriter<File>,
  source: &Path,
  archive_name: &str,
) -> Result<()> {
  let directory_options = SimpleFileOptions::default()
    .compression_method(CompressionMethod::Deflated)
    .unix_permissions(0o700);
  zip
    .add_directory(format!("{archive_name}/"), directory_options)
    .with_context(|| format!("failed to archive directory {}", source.display()))?;

  let mut entries = fs::read_dir(source)
    .with_context(|| format!("failed to read {}", source.display()))?
    .collect::<std::result::Result<Vec<_>, _>>()?;
  entries.sort_by_key(|entry| entry.file_name());
  for entry in entries {
    let path = entry.path();
    let metadata = fs::symlink_metadata(&path)?;
    if metadata.file_type().is_symlink() {
      bail!(
        "factory reset backup cannot include the symbolic link {}",
        path.display()
      );
    }
    let name = entry
      .file_name()
      .into_string()
      .map_err(|_| anyhow::anyhow!("backup path is not valid UTF-8: {}", path.display()))?;
    let child_archive_name = format!("{archive_name}/{name}");
    if metadata.is_dir() {
      add_zip_directory(zip, &path, &child_archive_name)?;
    } else if metadata.is_file() {
      let file_options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o600);
      zip
        .start_file(&child_archive_name, file_options)
        .with_context(|| format!("failed to archive file {}", path.display()))?;
      let mut input = File::open(&path)?;
      io::copy(&mut input, zip)?;
    } else {
      bail!(
        "factory reset backup cannot include the unsupported file {}",
        path.display()
      );
    }
  }
  Ok(())
}

pub fn validate_factory_reset_target(
  data_dir: &Path,
  database: &Path,
) -> Result<PathBuf> {
  if !data_dir.exists() || !database.is_file() {
    bail!(
      "Factory reset is only available on the Dopbase server host. Run it on the host with the server stopped."
    );
  }
  let target = data_dir
    .canonicalize()
    .with_context(|| format!("failed to resolve data directory {}", data_dir.display()))?;
  let database = database
    .canonicalize()
    .with_context(|| format!("failed to resolve database {}", database.display()))?;
  if !database.starts_with(&target) {
    bail!("refusing to reset a data directory that does not contain its database");
  }
  if target.parent().is_none() {
    bail!("refusing to use the filesystem root as the Dopbase data directory");
  }
  if std::env::home_dir().is_some_and(|home| target == home) {
    bail!("refusing to use the home directory as the Dopbase data directory");
  }
  let current = std::env::current_dir()?.canonicalize()?;
  if current.starts_with(&target) {
    bail!("leave the Dopbase data directory before running factory reset");
  }
  Ok(target)
}

pub fn factory_reset_quarantine_path(data_dir: &Path) -> Result<PathBuf> {
  let name = data_dir
    .file_name()
    .and_then(|name| name.to_str())
    .context("Dopbase data directory does not have a valid name")?;
  let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
  let path = data_dir.with_file_name(format!("{name}.factory-reset-{timestamp}"));
  if path.exists() {
    bail!(
      "factory reset quarantine path already exists: {}",
      path.display()
    );
  }
  Ok(path)
}

async fn reset_password(
  email: String,
  data_dir: Option<PathBuf>,
  config: Option<PathBuf>,
  master_key_file: Option<PathBuf>,
) -> Result<()> {
  if !io::stdin().is_terminal() {
    bail!("password reset requires an interactive terminal");
  }
  let config = ServerConfig::load(&ServerOverrides {
    data_dir,
    config_path: config,
    master_key_path: master_key_file,
    ..Default::default()
  })?;
  ensure_data_dir(&config.data_dir)?;
  let _lock = crate::server::InstanceLock::acquire(&config.database_url)?
    .context("offline recovery requires a file-backed SQLite database")?;
  let db = crate::services::db::DbClient::connect(&config.database_url).await?;
  db.migrate().await?;
  let _crypto =
    crate::services::crypto::CryptoService::initialize(db.pool(), &config.master_key.path).await?;
  let admin: Option<(String, String)> =
    sqlx::query_as("SELECT id,email FROM admins WHERE email=? COLLATE NOCASE")
      .bind(email.trim())
      .fetch_optional(db.pool())
      .await?;
  let (admin_id, normalized) = admin.context("no administrator exists with that email")?;
  let password = prompt::new_password("New password:", "Confirm new password:")?;
  let hash = crate::modules::common::hash_password(&password)
    .map_err(|_| anyhow::anyhow!("password hashing failed"))?;
  let now = chrono::Utc::now().to_rfc3339();
  let mut tx = db.pool().begin().await?;
  sqlx::query("UPDATE admins SET password_hash=?,updated_at=? WHERE id=?")
    .bind(hash)
    .bind(&now)
    .bind(&admin_id)
    .execute(&mut *tx)
    .await?;
  sqlx::query("UPDATE sessions SET revoked_at=? WHERE revoked_at IS NULL")
    .bind(&now)
    .execute(&mut *tx)
    .await?;
  crate::modules::common::audit(
    &mut *tx,
    "system",
    None,
    Some("offline-recovery"),
    "admin.password_reset",
    None,
    None,
    Some("admin"),
    Some(&admin_id),
    json!({"email":normalized}),
  )
  .await?;
  tx.commit().await?;
  db.checkpoint().await?;
  db.close().await;
  output::print_success("Password reset complete.");
  output::print_text("All human sessions were revoked.");
  Ok(())
}
