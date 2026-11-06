use clap::Args;

pub(crate) const LOGIN_HELP: &str = "\
Login stops while DOPBASE_TOKEN is set, because the token takes priority
over a saved login. Unset it first.

Examples:
  dopbase login
  dopbase login --browser
  dopbase login --browser-open
  dopbase login --token
  printf '%s' \"$TOKEN\" | dopbase login --token
";
pub(crate) const LOGOUT_HELP: &str = "\
Logout stops while DOPBASE_TOKEN is set, because the token keeps signing
commands in. Unset it first.

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

  /// Approve the login in the Admin UI instead of typing a password here.
  ///
  /// Dopbase prints a link and a short code, then waits until you open the
  /// link, sign in, and approve the code. Copy the link into any browser,
  /// including one on another machine.
  #[arg(long, conflicts_with_all = ["token", "password"])]
  pub browser: bool,

  /// Like --browser, and also open the link in this machine's browser.
  #[arg(long, conflicts_with_all = ["token", "password"])]
  pub browser_open: bool,

  /// Sign in with email and password even when the server prefers the
  /// browser.
  #[arg(long, conflicts_with = "token")]
  pub password: bool,
}
