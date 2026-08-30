use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "dopbase", version, about = "Secrets management in one binary")]
pub struct Cli {
  #[arg(long, global = true)]
  pub server: Option<String>,
  #[arg(long, global = true, value_name = "DIR")]
  pub data_dir: Option<PathBuf>,
  #[arg(long, global = true)]
  pub json: bool,
  #[command(subcommand)]
  pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
  Serve(ServeArgs),
  Client {
    #[command(subcommand)]
    command: ClientCommand,
  },
  Login,
  Logout,
  Config,
  Init {
    project: String,
    environment: String,
    #[arg(long, value_name = "FILE")]
    from: PathBuf,
  },
  Project {
    #[command(subcommand)]
    command: ProjectCommand,
  },
  Env {
    #[command(subcommand)]
    command: EnvCommand,
  },
  Secret {
    #[command(subcommand)]
    command: SecretCommand,
  },
  Import {
    environment: String,
    path: PathBuf,
    #[arg(long)]
    dry_run: bool,
    #[arg(long, conflicts_with = "dry_run")]
    replace: bool,
    #[arg(long)]
    yes: bool,
  },
  Export {
    environment: String,
    #[arg(long, value_name = "FILE", conflicts_with = "stdout")]
    output: Option<PathBuf>,
    #[arg(long, conflicts_with = "output")]
    stdout: bool,
    #[arg(long)]
    force: bool,
  },
  Token {
    #[command(subcommand)]
    command: TokenCommand,
  },
  Run {
    environment: Option<String>,
    #[arg(last = true, required = true)]
    command: Vec<String>,
  },
  Admin {
    #[command(subcommand)]
    command: AdminCommand,
  },
  /// Check GitHub for a newer Dopbase release (informational only).
  Update,
  /// Stop the background server started with `serve --background`.
  Stop {
    /// Seconds to wait for a graceful shutdown before forcing it.
    #[arg(long, default_value_t = 10)]
    timeout: u64,
  },
}

#[derive(Args, Debug, Default)]
pub struct ServeArgs {
  #[arg(long)]
  pub config: Option<PathBuf>,
  /// Port to listen on (with --host). Cannot be combined with --bind-address.
  #[arg(long, value_name = "PORT", conflicts_with = "bind_address")]
  pub port: Option<u16>,
  /// Network interface to bind, e.g. 127.0.0.1 (default) or 0.0.0.0 to expose
  /// the server. Requires --public-url when binding beyond loopback. Cannot be
  /// combined with --bind-address.
  #[arg(long, value_name = "HOST", conflicts_with = "bind_address")]
  pub host: Option<String>,
  /// Advanced: full socket to bind, e.g. 127.0.0.1:8840. Cannot be combined
  /// with --port/--host.
  #[arg(long)]
  pub bind_address: Option<String>,
  /// Public URL clients use to reach this server (banners, generated links).
  /// Optional for loopback binds; required with --host 0.0.0.0.
  #[arg(long)]
  pub public_url: Option<String>,
  #[arg(long)]
  pub database_url: Option<String>,
  #[arg(long)]
  pub shutdown_grace_seconds: Option<u64>,
  /// Serve the API documentation (Swagger UI) at /api/docs. Off by default.
  #[arg(long, overrides_with = "no_docs")]
  pub docs: bool,
  /// Disable the API documentation for this run, overriding server.toml or DOPBASE_DOCS.
  #[arg(long, overrides_with = "docs")]
  pub no_docs: bool,
  /// Run the server detached in the background (macOS and Linux).
  #[arg(long)]
  pub background: bool,
  /// Internal: set by `serve --background` on the detached child process.
  #[arg(long, hide = true)]
  pub supervised: bool,
  #[arg(long)]
  pub master_key_file: Option<PathBuf>,
}

impl ServeArgs {
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
#[derive(Subcommand, Debug)]
pub enum ClientCommand {
  Connect { server_url: String },
}
#[derive(Subcommand, Debug)]
pub enum ProjectCommand {
  Create {
    name: String,
  },
  List,
  Show {
    project: String,
  },
  Rename {
    project: String,
    new_name: String,
  },
  Delete {
    project: String,
    #[arg(long)]
    yes: bool,
  },
}
#[derive(Subcommand, Debug)]
pub enum EnvCommand {
  Create {
    project: String,
    name: String,
  },
  List {
    project: Option<String>,
  },
  Show {
    environment: String,
  },
  Rename {
    environment: String,
    new_name: String,
  },
  Delete {
    environment: String,
    #[arg(long)]
    yes: bool,
  },
}
#[derive(Subcommand, Debug)]
pub enum SecretCommand {
  List {
    environment: String,
  },
  Set {
    environment: String,
    key: String,
    #[arg(long)]
    stdin: bool,
  },
  Get {
    environment: String,
    key: String,
    #[arg(long)]
    reveal: bool,
  },
  Delete {
    environment: String,
    key: String,
    #[arg(long)]
    yes: bool,
  },
}
#[derive(Subcommand, Debug)]
pub enum TokenCommand {
  Create {
    environment: String,
    #[arg(long)]
    name: String,
    #[arg(long, default_value = "runner")]
    role: String,
  },
  List {
    environment: String,
  },
  Revoke {
    token_id: String,
  },
}
#[derive(Subcommand, Debug)]
pub enum AdminCommand {
  ResetPassword {
    email: String,
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    database_url: Option<String>,
    #[arg(long)]
    master_key_file: Option<PathBuf>,
  },
}
