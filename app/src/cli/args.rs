use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

const AFTER_HELP: &str = "\
Quickstart:
  dopbase serve                            # start a server on http://localhost:8840
  dopbase login                            # authenticate with the active server
  dopbase init myapp dev --from .env       # create a project + environment from a dotenv file
  dopbase secret set myapp/dev API_KEY --stdin
  dopbase run myapp/dev -- node server.js  # run with secrets injected as env vars

Run 'dopbase help <command>' for details on any command.
";

#[derive(Parser, Debug)]
#[command(
  name = "dopbase",
  version,
  about = "Secrets management in one binary",
  long_about = "Dopbase keeps application secrets in one binary: run a server, store \
secrets per project and environment, and inject them into any command with `run`.",
  after_help = AFTER_HELP
)]
pub struct Cli {
  /// Server URL to use for this invocation, overriding the saved server and
  /// DOPBASE_URL (e.g. https://dopbase.example.com).
  #[arg(long, global = true, value_name = "URL")]
  pub server: Option<String>,
  /// Directory for Dopbase state and configuration.
  #[arg(long, global = true, value_name = "DIR")]
  pub data_dir: Option<PathBuf>,
  /// Print machine-readable JSON instead of human-readable output.
  #[arg(long, global = true)]
  pub json: bool,
  #[command(subcommand)]
  pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
  /// Start a Dopbase server that stores projects, environments, and secrets.
  ///
  /// Binds to 127.0.0.1:8840 by default; see `dopbase help serve` for
  /// networking and background-mode options.
  Serve(ServeArgs),
  /// Connect the CLI to a Dopbase server (`client connect <url>`).
  ///
  /// Without a saved server, client commands use http://localhost:8840.
  Client {
    #[command(subcommand)]
    command: ClientCommand,
  },
  /// Authenticate with the active server.
  ///
  /// Example: dopbase login
  Login,
  /// Remove the saved credential for the active server.
  ///
  /// Example: dopbase logout
  Logout,
  /// Show effective client settings: config path, server, and auth status.
  ///
  /// Example: dopbase config
  Config,
  /// Create a project, its first environment, and import secrets.
  ///
  /// Bootstraps a new project on the server from an existing dotenv file.
  ///
  /// Example: dopbase init payment-service development --from .env
  Init {
    /// Name of the project to create (unique on the server).
    project: String,
    /// Name of the first environment to create (e.g. development).
    environment: String,
    /// Dotenv file to import the initial secrets from.
    #[arg(long, value_name = "FILE")]
    from: PathBuf,
  },
  /// Manage projects (create, list, show, rename, delete).
  ///
  /// Example: dopbase project list
  Project {
    #[command(subcommand)]
    command: ProjectCommand,
  },
  /// Manage environments inside a project (create, list, rename, delete).
  ///
  /// Example: dopbase env list payment-service
  Env {
    #[command(subcommand)]
    command: EnvCommand,
  },
  /// Manage secrets in an environment (list, set, get, delete).
  ///
  /// Example: dopbase secret list payment-service/production
  Secret {
    #[command(subcommand)]
    command: SecretCommand,
  },
  /// Bulk-import secrets into an environment from a dotenv file.
  ///
  /// Existing keys are kept unless --replace is passed. Use --dry-run to
  /// preview the result without changing anything.
  ///
  /// Example: dopbase import payment-service/production .env.production
  Import {
    /// Environment to import into (ID or `project/environment`).
    environment: String,
    /// Dotenv file to read secrets from.
    path: PathBuf,
    /// Preview what would change without applying it.
    #[arg(long)]
    dry_run: bool,
    /// Overwrite existing keys and delete keys missing from the file.
    #[arg(long, conflicts_with = "dry_run")]
    replace: bool,
    /// Skip confirmation prompts (for automation).
    #[arg(long)]
    yes: bool,
  },
  /// Export an environment's secrets to a dotenv file or stdout.
  ///
  /// Requires --output <FILE> or --stdout; --force overwrites an existing
  /// file.
  ///
  /// Example: dopbase export payment-service/production --output .env.prod
  Export {
    /// Environment to export (ID or `project/environment`).
    environment: String,
    /// File to write the dotenv output to.
    #[arg(long, value_name = "FILE", conflicts_with = "stdout")]
    output: Option<PathBuf>,
    /// Print the dotenv output to stdout instead of a file.
    #[arg(long, conflicts_with = "output")]
    stdout: bool,
    /// Overwrite the output file if it already exists.
    #[arg(long)]
    force: bool,
  },
  /// Manage CI/runner access tokens for an environment.
  ///
  /// Example: dopbase token list payment-service/production
  Token {
    #[command(subcommand)]
    command: TokenCommand,
  },
  /// Run a command with an environment's secrets injected as env vars.
  ///
  /// Falls back to the DOPBASE_ENV environment variable when no environment
  /// argument is given. Secret values are passed to the child process only
  /// and are never printed. Everything after `--` is the command to run.
  ///
  /// Example: dopbase run payment-service/production -- node server.js
  Run {
    /// Environment to inject secrets from (ID or `project/environment`).
    environment: Option<String>,
    /// Command to run with the injected secrets.
    #[arg(last = true, required = true)]
    command: Vec<String>,
  },
  /// Server administration (password reset).
  ///
  /// Example: dopbase admin reset-password admin@example.com
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
  /// Server config file to load (default: server.toml in the data dir).
  #[arg(long, value_name = "FILE")]
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
  /// Database connection URL override (else from server.toml).
  #[arg(long)]
  pub database_url: Option<String>,
  /// Seconds to wait for in-flight requests during shutdown.
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
  /// File containing the server master key (else from server.toml).
  #[arg(long, value_name = "FILE")]
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
  /// Validate a server URL and save it as the active server.
  ///
  /// Accepts a full URL or the `local` alias to return to the implicit
  /// local default (http://localhost:8840). Connecting clears the saved
  /// credential from the previous server; run `dopbase login` afterwards.
  ///
  /// Example: dopbase client connect https://dopbase.example.com
  Connect { server_url: String },
}
#[derive(Subcommand, Debug)]
pub enum ProjectCommand {
  /// Create an empty project.
  ///
  /// Example: dopbase project create payment-service
  Create {
    /// Project name, unique on the server.
    name: String,
  },
  /// List accessible projects.
  List,
  /// Show project metadata.
  ///
  /// Example: dopbase project show payment-service
  Show {
    /// Project ID or name.
    project: String,
  },
  /// Rename a project.
  ///
  /// Example: dopbase project rename payment-service payments
  Rename {
    /// Project ID or name.
    project: String,
    /// New project name.
    new_name: String,
  },
  /// Delete a project with all its environments, secrets, and tokens.
  ///
  /// Asks for confirmation unless --yes is passed.
  ///
  /// Example: dopbase project delete payment-service --yes
  Delete {
    /// Project ID or name.
    project: String,
    /// Skip the confirmation prompt (for automation).
    #[arg(long)]
    yes: bool,
  },
}
#[derive(Subcommand, Debug)]
pub enum EnvCommand {
  /// Create an environment inside a project.
  ///
  /// Example: dopbase env create payment-service production
  Create {
    /// Project ID or name.
    project: String,
    /// Environment name, unique within the project.
    name: String,
  },
  /// List environments, either for one project or all accessible ones.
  ///
  /// Example: dopbase env list payment-service
  List {
    /// Limit the listing to this project (ID or name).
    project: Option<String>,
  },
  /// Show environment metadata.
  ///
  /// Example: dopbase env show payment-service/production
  Show {
    /// Environment ID or `project/environment` reference.
    environment: String,
  },
  /// Rename an environment.
  ///
  /// Example: dopbase env rename payment-service/production prod
  Rename {
    /// Environment ID or `project/environment` reference.
    environment: String,
    /// New environment name.
    new_name: String,
  },
  /// Delete an environment with its secrets and tokens.
  ///
  /// Asks for confirmation unless --yes is passed.
  ///
  /// Example: dopbase env delete payment-service/staging --yes
  Delete {
    /// Environment ID or `project/environment` reference.
    environment: String,
    /// Skip the confirmation prompt (for automation).
    #[arg(long)]
    yes: bool,
  },
}
#[derive(Subcommand, Debug)]
pub enum SecretCommand {
  /// List secret keys in an environment (values are never shown).
  ///
  /// Example: dopbase secret list payment-service/production
  List {
    /// Environment ID or `project/environment` reference.
    environment: String,
  },
  /// Set a secret from an argument or, with --stdin, from standard input.
  ///
  /// Example: dopbase secret set payment-service/production API_KEY --stdin
  Set {
    /// Environment ID or `project/environment` reference.
    environment: String,
    /// Secret key name.
    key: String,
    /// Read the value from standard input instead of an argument.
    #[arg(long)]
    stdin: bool,
  },
  /// Print a secret value (masked unless --reveal).
  ///
  /// Example: dopbase secret get payment-service/production API_KEY
  Get {
    /// Environment ID or `project/environment` reference.
    environment: String,
    /// Secret key name.
    key: String,
    /// Print the actual value instead of a mask.
    #[arg(long)]
    reveal: bool,
  },
  /// Delete a secret from an environment.
  ///
  /// Asks for confirmation unless --yes is passed.
  ///
  /// Example: dopbase secret delete payment-service/production API_KEY --yes
  Delete {
    /// Environment ID or `project/environment` reference.
    environment: String,
    /// Secret key name.
    key: String,
    /// Skip the confirmation prompt (for automation).
    #[arg(long)]
    yes: bool,
  },
}
#[derive(Subcommand, Debug)]
pub enum TokenCommand {
  /// Create an access token for an environment (e.g. for CI/CD).
  ///
  /// The token value is shown once at creation; pass it to client commands
  /// via the DOPBASE_TOKEN environment variable.
  ///
  /// Example: dopbase token create payment-service/production --name deploy
  Create {
    /// Environment to scope the token to (ID or `project/environment`).
    environment: String,
    /// Display name for the token.
    #[arg(long)]
    name: String,
    /// Token role.
    #[arg(long, default_value = "runner")]
    role: String,
  },
  /// List tokens for an environment.
  ///
  /// Example: dopbase token list payment-service/production
  List {
    /// Environment ID or `project/environment` reference.
    environment: String,
  },
  /// Revoke a token by ID.
  ///
  /// Example: dopbase token revoke tok_01ABCDEF...
  Revoke {
    /// ID of the token to revoke.
    token_id: String,
  },
}
#[derive(Subcommand, Debug)]
pub enum AdminCommand {
  /// Reset a user's password (offline recovery on the server machine).
  ///
  /// Loads the local server configuration and database directly, so it must
  /// be run on the machine hosting the server and requires an interactive
  /// terminal.
  ///
  /// Example: dopbase admin reset-password admin@example.com
  ResetPassword {
    /// Email of the account to reset.
    email: String,
    /// Server config file to load (server.toml).
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,
    /// Database connection URL override.
    #[arg(long)]
    database_url: Option<String>,
    /// File containing the server master key.
    #[arg(long, value_name = "FILE")]
    master_key_file: Option<PathBuf>,
  },
}
