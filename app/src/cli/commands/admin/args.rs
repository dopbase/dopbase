use std::path::PathBuf;

use clap::Subcommand;

use crate::constants::limits::{
  LAUNCH_PASSCODE_DEFAULT_SECONDS, LAUNCH_PASSCODE_MAX_SECONDS, LAUNCH_PASSCODE_MIN_SECONDS,
};

#[derive(Subcommand, Debug)]
pub enum AdminCommand {
  /// Reset a user's password (offline recovery on the server machine).
  ///
  /// Loads the local server configuration and database directly, so it must
  /// be run on the machine hosting the server and requires an interactive
  /// terminal.
  #[command(after_help = RESET_PASSWORD_HELP)]
  ResetPassword {
    /// Email of the account to reset.
    email: String,
    /// Server config file to load (server.toml).
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,
    /// File containing the server master key.
    #[arg(long, value_name = "FILE")]
    master_key_file: Option<PathBuf>,
  },
  /// Print a one-time passcode that signs an account in to the Admin UI.
  ///
  /// Run it on the machine hosting the running server, as a user who can
  /// write the server's data directory. Open the printed link to sign in
  /// without a password. The passcode works once and expires after --ttl
  /// seconds; running the command again replaces it.
  #[command(after_help = PASSCODE_HELP)]
  Passcode {
    /// Email of the account to sign in.
    email: String,
    /// Seconds the passcode stays valid, from 15 to 600.
    #[arg(
      long,
      value_name = "SECONDS",
      default_value_t = LAUNCH_PASSCODE_DEFAULT_SECONDS,
      value_parser = clap::value_parser!(i64).range(LAUNCH_PASSCODE_MIN_SECONDS..=LAUNCH_PASSCODE_MAX_SECONDS)
    )]
    ttl: i64,
    /// Server config file to load (server.toml).
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,
  },
  /// Remove a local instance from the server machine.
  ///
  /// Removes the entire Dopbase data directory from its active location. A ZIP
  /// backup is created first unless --no-backup is passed. The server must be
  /// stopped and an interactive root password confirmation is required.
  #[command(after_help = FACTORY_RESET_HELP)]
  FactoryReset {
    /// Server config file to load (server.toml).
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,
    /// Permanently reset the instance without creating a ZIP backup.
    #[arg(long)]
    no_backup: bool,
  },
}

pub(crate) const HELP: &str = "\
Examples:
  dopbase admin reset-password admin@example.com
  dopbase admin passcode root@example.com
  dopbase admin factory-reset
";
const RESET_PASSWORD_HELP: &str = "\
Examples:
  dopbase admin reset-password admin@example.com
";
const PASSCODE_HELP: &str = "\
Examples:
  dopbase admin passcode root@example.com
  dopbase admin passcode alice@example.com --ttl 120
  dopbase --json admin passcode root@example.com
";
const FACTORY_RESET_HELP: &str = "\
Examples:
  dopbase admin factory-reset
  dopbase admin factory-reset --no-backup
  dopbase --data-dir /srv/dopbase admin factory-reset
";
