use clap::Subcommand;

use crate::constants::help::ENVIRONMENT_ARG_HELP;

#[derive(Subcommand, Debug)]
pub enum SecretCommand {
  /// List secret keys in an environment (values are never shown).
  #[command(after_help = LIST_HELP)]
  List {
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
    environment: String,
  },
  /// Set a secret through a masked prompt or read it from standard input.
  #[command(after_help = SET_HELP)]
  Set {
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
    environment: String,
    /// Secret key name.
    key: String,
    /// Read until EOF. In a terminal, finish with Ctrl+D (Ctrl+Z then Enter on Windows).
    #[arg(long)]
    stdin: bool,
  },
  /// Show secret metadata, or print its value with --reveal.
  #[command(after_help = GET_HELP)]
  Get {
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
    environment: String,
    /// Secret key name.
    key: String,
    /// Print the actual value after interactive password confirmation.
    #[arg(long)]
    reveal: bool,
  },
  /// Delete a secret from an environment.
  ///
  /// Asks for confirmation unless --yes is passed.
  #[command(after_help = DELETE_HELP)]
  Delete {
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
    environment: String,
    /// Secret key name.
    key: String,
    /// Skip the confirmation prompt (for automation).
    #[arg(long)]
    yes: bool,
  },
}

pub(crate) const HELP: &str = "Examples:\n  dopbase secret list payment-service/production\n  dopbase secret set payment-service/production API_KEY\n  dopbase secret get payment-service/production API_KEY\n  dopbase secret get payment-service/production API_KEY --reveal\n  dopbase secret delete payment-service/production API_KEY\n\nUse PROJECT_REF/ENVIRONMENT_NAME for readable references. PROJECT_REF can be a\nproject ID or name. Immutable environment IDs such as env_482731 also work.\n";
const LIST_HELP: &str = "Examples:\n  dopbase secret list payment-service/production\n  dopbase secret list env_482731\n\nRun `dopbase env list` to find an environment.\n";
const SET_HELP: &str = "Examples:\n  dopbase secret set payment-service/production API_KEY\n  dopbase secret set payment-service/production API_KEY --stdin\n  printf '%s' \"$API_KEY\" | dopbase secret set payment-service/production API_KEY --stdin\n\nWithout --stdin, Dopbase uses a masked prompt and displays * for each character.\n\nWith --stdin, Dopbase reads the value until EOF. In a terminal, paste or type\nthe value, then press Ctrl+D. On Windows, press Ctrl+Z, then Enter. Piped input\nis read exactly as supplied.\n";
const GET_HELP: &str = "Examples:\n  dopbase secret get payment-service/production API_KEY\n  dopbase secret get payment-service/production API_KEY --reveal\n";
const DELETE_HELP: &str = "Examples:\n  dopbase secret delete payment-service/production API_KEY\n  dopbase secret delete payment-service/production API_KEY --yes\n";
