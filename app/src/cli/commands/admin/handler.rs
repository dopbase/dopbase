use super::{AdminCommand, target};
use crate::cli::{output, prompt};
use crate::config::{ServerConfig, ServerOverrides, database_path};
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
    AdminCommand::Passcode { email, ttl, config } => {
      issue_passcode(email, ttl, data_dir, config, json_output).await?
    }
    AdminCommand::FactoryReset { config, no_backup } => {
      factory_reset_offline(data_dir, config, no_backup, json_output).await?
    }
  }
  Ok(0)
}

struct IssuedPasscode {
  email: String,
  passcode: String,
  expires_at: chrono::DateTime<chrono::Utc>,
}

/// Mint a launch passcode for a running server. The CLI never talks to the
/// server: it writes the hashed passcode into the server's database, so only
/// someone who can write the data directory can sign anyone in this way.
async fn issue_passcode(
  email: String,
  ttl: i64,
  data_dir: Option<PathBuf>,
  config: Option<PathBuf>,
  json_output: bool,
) -> Result<()> {
  let config = ServerConfig::load(&ServerOverrides {
    data_dir,
    config_path: config,
    ..Default::default()
  })?;
  if !target::connected_here(&config)? {
    bail!("please connect to local instance to perform passcode action");
  }
  if !config.web_ui_enabled {
    bail!(
      "the Admin UI is turned off on this server (web_ui = false), so there is nothing to sign in to"
    );
  }
  database_path(&config.database_url)?;
  // The server holds the instance lock while it runs. A passcode only
  // lives for seconds, so minting one while nothing can read it is
  // pointless. This command never takes the lock itself.
  if !crate::server::InstanceLock::is_held(&config.database_url)? {
    bail!(
      "the Dopbase server isn't running for the instance in {}. Start it, then run this command again",
      config.data_dir.display()
    );
  }
  let db = crate::services::db::DbClient::connect(&config.database_url).await?;
  let issued = mint_launch_passcode(&db, &config, email.trim(), ttl).await;
  db.close().await;
  let issued = issued?;

  let url = format!(
    "{}{}#identity={}&passcode={}",
    config.public_url.trim_end_matches('/'),
    crate::constants::api::auth::LAUNCH_PAGE,
    url::form_urlencoded::byte_serialize(issued.email.as_bytes()).collect::<String>(),
    issued.passcode,
  );
  if json_output {
    output::print_value(
      true,
      &json!({
        "email": issued.email,
        "passcode": issued.passcode,
        "expiresAt": issued.expires_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "ttlSeconds": ttl,
        "url": url,
        "dataDir": config.data_dir,
      }),
    );
    return Ok(());
  }
  output::print_success(&format!("Launch passcode for {}", issued.email));
  output::print_text("");
  output::print_fields(&[
    ("Instance:", config.data_dir.display().to_string()),
    ("Passcode:", issued.passcode),
    (
      "Expires:",
      format!(
        "in {ttl} seconds ({} UTC)",
        issued.expires_at.format("%H:%M:%S")
      ),
    ),
    ("Open:", url),
  ]);
  output::print_text("");
  output::print_text("It works once. Running this command again replaces it.");
  Ok(())
}

async fn mint_launch_passcode(
  db: &crate::services::db::DbClient,
  config: &ServerConfig,
  email: &str,
  ttl: i64,
) -> Result<IssuedPasscode> {
  use crate::services::{
    authx::{self, SignInRefusal},
    passcodes::{self, PasscodePurpose},
  };

  check_schema(db).await?;
  let has_table: Option<i64> =
    sqlx::query_scalar("SELECT 1 FROM sqlite_master WHERE type='table' AND name='passcodes'")
      .fetch_optional(db.pool())
      .await?;
  if has_table.is_none() {
    bail!(
      "the running server is older than this command. Restart the server on this version first"
    );
  }
  let account: Option<(String, String)> =
    sqlx::query_as("SELECT id,email FROM admins WHERE email=? COLLATE NOCASE")
      .bind(email)
      .fetch_optional(db.pool())
      .await?;
  let (admin_id, email) = account.context("no account exists with that email")?;
  let mut connection = db.pool().acquire().await?;
  let access = authx::load_access(&mut connection, &admin_id, &config.features).await?;
  drop(connection);
  let refusal = authx::sign_in_refusal(access.as_ref(), || async {
    Ok::<_, anyhow::Error>(
      crate::modules::sso::service::policy_for(db.pool(), &config.features)
        .await?
        .require_sso,
    )
  })
  .await?;
  match refusal {
    None => {}
    Some(SignInRefusal::Disabled) => bail!("{email} is turned off, so it can't sign in"),
    Some(SignInRefusal::TokenOnly) => {
      bail!("{email} signs in with tokens only, so it can't use a passcode")
    }
    Some(SignInRefusal::SsoRequired) => {
      bail!("single sign-on is required on this instance, so only root can sign in with a passcode")
    }
  }

  let code = crate::utils::passcode::generate()
    .map_err(|_| anyhow::anyhow!("could not generate a passcode"))?;
  let hostname = crate::cli::client::local_hostname();
  let now = chrono::Utc::now();
  let mut tx = db.pool().begin().await?;
  let expires_at = passcodes::issue(
    &mut tx,
    PasscodePurpose::Launch,
    &admin_id,
    &code,
    chrono::Duration::seconds(ttl),
    hostname.as_deref(),
    now,
  )
  .await?;
  crate::modules::common::audit(
    &mut tx,
    "system",
    None,
    Some("server-cli"),
    "passcode.issued",
    None,
    None,
    Some("admin"),
    Some(&admin_id),
    json!({"purpose":PasscodePurpose::Launch.as_str(),"email":email,"ttlSeconds":ttl,"host":hostname}),
  )
  .await?;
  tx.commit().await?;
  Ok(IssuedPasscode {
    email,
    passcode: crate::utils::passcode::display(&code),
    expires_at,
  })
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
  if !target::connected_here(&config)? {
    bail!("please connect to local instance to perform factory-reset action");
  }
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
    sqlx::query_as("SELECT email,password_hash FROM admins WHERE is_root=1")
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
  eprintln!("Run `dopbase server setup` before starting the fresh instance.");
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
  output::print_text(
    "Run `dopbase server setup`, then `dopbase server start`, using the same instance options.",
  );
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

/// Set a new password straight in the instance's database. It works with
/// the server running or stopped. It never creates an instance, a master
/// key or runs migrations, so pointing it at the wrong directory changes
/// nothing.
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
  if !target::connected_here(&config)? {
    bail!("please connect to local instance to perform reset-password action");
  }
  let database = database_path(&config.database_url)?;
  if !database.is_file() {
    bail!(
      "there's no Dopbase instance in {}. Pass --data-dir for the right one, or run this command on the machine or inside the container that hosts the server",
      config.data_dir.display()
    );
  }
  eprintln!("Instance: {}", config.data_dir.display());

  let db = crate::services::db::DbClient::connect(&config.database_url).await?;
  let reset = reset_password_in(&db, &config, email.trim()).await;
  db.close().await;
  let running = reset?;

  output::print_success(&format!(
    "Password reset on the instance in {}.",
    config.data_dir.display()
  ));
  output::print_text("All sessions on this instance were revoked.");
  if running {
    output::print_text(
      "If too many wrong passwords paused sign-in for this account, that pause still runs out on its own.",
    );
  }
  Ok(())
}

