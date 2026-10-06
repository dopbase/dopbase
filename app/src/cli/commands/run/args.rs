use clap::Args;

use crate::constants::help::ENVIRONMENT_ARG_HELP;

#[derive(Args, Debug)]
pub struct RunArgs {
  #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
  pub environment: Option<String>,
  /// Command to run with the injected secrets.
  #[arg(last = true, required = true)]
  pub command: Vec<String>,
}

pub(crate) const HELP: &str = "\
Examples:
  dopbase run -- npm run dev
  dopbase run env_482731 -- npm run dev
  dopbase run payment-service/development -- npm run dev
  dopbase run payment-service/production -- npm start
";
