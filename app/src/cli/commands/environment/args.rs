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
  /// Clone an environment's secrets into a new environment in the same project.
  ///
  /// Asks for confirmation unless --yes is passed.
  #[command(after_help = CLONE_HELP)]
  Clone {
    /// Existing source, written as PROJECT_REF/SOURCE_ENVIRONMENT_NAME.
    #[arg(
      value_name = "PROJECT_REF/SOURCE_ENVIRONMENT_NAME",
      value_parser = environment_target::parse_create
    )]
    source: EnvironmentTarget,
    /// Name for the new environment.
    #[arg(
      value_name = "NEW_ENVIRONMENT_NAME",
      value_parser = environment_target::parse_name
    )]
    new_name: String,
    /// Skip the clone confirmation. Recent human authentication is still required.
    #[arg(long)]
    yes: bool,
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

pub(crate) const HELP: &str = "\
Examples:
  dopbase env create payment-service/production
  dopbase env clone payment-service/local production
  dopbase env list payment-service
  dopbase env show payment-service/production
  dopbase env default payment-service/development
  dopbase env rename payment-service/production prod
  dopbase env delete payment-service/staging
";
const DEFAULT_HELP: &str = "\
Examples:
  dopbase env default payment-service/development
  dopbase env default --clear
";
const CREATE_HELP: &str = "\
Examples:
  dopbase env create payment-service/production
";
const CLONE_HELP: &str = "\
Examples:
  dopbase env clone payment-service/local production
  dopbase env clone storefront/staging preview-42 --yes
";
const LIST_HELP: &str = "\
Examples:
  dopbase env list
  dopbase env list payment-service
";
const SHOW_HELP: &str = "\
Examples:
  dopbase env show payment-service/production
  dopbase env show env_482731
";
const RENAME_HELP: &str = "\
Examples:
  dopbase env rename payment-service/production prod
";
const DELETE_HELP: &str = "\
Examples:
  dopbase env delete payment-service/staging
  dopbase env delete payment-service/staging --yes
";
