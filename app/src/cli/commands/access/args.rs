use clap::Args;

#[derive(Args, Debug)]
pub struct AccessArgs {
  /// Show who can reach this project instead of your own access. Needs the
  /// project manager level on it, or the access.report capability.
  #[arg(long, value_name = "PROJECT_REF", conflicts_with = "user")]
  pub project: Option<String>,
  /// Show another account's access. A person's needs users.view; an AI
  /// agent's needs agents.view_all, or agents.view if you own it.
  #[arg(long, value_name = "USER_ID")]
  pub user: Option<String>,
}

pub(crate) const HELP: &str = "\
Shows what your account, or the personal token in DOPBASE_TOKEN, can reach.
Access is changed in the Admin UI, on a project's Access page. An AI agent's
owner gives it access from the agent's page.

Examples:
  dopbase access
  DOPBASE_TOKEN=dpp_... dopbase access
  dopbase access --project payment-service
  dopbase access --user usr_01ABCDEF
";