/// Returns whether the server was running when the password was written.
async fn reset_password_in(
  db: &crate::services::db::DbClient,
  config: &ServerConfig,
  email: &str,
) -> Result<bool> {
  check_schema(db).await?;
  crate::services::crypto::CryptoService::open_existing(db.pool(), &config.master_key.path).await?;
  let admin: Option<(String, String)> =
    sqlx::query_as("SELECT id,email FROM admins WHERE email=? COLLATE NOCASE")
      .bind(email)
      .fetch_optional(db.pool())
      .await?;
  let (admin_id, normalized) = admin.context("no administrator exists with that email")?;
  let password = prompt::new_password(
    "New password:",
    "Confirm new password:",
    crate::cli::CliCancelled::PasswordConfirmation,
  )?;
  let hash = crate::modules::common::hash_password(&password)
    .map_err(|_| anyhow::anyhow!("password hashing failed"))?;

  // A stopped server could start while this writes and run its migrations
  // underneath it, so hold the lock for the write. A running server has
  // already migrated, so write alongside it. Check the schema again in
  // case a newer server started while the prompt was open.
  let lock = crate::server::InstanceLock::acquire_if_free(&config.database_url)?;
  let running = lock.is_none();
  check_schema(db).await?;
  let now = chrono::Utc::now().to_rfc3339();
  let mut tx = db.pool().begin().await?;
  let updated = sqlx::query("UPDATE admins SET password_hash=?,updated_at=? WHERE id=?")
    .bind(hash)
    .bind(&now)
    .bind(&admin_id)
    .execute(&mut *tx)
    .await?;
  if updated.rows_affected() != 1 {
    bail!("{normalized} was removed while the password was being typed");
  }
  sqlx::query("UPDATE sessions SET revoked_at=? WHERE revoked_at IS NULL")
    .bind(&now)
    .execute(&mut *tx)
    .await?;
  crate::modules::common::audit(
    &mut tx,
    "system",
    None,
    Some("offline-recovery"),
    "admin.password_reset",
    None,
    None,
    Some("admin"),
    Some(&admin_id),
    json!({"email":normalized,"serverRunning":running}),
  )
  .await?;
  tx.commit().await?;
  if !running {
    // Nothing else has the database open, so fold the WAL back in now.
    db.checkpoint().await?;
  }
  drop(lock);
  Ok(running)
}

/// Refuse a database this binary cannot safely write: one that isn't a
/// Dopbase database, one a newer version upgraded, or one too old to have
/// the hash-chained audit log the reset writes to.
async fn check_schema(db: &crate::services::db::DbClient) -> Result<()> {
  let applied: Vec<(i64, bool)> =
    match sqlx::query_as("SELECT version,success FROM _sqlx_migrations")
      .fetch_all(db.pool())
      .await
    {
      Ok(applied) => applied,
      Err(_) => bail!("this database was never set up by Dopbase"),
    };
  if applied.iter().any(|(_, success)| !success) {
    bail!("a database upgrade failed on this instance. Start the server to see the error");
  }
  let newest = crate::services::db::MIGRATOR
    .iter()
    .map(|migration| migration.version)
    .max()
    .unwrap_or_default();
  if applied.iter().any(|(version, _)| *version > newest) {
    bail!("a newer version of Dopbase upgraded this database. Run this command with that version");
  }
  let chained: Option<i64> =
    sqlx::query_scalar("SELECT 1 FROM pragma_table_info('audit_events') WHERE name='record_hash'")
      .fetch_optional(db.pool())
      .await?;
  if chained.is_none() {
    bail!(
      "this database is older than this command. Start the server on this version once, then run it again"
    );
  }
  Ok(())
}
