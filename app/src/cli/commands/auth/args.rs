use clap::Args;

pub(crate) const LOGIN_HELP: &str = "\
Examples:
  dopbase login
  dopbase login --token
  printf '%s' \"$TOKEN\" | dopbase login --token
";
pub(crate) const LOGOUT_HELP: &str = "\
Examples:
  dopbase logout
";

#[derive(Args, Debug)]
pub struct LoginArgs {
  /// Save a runner token instead of signing in with email and password.
  ///
  /// In a terminal, Dopbase prompts for the token without echoing it. When
  /// standard input is piped, Dopbase reads the token from standard input.
  #[arg(long)]
  pub token: bool,
}
