use clap::Args;

use crate::{constants::help::RUN_ENVIRONMENT_ARG_HELP, utils::env_layout::valid_key};

fn parse_key(value: &str) -> Result<String, String> {
  if valid_key(value) {
    Ok(value.to_owned())
  } else {
    Err("keys use letters, numbers, and '_', and cannot start with a number".into())
  }
}

#[derive(Args, Debug)]
pub struct RunArgs {
  #[arg(value_name = "ENVIRONMENT_REF", help = RUN_ENVIRONMENT_ARG_HELP)]
  pub environment: Option<String>,
  /// Inject only these secrets. Repeat the flag or separate keys with commas.
  #[arg(long, value_name = "KEY", value_delimiter = ',', value_parser = parse_key)]
  pub only: Vec<String>,
  /// Warn instead of stopping when an --only key is not stored.
  #[arg(long, requires = "only")]
  pub allow_missing: bool,
  /// Keep the existing value of these variables instead of the stored secret.
  /// Repeat the flag or separate keys with commas.
  #[arg(long, value_name = "KEY", value_delimiter = ',', value_parser = parse_key)]
  pub preserve_env: Vec<String>,
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
  dopbase run payment-service/production --only API_KEY,DATABASE_URL -- npm start
  dopbase run payment-service/development --preserve-env PORT,LOG_LEVEL -- npm run dev
  DOPBASE_TOKEN=dbs_... dopbase run -- npm start
";
