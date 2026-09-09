use super::{output, prompt};
use crate::{
  cli::args::AdminCommand,
  config::{ServerConfig, ServerOverrides, database_path, ensure_data_dir},
};
use anyhow::{Context, Result, bail};
use serde_json::json;
use std::{
  io::{self, IsTerminal},
  path::{Path, PathBuf},
};

pub(super) async fn execute(
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
    AdminCommand::FactoryReset { config } => {
      factory_reset_offline(data_dir, config, json_output).await?
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
    "Every database, backup, log, configuration, master-key, and CLI-state file inside this directory will move with it."
  );
  eprintln!("The next server start will create a fresh installation.");
  eprintln!("Keep an external backup of anything you need before continuing.");
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
  let quarantine = factory_reset_quarantine_path(&reset_target)?;
  std::fs::rename(&reset_target, &quarantine).with_context(|| {
    format!(
      "failed to move Dopbase data directory {} to {}",
      reset_target.display(),
      quarantine.display()
    )
  })?;
  output::print_success("Factory reset complete.");
  output::print_fields(&[("Previous data:", quarantine.display().to_string())]);
  output::print_text("Start Dopbase to create a fresh installation and complete first-run setup.");
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
