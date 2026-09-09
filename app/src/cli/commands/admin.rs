use crate::{
  cli::args::AdminCommand,
  config::{ServerConfig, ServerOverrides, database_path, ensure_data_dir},
};
use anyhow::{Context, Result, bail};
use serde_json::json;
use std::{
  io::{self, IsTerminal, Write},
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

