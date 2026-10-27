use crate::cli::args::{Cli, Command};
use anyhow::{Context, Result, bail};
use std::{
  io::IsTerminal,
  process::{Command as ProcessCommand, Stdio},
};

/// Docker container name for `--container` and `dopbase docker exec`.
pub fn container_name() -> String {
  std::env::var("DOPBASE_CONTAINER").unwrap_or_else(|_| "dopbase".to_string())
}

pub fn execute_in_container(cli: Cli) -> Result<i32> {
  let mut arguments = vec!["exec".to_string()];
  if std::io::stdin().is_terminal() {
    arguments.push("-it".to_string());
  }
  arguments.push(container_name());
  arguments.push("dopbase".to_string());
  if let Some(data_dir) = &cli.data_dir {
    arguments.push("--data-dir".to_string());
    arguments.push(data_dir.display().to_string());
  }
  if cli.json {
    arguments.push("--json".to_string());
  }
  if let Some(server) = &cli.server {
    arguments.push("--server".to_string());
    arguments.push(server.clone());
  }
  push_command(&mut arguments, cli.command)?;
  let status = ProcessCommand::new("docker")
    .args(&arguments)
    .stdin(Stdio::inherit())
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit())
    .status()
    .with_context(|| format!("failed to run `docker {}`", arguments.join(" ")))?;
  Ok(status.code().unwrap_or(1))
}

fn push_command(
  arguments: &mut Vec<String>,
  command: Command,
) -> Result<()> {
  match command {
    Command::Admin { command } => {
      arguments.push("admin".to_string());
      match command {
        crate::cli::args::AdminCommand::ResetPassword {
          email,
          config,
          master_key_file,
        } => {
          arguments.push("reset-password".to_string());
          arguments.push(email);
          if let Some(path) = config {
            arguments.push("--config".to_string());
            arguments.push(path.display().to_string());
          }
          if let Some(path) = master_key_file {
            arguments.push("--master-key-file".to_string());
            arguments.push(path.display().to_string());
          }
        }
        crate::cli::args::AdminCommand::FactoryReset { config, no_backup } => {
          arguments.push("factory-reset".to_string());
          if let Some(path) = config {
            arguments.push("--config".to_string());
            arguments.push(path.display().to_string());
          }
          if no_backup {
            arguments.push("--no-backup".to_string());
          }
        }
      }
    }
    Command::Server { command } => {
      arguments.push("server".to_string());
      push_server_command(arguments, command)?;
    }
    _ => bail!("`--container` only supports local `admin` and `server` commands"),
  }
  Ok(())
}

fn push_server_command(
  arguments: &mut Vec<String>,
  command: crate::cli::args::ServerCommand,
) -> Result<()> {
  use crate::cli::args::ServerCommand;
  match command {
    ServerCommand::Setup(_)
    | ServerCommand::Start(_)
    | ServerCommand::Stop { .. }
    | ServerCommand::Restart { .. }
    | ServerCommand::Up(_)
    | ServerCommand::Down { .. } => {
      bail!(
        "Docker controls the Dopbase server process. Use `docker compose up`, `stop`, or `restart`. `dopbase --container` can run `server status`, `server logs`, and `admin` commands."
      );
    }
    ServerCommand::Status => arguments.push("status".to_string()),
    ServerCommand::Logs {
      lines,
      clean,
      watch,
    } => {
      arguments.push("logs".to_string());
      arguments.push("--lines".to_string());
      arguments.push(lines.to_string());
      if clean {
        arguments.push("--clean".to_string());
      }
      if watch {
        arguments.push("--watch".to_string());
      }
    }
  }
  Ok(())
}
