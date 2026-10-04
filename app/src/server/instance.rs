use crate::config::{ServerConfig, database_path};
use anyhow::{Context, Result, bail};
use sqlx::{ConnectOptions, Connection, sqlite::SqliteConnectOptions};
use std::{fmt, path::PathBuf, str::FromStr};

#[derive(Debug)]
pub struct InitializationRequired {
  pub data_dir: PathBuf,
}

impl fmt::Display for InitializationRequired {
  fn fmt(
    &self,
    formatter: &mut fmt::Formatter<'_>,
  ) -> fmt::Result {
    write!(
      formatter,
      "This instance has not been initialized.\nRun `dopbase server setup` first using the same --data-dir and --config options.\nData: {}",
      self.data_dir.display()
    )
  }
}

impl std::error::Error for InitializationRequired {}

pub async fn require_initialized(config: &ServerConfig) -> Result<()> {
  if initialization_required(config).await? {
    return Err(
      InitializationRequired {
        data_dir: config.data_dir.clone(),
      }
      .into(),
    );
  }
  Ok(())
}

pub(crate) async fn require_uninitialized(config: &ServerConfig) -> Result<()> {
  if !initialization_required(config).await? {
    bail!(
      "This instance has already been initialized. Use `dopbase admin reset-password` for password recovery.\nData: {}",
      config.data_dir.display()
    );
  }
  Ok(())
}

async fn initialization_required(config: &ServerConfig) -> Result<bool> {
  if config
    .data_dir
    .join(".factory-reset.pending")
    .try_exists()?
  {
    return Ok(true);
  }
  let database = database_path(&config.database_url)?;
  if !database.try_exists()? {
    return Ok(true);
  }
  // Inspect existing storage without creating files or applying migrations.
  let options = SqliteConnectOptions::from_str(&config.database_url)?
    .read_only(true)
    .create_if_missing(false);
  let mut connection = options
    .connect()
    .await
    .context("failed to inspect SQLite initialization")?;
  let result = async {
    let has_admins: bool = sqlx::query_scalar(
      "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='admins')",
    )
    .fetch_one(&mut connection)
    .await?;
    if !has_admins {
      return Ok::<bool, sqlx::Error>(true);
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admins")
      .fetch_one(&mut connection)
      .await?;
    Ok(count == 0)
  }
  .await;
  connection
    .close()
    .await
    .context("failed to close SQLite initialization inspection")?;
  result.context("failed to inspect SQLite initialization")
}
