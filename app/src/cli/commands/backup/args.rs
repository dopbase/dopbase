use std::path::PathBuf;

use clap::Args;

#[derive(Args, Debug)]
pub struct BackupArgs {
  /// Optional backup name (default: dopbase_backup_<timestamp>.dop).
  pub name: Option<String>,
  /// Optional local file path to download and save the backup to.
  #[arg(short, long, value_name = "FILE")]
  pub output: Option<PathBuf>,
}

pub(crate) const HELP: &str =
  "Examples:\n  dopbase backup\n  dopbase backup pre-migration --output ./pre-migration.dop\n";
