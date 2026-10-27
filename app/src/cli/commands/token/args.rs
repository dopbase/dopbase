use clap::Subcommand;

use crate::constants::help::ENVIRONMENT_ARG_HELP;
use crate::services::token;

fn parse_expiry(value: &str) -> Result<String, String> {
  token::expiry_duration(value)
    .map(|_| value.to_owned())
    .map_err(str::to_owned)
}

#[derive(Subcommand, Debug)]
pub enum TokenCommand {
  /// Create an access token for an environment or workspace (e.g. for CI/CD).
  ///
  /// The token value is shown once at creation. Pass it to client commands
  /// via the DOPBASE_TOKEN environment variable.
  #[command(after_help = CREATE_HELP)]
  Create {
    #[arg(
      value_name = "ENVIRONMENT_REF",
      help = ENVIRONMENT_ARG_HELP,
      required_unless_present = "workspace"
    )]
    environment: Option<String>,
    /// Workspace name or ID (mutually exclusive with the environment argument).
    #[arg(long, value_name = "WORKSPACE_REF", conflicts_with = "environment")]
    workspace: Option<String>,
    /// Display name for the token.
    #[arg(long)]
    name: String,
    /// Token role (`runner` for an environment, `workspace` for a workspace).
    #[arg(long, default_value = "runner")]
    role: String,
    /// Token lifetime: never or a whole number followed by h or d (maximum 3 years).
    #[arg(long, value_name = "DURATION", value_parser = parse_expiry)]
    expires_in: Option<String>,
  },
  /// List tokens for an environment or workspace.
  #[command(after_help = LIST_HELP)]
  List {
    #[arg(
      value_name = "ENVIRONMENT_REF",
      help = ENVIRONMENT_ARG_HELP,
      required_unless_present = "workspace"
    )]
    environment: Option<String>,
    /// Workspace name or ID (mutually exclusive with the environment argument).
    #[arg(long, value_name = "WORKSPACE_REF", conflicts_with = "environment")]
    workspace: Option<String>,
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
  dopbase token create --workspace default --name deploy --role workspace
  dopbase token list payment-service/production
  dopbase token list --workspace default
  dopbase token revoke tok_01ABCDEF
";
const CREATE_HELP: &str = "\
Examples:
  dopbase token create payment-service/production --name deploy
  dopbase token create payment-service/production --name deploy --expires-in 45d
  dopbase token create --workspace default --name deploy --role workspace
";
const LIST_HELP: &str = "\
Examples:
  dopbase token list payment-service/production
  dopbase token list --workspace default
";
const REVOKE_HELP: &str = "\
Examples:
  dopbase token revoke tok_01ABCDEF
  dopbase token revoke wtk_01ABCDEF
";
