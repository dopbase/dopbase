use clap::Subcommand;

use crate::constants::help::ENVIRONMENT_ARG_HELP;

#[derive(Subcommand, Debug)]
pub enum TokenCommand {
  /// Create an access token for an environment (e.g. for CI/CD).
  ///
  /// The token value is shown once at creation. Pass it to client commands
  /// via the DOPBASE_TOKEN environment variable.
  #[command(after_help = CREATE_HELP)]
  Create {
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
    environment: String,
    /// Display name for the token.
    #[arg(long)]
    name: String,
    /// Token role.
    #[arg(long, default_value = "runner")]
    role: String,
  },
  /// List tokens for an environment.
  #[command(after_help = LIST_HELP)]
  List {
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
    environment: String,
  },
  /// Revoke a token by ID.
  #[command(after_help = REVOKE_HELP)]
  Revoke {
    /// ID of the token to revoke.
    token_id: String,
  },
}

pub(crate) const HELP: &str = "\
Examples:
  dopbase token create payment-service/production --name deploy
  dopbase token list payment-service/production
  dopbase token revoke tok_01ABCDEF
";
const CREATE_HELP: &str = "\
Examples:
  dopbase token create payment-service/production --name deploy
";
const LIST_HELP: &str = "\
Examples:
  dopbase token list payment-service/production
";
const REVOKE_HELP: &str = "\
Examples:
  dopbase token revoke tok_01ABCDEF
";
