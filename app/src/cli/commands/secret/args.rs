use clap::Subcommand;

use crate::constants::help::ENVIRONMENT_ARG_HELP;
use crate::services::rotation;

fn parse_grace(value: &str) -> Result<String, String> {
  rotation::grace_duration(value)
    .map(|_| value.to_owned())
    .map_err(str::to_owned)
}

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
    /// Also print the previous value while its rotation grace period lasts.
    #[arg(long, requires = "reveal")]
    include_previous: bool,
  },
  /// Store a new value and keep the old one valid for a grace period.
  ///
  /// Rotation requires an existing secret. Consumers that still hold the
  /// previous value keep working until the grace period ends, so no
  /// deployment has to change at the same moment.
  #[command(after_help = ROTATE_HELP)]
  Rotate {
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
    environment: String,
    /// Secret key name.
    key: String,
    /// How long the previous value stays valid: 0, or a whole number of hours or days.
    #[arg(long, value_name = "DURATION", default_value = "24h", value_parser = parse_grace)]
    grace: String,
    /// Read the new value until EOF. In a terminal, finish with Ctrl+D (Ctrl+Z then Enter on Windows).
    #[arg(long)]
    stdin: bool,
  },
  /// Roll a secret back to the value replaced by its latest rotation.
  ///
  /// Asks for confirmation unless --yes is passed.
  #[command(after_help = PROMOTE_HELP)]
  Promote {
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
    environment: String,
    /// Secret key name.
    key: String,
    /// Skip the confirmation prompt (for automation).
    #[arg(long)]
    yes: bool,
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

pub(crate) const HELP: &str = "\
Examples:
  dopbase secret list payment-service/production
  dopbase secret set payment-service/production API_KEY
  dopbase secret get payment-service/production API_KEY
  dopbase secret get payment-service/production API_KEY --reveal
  dopbase secret rotate payment-service/production API_KEY
  dopbase secret promote payment-service/production API_KEY
  dopbase secret delete payment-service/production API_KEY

Use PROJECT_REF/ENVIRONMENT_NAME for readable references. PROJECT_REF can be a
project ID or name. Immutable environment IDs such as env_482731 also work.
";
const LIST_HELP: &str = "\
Examples:
  dopbase secret list payment-service/production
  dopbase secret list env_482731

Run `dopbase env list` to find an environment.
";
const SET_HELP: &str = "\
Examples:
  dopbase secret set payment-service/production API_KEY
  dopbase secret set payment-service/production API_KEY --stdin
  printf '%s' \"$API_KEY\" | dopbase secret set payment-service/production API_KEY --stdin

Without --stdin, Dopbase uses a masked prompt and displays * for each character.

With --stdin, Dopbase reads the value until EOF. In a terminal, paste or type
the value, then press Ctrl+D. On Windows, press Ctrl+Z, then Enter. Piped input
is read exactly as supplied.

Overwriting a secret replaces it: any value kept by an earlier rotation is
retired. Use `secret rotate` to keep the old value readable for a while.
";
const GET_HELP: &str = "\
Examples:
  dopbase secret get payment-service/production API_KEY
  dopbase secret get payment-service/production API_KEY --reveal
  dopbase secret get payment-service/production API_KEY --reveal --include-previous

--include-previous prints the value replaced by the latest rotation together
with the time it stops being served. It is meant for checking a rotation
before the grace period ends.
";
const ROTATE_HELP: &str = "\
Examples:
  dopbase secret rotate payment-service/production API_KEY
  dopbase secret rotate payment-service/production API_KEY --grace 72h
  printf '%s' \"$API_KEY\" | dopbase secret rotate payment-service/production API_KEY --stdin

Without --stdin, Dopbase uses a masked prompt and displays * for each character.

--grace keeps the previous value readable for that long, so consumers can pick
up the new value without downtime. It defaults to 24h and accepts 0, or a
whole number of hours or days. --grace 0 retires the previous value at once.
";
const PROMOTE_HELP: &str = "\
Examples:
  dopbase secret promote payment-service/production API_KEY
  dopbase secret promote payment-service/production API_KEY --yes

Promote swaps the current and previous values. The demoted value stays
readable until the retire deadline the rotation recorded, so the rollback
can be rolled back the same way.
";
const DELETE_HELP: &str = "\
Examples:
  dopbase secret delete payment-service/production API_KEY
  dopbase secret delete payment-service/production API_KEY --yes
";
