use clap::Args;

pub(crate) const LOGIN_HELP: &str = "Examples:\n  dopbase login\n  dopbase login --token\n  printf '%s' \"$TOKEN\" | dopbase login --token\n";
pub(crate) const LOGOUT_HELP: &str = "Examples:\n  dopbase logout\n";

#[derive(Args, Debug)]
pub struct LoginArgs {
  /// Save a runner token instead of signing in with email and password.
  ///
  /// In a terminal, Dopbase prompts for the token without echoing it. When
  /// standard input is piped, Dopbase reads the token from standard input.
  #[arg(long)]
  pub token: bool,
}
