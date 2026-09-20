use crate::cli::{
  environment_target, environment_target::EnvironmentTarget, secret_format::SecretFormat,
};
use clap::Args;
use std::path::PathBuf;

pub(crate) const HELP: &str = "\
Examples:
  dopbase init
  dopbase init payment-service/development --from .env
  dopbase init payment-service/development --from secrets.toml
  dopbase init payment-service/development --from secrets.json
  cat secrets.yml | dopbase init payment-service/development --from - --format yaml
";

#[derive(Args, Debug)]
pub struct InitArgs {
  /// New project and first environment, written as PROJECT_NAME/ENVIRONMENT_NAME.
  #[arg(
    value_name = "PROJECT_NAME/ENVIRONMENT_NAME",
    value_parser = environment_target::parse_init,
    requires = "from"
  )]
  pub target: Option<EnvironmentTarget>,
  /// Secret file to import, or - to read from stdin.
  #[arg(long, value_name = "FILE", requires = "target")]
  pub from: Option<PathBuf>,
  /// Input format. Required for stdin; otherwise inferred from the filename.
  #[arg(long, value_enum, requires = "from")]
  pub format: Option<SecretFormat>,
}
