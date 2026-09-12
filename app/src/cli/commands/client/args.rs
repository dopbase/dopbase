use clap::Subcommand;

pub(crate) const HELP: &str = "\
Examples:
  dopbase client connect https://dopbase.example.com
  dopbase client connect local
  dopbase client status
";
pub(crate) const STATUS_ALIAS_HELP: &str = "Examples:\n  dopbase status\n";

const CONNECT_HELP: &str = "\
Examples:
  dopbase client connect https://dopbase.example.com
  dopbase client connect http://dopbase.internal
  dopbase client connect 192.168.1.20:8840
  dopbase client connect local
";
const STATUS_HELP: &str = "\
Examples:
  dopbase client status
  dopbase client status --json
";

#[derive(Subcommand, Debug)]
pub enum ClientCommand {
  /// Validate a server URL and save it as the active server.
  ///
  /// Domains must include http:// or https://. Bare IP addresses use HTTP.
  /// Accepts the `local` alias to return to the implicit local default
  /// (http://localhost:8840). Changing servers requires interactive
  /// confirmation, stops the current managed background server, clears the
  /// saved CLI session and default, and then requires a new login.
  #[command(after_help = CONNECT_HELP)]
  Connect {
    /// Server URL or bare IP to save, or `local` to use http://localhost:8840.
    server_url: String,
  },
  /// Show the active server, connection state, login, and default environment.
  #[command(after_help = STATUS_HELP)]
  Status,
}
