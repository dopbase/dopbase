//! Feature-owned CLI command modules.
//!
//! Each command keeps its Clap arguments, help text, handler, and private
//! support code together. This file only routes parsed commands and re-exports
//! the small set of helpers used outside their owning module.

pub mod admin;
pub mod auth;
pub mod backup;
pub mod client;
pub mod environment;
pub mod export;
pub mod import;
pub mod init;
pub mod project;
pub mod restore;
pub mod run;
pub mod secret;
pub mod server;
pub mod token;
pub mod update;

#[doc(hidden)]
pub use super::output::{render_fields, render_table};
#[doc(hidden)]
pub use super::prompt::remove_one_line_ending;
pub use admin::{
  complete_factory_reset, factory_reset_archive_path, factory_reset_confirmation_matches,
  factory_reset_quarantine_path, validate_factory_reset_target,
};
pub use client::{insecure_transport_warning, server_switch_confirmed, status_document};
pub use run::{RunEnvironment, run_environment};

use super::{args::*, local_config};
use anyhow::{Result, bail};

pub async fn execute(cli: Cli) -> Result<i32> {
  let server_argument = cli.server.clone();
  let data_dir = cli.data_dir.clone();
  let json_output = cli.json;
  match cli.command {
    Command::Server { command } => {
      if server_argument.is_some() {
        bail!("--server cannot be used with local `dopbase server` commands");
      }
      server::execute(command, data_dir, json_output).await
    }
    Command::Client {
      command: ClientCommand::Connect { server_url },
    } => {
      if server_argument.is_some() {
        bail!(
          "--server cannot be used with `dopbase client connect`. Pass the destination as the positional server URL"
        );
      }
      client::connect(&server_url, data_dir.as_deref(), json_output).await?;
      Ok(0)
    }
    Command::Client {
      command: ClientCommand::Status,
    }
    | Command::Status => {
      client::show_status(server_argument.as_deref(), data_dir.as_deref(), json_output).await?;
      Ok(0)
    }
    Command::Login(args) => {
      let server = local_config::resolve(server_argument.as_deref(), data_dir.as_deref())?;
      auth::login(&server, args, json_output).await
    }
    Command::Logout => {
      let server = local_config::resolve(server_argument.as_deref(), data_dir.as_deref())?;
      auth::logout(&server, json_output)
    }
    Command::Admin { command } => {
      if server_argument.is_some() {
        bail!("--server cannot be used with local `dopbase admin` commands");
      }
      admin::execute(command, data_dir, json_output).await
    }
    Command::Update => update::run(json_output).await,
    command => {
      let server = local_config::resolve(server_argument.as_deref(), data_dir.as_deref())?;
      execute_client(command, &server, json_output).await
    }
  }
}

async fn execute_client(
  command: Command,
  server: &local_config::ResolvedServer,
  json_output: bool,
) -> Result<i32> {
  match command {
    Command::Init(args) => init::execute(server, args, json_output).await,
    Command::Project { command } => project::execute(command, server, json_output).await,
    Command::Env { command } => environment::execute(command, server, json_output).await,
    Command::Secret { command } => secret::execute(command, server, json_output).await,
    Command::Import(args) => import::execute(server, args, json_output).await,
    Command::Export(args) => export::execute(server, args, json_output).await,
    Command::Token { command } => token::execute(command, server, json_output).await,
    Command::Run(args) => run::execute(server, args).await,
    Command::Backup(args) => backup::execute(server, args, json_output).await,
    Command::Restore(args) => restore::execute(server, args, json_output).await,
    _ => bail!("unsupported command"),
  }
}
