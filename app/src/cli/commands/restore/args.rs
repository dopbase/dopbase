use std::path::PathBuf;

use clap::Args;

#[derive(Args, Debug)]
pub struct RestoreArgs {
  /// Path to the .dop backup file to restore.
  pub path: PathBuf,
  /// Master key file path or 64-character hex string (required when restoring onto a new server).
  #[arg(short, long, value_name = "KEY")]
  pub key: Option<String>,
  /// Setup token printed by the target server when restoring an uninitialized instance.
  #[arg(long, value_name = "TOKEN")]
  pub setup_token: Option<String>,
  /// Skip confirmation prompt.
  #[arg(long)]
  pub yes: bool,
}

pub(crate) const HELP: &str = "Examples:\n  dopbase restore ./backup.dop\n  dopbase restore ./backup.dop --setup-token dbs_... --yes\n";
