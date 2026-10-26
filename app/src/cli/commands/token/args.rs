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
    /// Token lifetime: never or a whole number followed by h or d (maximum 3 years).
    #[arg(long, value_name = "DURATION", value_parser = parse_expiry)]
    expires_in: Option<String>,
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
  /// Manage your personal tokens, for scripts and AI agents that act as you.
  #[command(after_help = PERSONAL_HELP)]
  Personal {
    #[command(subcommand)]
    command: PersonalTokenCommand,
  },
}

fn parse_level(value: &str) -> Result<String, String> {
  match value {
    "metadata" | "read" | "write" => Ok(value.to_owned()),
    _ => Err("Use metadata, read, or write.".into()),
  }
}

#[derive(Subcommand, Debug)]
pub enum PersonalTokenCommand {
  /// Create a personal token. It reaches at most what you reach, within its
  /// own scope and level, and always expires.
  ///
  /// The value is shown once. Pass it to client commands through
  /// DOPBASE_TOKEN.
  #[command(after_help = PERSONAL_CREATE_HELP)]
  Create {
    /// Display name for the token, unique among yours.
    #[arg(long)]
    name: String,
    /// metadata (names only), read (values), or write (changes).
    #[arg(long, default_value = "read", value_parser = parse_level)]
    level: String,
    /// Create an audit token, which reads the audit log and no project
    /// data. For roles such as Auditor.
    #[arg(long, conflicts_with_all = ["project", "env"])]
    audit: bool,
    /// Limit the token to one project.
    #[arg(long, value_name = "PROJECT_REF", conflicts_with = "env")]
    project: Option<String>,
    /// Limit the token to one environment.
    #[arg(long, value_name = "ENVIRONMENT_REF")]
    env: Option<String>,
    /// Lifetime as a whole number followed by h or d. Defaults to your role's default.
    #[arg(long, value_name = "DURATION", value_parser = parse_expiry)]
    expires_in: Option<String>,
  },
  /// List your personal tokens.
  #[command(after_help = PERSONAL_LIST_HELP)]
  List,
  /// Revoke one of your personal tokens by ID.
  #[command(after_help = PERSONAL_REVOKE_HELP)]
  Revoke {
    /// ID of the token to revoke.
    token_id: String,
  },
}

pub(crate) const HELP: &str = "\
Examples:
  dopbase token create payment-service/production --name deploy
  dopbase token create payment-service/production --name deploy --expires-in 12h
  dopbase token list payment-service/production
  dopbase token revoke tok_01ABCDEF
  dopbase token personal create --name laptop-agent --env payment-service/development
";
const PERSONAL_HELP: &str = "\
Examples:
  dopbase token personal create --name laptop-agent --level metadata
  dopbase token personal list
  dopbase token personal revoke ptk_01ABCDEF
";
const PERSONAL_LIST_HELP: &str = "\
Examples:
  dopbase token personal list
  dopbase token personal list --json
";
const PERSONAL_REVOKE_HELP: &str = "\
Examples:
  dopbase token personal revoke ptk_01ABCDEF
";
const PERSONAL_CREATE_HELP: &str = "\
Examples:
  dopbase token personal create --name laptop-agent --level metadata
  dopbase token personal create --name ci --level write --project payment-service --expires-in 30d
  dopbase token personal create --name reader --env payment-service/production --expires-in 12h
  dopbase token personal create --name siem --audit --expires-in 90d
";
const CREATE_HELP: &str = "\
Examples:
  dopbase token create payment-service/production --name deploy
  dopbase token create payment-service/production --name deploy --expires-in 45d
  dopbase token create payment-service/production --name deploy --expires-in never
";
const LIST_HELP: &str = "\
Examples:
  dopbase token list payment-service/production
";
const REVOKE_HELP: &str = "\
Examples:
  dopbase token revoke tok_01ABCDEF
";
