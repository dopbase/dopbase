use std::path::PathBuf;

use clap::Args;

use crate::cli::secret_format::SecretFormat;
use crate::constants::help::ENVIRONMENT_ARG_HELP;

#[derive(Args, Debug)]
pub struct ExportArgs {
  #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
  pub environment: String,
  /// File to write. The format is inferred from its filename by default.
  #[arg(
    long,
    value_name = "FILE",
    conflicts_with = "stdout",
    required_unless_present = "stdout"
  )]
  pub output: Option<PathBuf>,
  /// Print plaintext secrets to stdout. Defaults to dotenv format.
  #[arg(long, conflicts_with = "output", required_unless_present = "output")]
  pub stdout: bool,
  /// Output format. Overrides filename inference.
  #[arg(long, value_enum)]
  pub format: Option<SecretFormat>,
  /// Overwrite the output file if it already exists.
  #[arg(long)]
  pub force: bool,
}

pub(crate) const HELP: &str = "Examples:\n  dopbase export payment-service/production --output .env.production\n  dopbase export payment-service/production --output secrets.json\n  dopbase export payment-service/production --stdout --format yaml\n";
