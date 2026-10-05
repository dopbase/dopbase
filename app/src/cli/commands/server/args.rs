use clap::{Args, Subcommand};
use std::path::PathBuf;

pub(crate) const UP_MIGRATION: &str = "\
`dopbase server up` has been replaced by `dopbase server start --background`.
You can also use `dopbase server start -b`.

Use `dopbase server start` to run the server in this terminal.

No server was started.";
pub(crate) const DOWN_MIGRATION: &str = "\
`dopbase server down` has been replaced by `dopbase server stop`.
Use `dopbase server stop --timeout 30` to allow more time for shutdown.

No server was stopped.";

pub(crate) const HELP: &str = "\
Examples:
  dopbase server setup
  dopbase server start
  dopbase server start --background --port 9000
  dopbase server status
  dopbase server logs --watch
  dopbase server stop
";

const START_HELP: &str = "\
Examples:
  dopbase server start
  dopbase server start --port 9000
  dopbase server start --no-web-ui
  dopbase server start --background
  dopbase server start -b --port 9000
  dopbase server start --host 0.0.0.0 --public-url https://dopbase.example.com
";
const STOP_HELP: &str = "\
Examples:
  dopbase server stop
  dopbase server stop --timeout 30
";
const STATUS_HELP: &str = "\
Examples:
  dopbase server status
  dopbase --data-dir /srv/dopbase server status
";
const LOGS_HELP: &str = "\
Examples:
  dopbase server logs
  dopbase server logs --lines 50
  dopbase server logs --watch
  dopbase server logs --clean
";

#[derive(Subcommand, Debug)]
pub enum ServerCommand {
  /// Initialize a local instance before starting the server.
  #[command(after_help = SETUP_HELP)]
  Setup(ServerSetupArgs),
  /// Run the server in the foreground. Press Ctrl+C to stop it.
  #[command(after_help = START_HELP)]
  Start(ServerStartArgs),
  /// Stop the managed background server.
  #[command(after_help = STOP_HELP)]
  Stop {
    /// Seconds to wait for a graceful shutdown before forcing it.
    #[arg(long, default_value_t = 10)]
    timeout: u64,
  },
  /// Restart the running background server with its saved launch settings.
  #[command(
    after_help = "Examples:\n  dopbase server restart\n  dopbase server restart --timeout 30"
  )]
  Restart {
    /// Seconds to allow for graceful shutdown before forcing it.
    #[arg(long, default_value_t = 10)]
    timeout: u64,
  },
  #[command(hide = true, about = "Replaced by server start --background", after_help = UP_MIGRATION)]
  Up(ServerLaunchArgs),
  #[command(hide = true, about = "Replaced by server stop", after_help = DOWN_MIGRATION)]
  Down {
    /// Legacy shutdown timeout; this command only prints migration instructions.
    #[arg(long, default_value_t = 10)]
    timeout: u64,
  },
  /// Show whether the local server is running in the foreground or background.
  #[command(after_help = STATUS_HELP)]
  Status,
  /// Print logs written by the managed background server.
  #[command(after_help = LOGS_HELP)]
  Logs {
    /// Number of recent lines to print.
    #[arg(long, default_value_t = 100, value_name = "COUNT")]
    lines: usize,
    /// Clear the background server log before reading or watching it.
    #[arg(long)]
    clean: bool,
    /// Continue printing new lines until Ctrl+C.
    #[arg(short = 'w', long)]
    watch: bool,
  },
}

#[derive(Args, Debug, Default)]
pub struct ServerStartArgs {
  #[command(flatten)]
  pub launch: ServerLaunchArgs,
  /// Disable the web UI for this run, overriding server.toml or DOPBASE_WEB_UI.
  #[arg(long)]
  pub no_web_ui: bool,
  /// Start the server in the background on macOS or Linux.
  #[arg(short = 'b', long, conflicts_with = "supervised")]
  pub background: bool,
  /// Internal: set on the detached child process.
  #[arg(long, hide = true)]
  pub supervised: bool,
}

#[derive(Args, Clone, Debug, Default)]
pub struct ServerLaunchArgs {
  /// Server config file to load (default: server.toml in the data dir).
  #[arg(long, value_name = "FILE")]
  pub config: Option<PathBuf>,
  /// Port to listen on (default: 8840).
  #[arg(long, value_name = "PORT")]
  pub port: Option<u16>,
  /// Network interface to bind, e.g. 127.0.0.1 (default) or 0.0.0.0 to expose
  /// the server. Without --public-url, remote binds show SERVER_HOST.
  #[arg(long, value_name = "HOST")]
  pub host: Option<String>,
  /// Public URL clients use to reach this server (banners, generated links).
  /// Recommended for network binds, reverse proxies, and HTTPS.
  #[arg(long, value_name = "URL")]
  pub public_url: Option<String>,
  /// Seconds to wait for in-flight requests during shutdown.
  #[arg(long, value_name = "SECONDS")]
  pub shutdown_grace_seconds: Option<u64>,
  /// Enable Swagger UI at /api/docs. Off by default.
  #[arg(long, overrides_with = "no_docs")]
  pub docs: bool,
  /// Disable the API documentation for this run, overriding server.toml or DOPBASE_DOCS.
  #[arg(long, overrides_with = "docs")]
  pub no_docs: bool,
  /// Read the server master key from this file.
  #[arg(long, value_name = "FILE")]
  pub master_key_file: Option<PathBuf>,
}

impl ServerLaunchArgs {
  /// Fold the `--docs`/`--no-docs` pair into a single tri-state value
  /// (last flag on the command line wins).
  pub fn docs(&self) -> Option<bool> {
    if self.docs {
      Some(true)
    } else if self.no_docs {
      Some(false)
    } else {
      None
    }
  }
}

const SETUP_HELP: &str = "\
Examples:
  dopbase server setup
  dopbase server setup --email admin@example.com
  dopbase --data-dir /srv/dopbase --json server setup --email admin@example.com
  dopbase server setup --web
  dopbase server setup --web --port 9000
  dopbase server setup --web --email admin@example.com
  DOPBASE_ROOT_EMAIL=admin@example.com dopbase server setup

CLI setup creates the root account and exits without starting a server.
--email or DOPBASE_ROOT_EMAIL generates a password and prints it once in CLI mode.
--email overrides DOPBASE_ROOT_EMAIL. Empty environment values use guided setup.
Save the generated password before closing the terminal.
--web enables the web UI for this run, even when web_ui or DOPBASE_WEB_UI is false.
The setup server keeps running until you stop it with Ctrl+C.
An email supplied with --web prefills the form through the printed setup link.
Use the same --data-dir and --config options when starting the instance.
";

#[derive(Args, Debug, Default)]
pub struct ServerSetupArgs {
  /// Use the existing web setup flow in this terminal.
  #[arg(long)]
  pub web: bool,
  /// Root email; generates a password in CLI mode or prefills web setup.
  /// Defaults to DOPBASE_ROOT_EMAIL when set.
  #[arg(long, value_name = "EMAIL")]
  pub email: Option<String>,
  #[command(flatten)]
  pub launch: ServerLaunchArgs,
}
