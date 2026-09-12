use std::path::PathBuf;

use clap::Subcommand;

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

pub(crate) const HELP: &str =
  "Examples:\n  dopbase admin reset-password admin@example.com\n  dopbase admin factory-reset\n";
const RESET_PASSWORD_HELP: &str = "Examples:\n  dopbase admin reset-password admin@example.com\n";
const FACTORY_RESET_HELP: &str = "Examples:\n  dopbase admin factory-reset\n  dopbase admin factory-reset --no-backup\n  dopbase --data-dir /srv/dopbase admin factory-reset\n";
