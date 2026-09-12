use std::path::PathBuf;

use clap::Args;

use crate::cli::secret_format::SecretFormat;
use crate::constants::help::ENVIRONMENT_ARG_HELP;

#[derive(Args, Debug)]
pub struct ImportArgs {
  #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
  pub environment: String,
  /// Secret file to import, or - to read from stdin.
  pub path: PathBuf,
  /// Input format. Required for stdin; otherwise inferred from the filename.
  #[arg(long, value_enum)]
  pub format: Option<SecretFormat>,
  /// Preview what would change without applying it.
  #[arg(long)]
  pub dry_run: bool,
  /// Overwrite existing keys and delete keys missing from the file.
  #[arg(long, conflicts_with = "dry_run")]
  pub replace: bool,
  /// Skip confirmation prompts (for automation).
  #[arg(long)]
  pub yes: bool,
}

pub(crate) const HELP: &str = "Examples:\n  dopbase import payment-service/production .env.production\n  dopbase import payment-service/production secrets.json --dry-run\n  cat secrets.yml | dopbase import payment-service/production - --format yaml\n";
