use clap::{Args, Subcommand};

use crate::constants::help::ENVIRONMENT_ARG_HELP;

pub(crate) const HELP: &str = "\
Examples:
  dopbase docker env-file payment-service/production -o staging.env
  dopbase docker run payment-service/production -- docker run --rm my-image
  dopbase docker compose payment-service/production -- docker compose up
  dopbase docker exec payment-service/production -- admin reset-password admin@example.com
";

const ENV_FILE_HELP: &str = "\
Examples:
  dopbase docker env-file payment-service/production -o staging.env
  dopbase docker env-file payment-service/production
";
const RUN_HELP: &str = "\
Examples:
  dopbase docker run payment-service/production -- docker run --rm my-image
";
const COMPOSE_HELP: &str = "\
Examples:
  dopbase docker compose payment-service/production -- docker compose up
";
const EXEC_HELP: &str = "\
Examples:
  dopbase docker exec workspace1/api/staging -- admin reset-password admin@example.com
";

#[derive(Subcommand, Debug)]
pub enum DockerCommand {
  /// Write a Docker-compatible env file for an environment.
  #[command(after_help = ENV_FILE_HELP)]
  EnvFile {
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
    environment: String,
    /// Write to this file instead of standard output.
    #[arg(short = 'o', long, value_name = "FILE")]
    output: Option<std::path::PathBuf>,
  },
  /// Run `docker run` with secrets injected through a temporary --env-file.
  #[command(after_help = RUN_HELP)]
  Run {
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
    environment: String,
    #[arg(
      last = true,
      required = true,
      help = "Full Docker command after `--`, starting with `docker run`"
    )]
    command: Vec<String>,
  },
  /// Run `docker compose` with secrets injected through --env-file.
  #[command(after_help = COMPOSE_HELP)]
  Compose {
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
    environment: String,
    #[arg(
      last = true,
      required = true,
      help = "Full Docker command after `--`, starting with `docker compose`"
    )]
    command: Vec<String>,
  },
  /// Run a command in the Dopbase container with secrets injected through --env-file.
  #[command(after_help = EXEC_HELP)]
  Exec {
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
    environment: String,
    #[arg(
      last = true,
      required = true,
      help = "Command inside the container after `--` (use `admin` or `server` without a leading `dopbase`)"
    )]
    command: Vec<String>,
  },
}

#[derive(Args, Debug)]
pub struct DockerArgs {
  #[command(subcommand)]
  pub command: DockerCommand,
}
