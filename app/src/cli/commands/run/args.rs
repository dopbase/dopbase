use clap::Args;

use crate::constants::help::ENVIRONMENT_ARG_HELP;

#[derive(Args, Debug)]
pub struct RunArgs {
  #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
  pub environment: Option<String>,
  /// Runner token for this invocation. Overrides DOPBASE_TOKEN and the saved credential.
  #[arg(short = 't', long, value_name = "TOKEN")]
  pub token: Option<String>,
  /// Command to run with the injected secrets.
  #[arg(last = true, required = true)]
  pub command: Vec<String>,
}

pub(crate) const HELP: &str = "Examples:\n  dopbase run -- npm run dev\n  dopbase run env_482731 -- npm run dev\n  dopbase run payment-service/development -- npm run dev\n  dopbase run payment-service/production -t dbs_xxx -- npm start\n  dopbase run payment-service/production --token dbs_xxx -- npm start\n";
