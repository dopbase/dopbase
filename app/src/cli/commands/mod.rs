mod admin;
mod auth;
mod backup;
mod client;
mod environment;
mod export;
mod import;
mod init;
mod output;
mod project;
pub(crate) mod prompt;
mod restore;
mod run;
mod secret;
mod server;
mod token;

pub use admin::{
  complete_factory_reset, factory_reset_archive_path, factory_reset_confirmation_matches,
  factory_reset_quarantine_path, validate_factory_reset_target,
};
pub use client::{insecure_transport_warning, server_switch_confirmed, status_document};
#[doc(hidden)]
pub use output::{render_fields, render_table};
#[doc(hidden)]
pub use prompt::remove_one_line_ending;
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
    Command::Login { token } => {
      let server = local_config::resolve(server_argument.as_deref(), data_dir.as_deref())?;
      auth::login(&server, token, json_output).await
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
    Command::Update => super::update::run(json_output).await,
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
    Command::Init {
      project,
      environment,
      from,
    } => init::execute(server, project, environment, &from, json_output).await,
    Command::Project { command } => project::execute(command, server, json_output).await,
    Command::Env { command } => environment::execute(command, server, json_output).await,
    Command::Secret { command } => secret::execute(command, server, json_output).await,
    Command::Import {
      environment,
      path,
      dry_run,
      replace,
      yes,
    } => {
      import::execute(
        server,
        &environment,
        &path,
        dry_run,
        replace,
        yes,
        json_output,
      )
      .await
    }
    Command::Export {
      environment,
      output,
      stdout,
      force,
    } => export::execute(server, &environment, output, stdout, force, json_output).await,
    Command::Token { command } => token::execute(command, server, json_output).await,
    Command::Run {
      environment,
      token,
      command,
    } => run::execute(server, environment, token, command).await,
    Command::Backup { name, output } => backup::execute(server, name, output, json_output).await,
    Command::Restore {
      path,
      key,
      setup_token,
      yes,
    } => restore::execute(server, path, key, setup_token, yes, json_output).await,
    _ => bail!("unsupported command"),
  }
}
