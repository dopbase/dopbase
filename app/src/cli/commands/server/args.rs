use clap::{Args, Subcommand};
use std::path::PathBuf;

pub(crate) const HELP: &str = "\
Examples:
  dopbase server start
  dopbase server up --port 9000
  dopbase server status
  dopbase server logs --watch
  dopbase server down
";

const START_HELP: &str = "\
Examples:
  dopbase server start
  dopbase server start --port 9000
  dopbase server start --host 0.0.0.0 --public-url https://dopbase.example.com
";
const UP_HELP: &str = "\
Examples:
  dopbase server up
  dopbase server up --port 9000
  dopbase server up --host 0.0.0.0
  dopbase server up --host 0.0.0.0 --public-url http://203.0.113.10:8840
  dopbase server up --docs
";
const DOWN_HELP: &str = "\
Examples:
  dopbase server down
  dopbase server down --timeout 30
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
  /// Run the server in the foreground. Press Ctrl+C to stop it.
  #[command(after_help = START_HELP)]
  Start(ServerStartArgs),
  /// Start the server in the background on macOS or Linux.
  #[command(after_help = UP_HELP)]
  Up(ServerLaunchArgs),
  /// Stop the managed background server.
  #[command(after_help = DOWN_HELP)]
  Down {
    /// Seconds to wait for a graceful shutdown before forcing it.
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
  /// Internal: set by `server up` on the detached child process.
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
