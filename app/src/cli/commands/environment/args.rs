use clap::Subcommand;

use crate::cli::{environment_target, environment_target::EnvironmentTarget};
use crate::constants::help::ENVIRONMENT_ARG_HELP;

#[derive(Subcommand, Debug)]
pub enum EnvCommand {
  /// Set or clear the default environment used by `dopbase run`.
  #[command(after_help = DEFAULT_HELP)]
  Default {
    #[arg(value_name = "ENVIRONMENT_REF", required_unless_present = "clear", help = ENVIRONMENT_ARG_HELP)]
    environment: Option<String>,
    /// Clear the default environment for the active server.
    #[arg(long, conflicts_with = "environment")]
    clear: bool,
  },
  /// Create an environment inside a project.
  #[command(after_help = CREATE_HELP)]
  Create {
    /// Existing project and new environment, written as PROJECT_REF/ENVIRONMENT_NAME.
    #[arg(value_name = "PROJECT_REF/ENVIRONMENT_NAME", value_parser = environment_target::parse_create)]
    target: EnvironmentTarget,
  },
  /// List environments, either for one project or all accessible ones.
  #[command(after_help = LIST_HELP)]
  List {
    /// Limit the listing to this project (ID or name).
    #[arg(value_name = "PROJECT_REF")]
    project: Option<String>,
  },
  /// Show environment metadata.
  #[command(after_help = SHOW_HELP)]
  Show {
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
    environment: String,
  },
  /// Rename an environment.
  #[command(after_help = RENAME_HELP)]
  Rename {
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
    environment: String,
    /// New environment name.
    #[arg(value_name = "NEW_ENVIRONMENT_NAME")]
    new_name: String,
  },
  /// Delete an environment with its secrets and tokens.
  ///
  /// Asks for confirmation unless --yes is passed.
  #[command(after_help = DELETE_HELP)]
  Delete {
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
    environment: String,
    /// Skip the confirmation prompt (for automation).
    #[arg(long)]
    yes: bool,
  },
}

pub(crate) const HELP: &str = "Examples:\n  dopbase env create payment-service/production\n  dopbase env list payment-service\n  dopbase env show payment-service/production\n  dopbase env default payment-service/development\n  dopbase env rename payment-service/production prod\n  dopbase env delete payment-service/staging\n";
const DEFAULT_HELP: &str =
  "Examples:\n  dopbase env default payment-service/development\n  dopbase env default --clear\n";
const CREATE_HELP: &str = "Examples:\n  dopbase env create payment-service/production\n";
const LIST_HELP: &str = "Examples:\n  dopbase env list\n  dopbase env list payment-service\n";
const SHOW_HELP: &str =
  "Examples:\n  dopbase env show payment-service/production\n  dopbase env show env_482731\n";
const RENAME_HELP: &str = "Examples:\n  dopbase env rename payment-service/production prod\n";
const DELETE_HELP: &str = "Examples:\n  dopbase env delete payment-service/staging\n  dopbase env delete payment-service/staging --yes\n";
