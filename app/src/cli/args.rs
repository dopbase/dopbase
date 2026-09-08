use clap::{Args, Parser, Subcommand, error::ErrorKind};
use std::{
  ffi::{OsStr, OsString},
  path::PathBuf,
};

const AFTER_HELP: &str = "\
Quickstart:
  dopbase server start                     # run a server on http://localhost:8840
  dopbase server up                        # run the server in the background
  dopbase login                            # authenticate with the active server
  dopbase init myapp dev --from .env       # create a project + environment from a dotenv file
  dopbase secret set myapp/dev API_KEY --stdin
  dopbase run myapp/dev -- node server.js  # run with secrets injected as env vars

Common server options:
  --host <HOST>    Bind host (default: 127.0.0.1)
  --port <PORT>    Listen port (default: 8840)

Run 'dopbase help <command>' for details on any command.
";

const SERVER_HELP: &str = "\
Examples:
  dopbase server start
  dopbase server up --port 9000
  dopbase server status
  dopbase server logs --follow
  dopbase server down
";

const CLIENT_HELP: &str = "\
Examples:
  dopbase client connect https://dopbase.example.com
  dopbase client connect local
  dopbase client status
";

const PROJECT_HELP: &str = "\
Examples:
  dopbase project create payment-service
  dopbase project list
  dopbase project show payment-service
  dopbase project rename payment-service payments
  dopbase project delete payment-service
";

const ENV_HELP: &str = "\
Examples:
  dopbase env create payment-service production
  dopbase env list payment-service
  dopbase env show payment-service/production
  dopbase env default payment-service/development
  dopbase env rename payment-service/production prod
  dopbase env delete payment-service/staging
";

const SECRET_HELP: &str = "\
Examples:
  dopbase secret list payment-service/production
  dopbase secret set payment-service/production API_KEY
  dopbase secret get payment-service/production API_KEY
  dopbase secret get payment-service/production API_KEY --reveal
  dopbase secret delete payment-service/production API_KEY

Use project/environment for readable references. Run `dopbase env list` to find
an environment. Immutable IDs such as env_01ABCDEF are also accepted.
";

const SECRET_LIST_HELP: &str = "\
Examples:
  dopbase secret list payment-service/production
  dopbase secret list env_01ABCDEF

Run `dopbase env list` to find an environment.
";

const SECRET_SET_HELP: &str = "\
Examples:
  dopbase secret set payment-service/production API_KEY
  printf '%s' \"$API_KEY\" | dopbase secret set payment-service/production API_KEY --stdin

Without --stdin, Dopbase prompts for the value without showing it on screen.
";

const SECRET_GET_HELP: &str = "\
Examples:
  dopbase secret get payment-service/production API_KEY
  dopbase secret get payment-service/production API_KEY --reveal
";

const SECRET_DELETE_HELP: &str = "\
Examples:
  dopbase secret delete payment-service/production API_KEY
  dopbase secret delete payment-service/production API_KEY --yes
";

const TOKEN_HELP: &str = "\
Examples:
  dopbase token create payment-service/production --name deploy
  dopbase token list payment-service/production
  dopbase token revoke tok_01ABCDEF
";

const ADMIN_HELP: &str = "\
Examples:
  dopbase admin reset-password admin@example.com
";

const ENVIRONMENT_ARG_HELP: &str = "Environment ID or project/environment reference, for example \
payment-service/production. Run `dopbase env list` to see available environments.";

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
  /// Client endpoint for this invocation, overriding the saved server and
  /// DOPBASE_URL. This does not apply to local server commands.
  #[arg(long, global = true, value_name = "URL")]
  pub server: Option<String>,
  /// Directory for Dopbase state and configuration.
  #[arg(long, global = true, value_name = "DIR")]
  pub data_dir: Option<PathBuf>,
  /// Print machine-readable JSON instead of human-readable output, where supported.
  #[arg(long, global = true)]
  pub json: bool,
  #[command(subcommand)]
  pub command: Command,
}

impl Cli {
  /// Parse CLI arguments, showing the active command's help when required
  /// input is missing.
  pub fn try_parse_with_help_from<I, T>(arguments: I) -> Result<Self, clap::Error>
  where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
  {
    let mut arguments = arguments.into_iter().map(Into::into).collect::<Vec<_>>();
    match Self::try_parse_from(arguments.clone()) {
      Err(error)
        if matches!(
          error.kind(),
          ErrorKind::MissingRequiredArgument | ErrorKind::MissingSubcommand
        ) =>
      {
        let help_position = arguments
          .iter()
          .position(|argument| argument == OsStr::new("--"))
          .unwrap_or(arguments.len());
        arguments.insert(help_position, OsString::from("--help"));
        Self::try_parse_from(arguments)
      }
      result => result,
    }
  }

  pub fn parse_with_help() -> Self {
    let arguments = std::env::args_os().collect::<Vec<_>>();
    match Self::try_parse_from(arguments.clone()) {
      Ok(cli) => cli,
      Err(error)
        if matches!(
          error.kind(),
          ErrorKind::MissingRequiredArgument | ErrorKind::MissingSubcommand
        ) =>
      {
        let exit_code = error.exit_code();
        let help = Self::try_parse_with_help_from(arguments).unwrap_err();
        help
          .print()
          .unwrap_or_else(|write_error| clap::Error::raw(ErrorKind::Io, write_error).exit());
        std::process::exit(exit_code)
      }
      Err(error) => error.exit(),
    }
  }
}

#[derive(Subcommand, Debug)]
pub enum Command {
  /// Start, stop, inspect, and read logs from a local Dopbase server.
  #[command(after_help = SERVER_HELP)]
  Server {
    #[command(subcommand)]
    command: ServerCommand,
  },
  /// Connect the CLI to a Dopbase server (`client connect <url>`).
  ///
  /// Without a saved server, client commands use http://localhost:8840.
  #[command(after_help = CLIENT_HELP)]
  Client {
    #[command(subcommand)]
    command: ClientCommand,
  },
  /// Authenticate with the active server.
  #[command(after_help = "Examples:\n  dopbase login\n")]
  Login,
  /// Remove the saved credential for the active server.
  #[command(after_help = "Examples:\n  dopbase logout\n")]
  Logout,
  /// Alias for `dopbase client status`.
  #[command(after_help = "Examples:\n  dopbase status\n")]
  Status,
  /// Create a project, its first environment, and import secrets.
  ///
  /// Bootstraps a new project on the server from an existing dotenv file.
  #[command(after_help = "Examples:\n  dopbase init payment-service development --from .env\n")]
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
  #[command(after_help = PROJECT_HELP)]
  Project {
    #[command(subcommand)]
    command: ProjectCommand,
  },
  /// Manage environments inside a project (create, list, rename, delete).
  #[command(after_help = ENV_HELP)]
  Env {
    #[command(subcommand)]
    command: EnvCommand,
  },
  /// Manage secrets in an environment (list, set, get, delete).
  #[command(after_help = SECRET_HELP)]
  Secret {
    #[command(subcommand)]
    command: SecretCommand,
  },
  /// Bulk-import secrets into an environment from a dotenv file.
  ///
  /// Existing keys are kept unless --replace is passed. Use --dry-run to
  /// preview the result without changing anything.
  #[command(
    after_help = "Examples:\n  dopbase import payment-service/production .env.production\n  dopbase import payment-service/production .env.production --dry-run\n"
  )]
  Import {
    #[arg(help = ENVIRONMENT_ARG_HELP)]
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
  /// file. Every export requires interactive password confirmation.
  #[command(
    after_help = "Examples:\n  dopbase export payment-service/production --output .env.production\n  dopbase export payment-service/production --stdout\n"
  )]
  Export {
    #[arg(help = ENVIRONMENT_ARG_HELP)]
    environment: String,
    /// File to write the dotenv output to.
    #[arg(
      long,
      value_name = "FILE",
      conflicts_with = "stdout",
      required_unless_present = "stdout"
    )]
    output: Option<PathBuf>,
    /// Print the dotenv output to stdout instead of a file.
    #[arg(long, conflicts_with = "output", required_unless_present = "output")]
    stdout: bool,
    /// Overwrite the output file if it already exists.
    #[arg(long)]
    force: bool,
  },
  /// Manage CI/runner access tokens for an environment.
  #[command(after_help = TOKEN_HELP)]
  Token {
    #[command(subcommand)]
    command: TokenCommand,
  },
  /// Run a command with an environment's secrets injected as env vars.
  ///
  /// Falls back to DOPBASE_ENV, then the active server's saved default, when
  /// no environment argument is given. Secret values are passed to the child
  /// process only and are never printed. A successful fetch refreshes an
  /// encrypted local cache; if the server is unavailable, the last cache for
  /// the same server, environment, and credential is used. Everything after
  /// `--` is the command to run.
  #[command(
    after_help = "Examples:\n  dopbase run payment-service/development -- npm run dev\n  dopbase run -- npm run dev\n"
  )]
  Run {
    #[arg(help = ENVIRONMENT_ARG_HELP)]
    environment: Option<String>,
    /// Command to run with the injected secrets.
    #[arg(last = true, required = true)]
    command: Vec<String>,
  },
  /// Server administration (password reset).
  #[command(after_help = ADMIN_HELP)]
  Admin {
    #[command(subcommand)]
    command: AdminCommand,
  },
  /// Check GitHub for a newer Dopbase release (informational only).
  #[command(after_help = "Examples:\n  dopbase update\n")]
  Update,
  /// Create an encrypted backup snapshot of the Dopbase instance.
  ///
  /// Backs up projects, environments, secrets, runner tokens, and admin
  /// accounts into an XChaCha20-Poly1305 encrypted .dop archive. Without [name],
  /// a timestamped name is generated automatically. Stored on the server by
  /// default; specify -o/--output to also download and save locally.
  #[command(
    after_help = "Examples:\n  dopbase backup\n  dopbase backup pre-migration --output ./pre-migration.dop\n"
  )]
  Backup {
    /// Optional backup name (default: dopbase_backup_<timestamp>.dop).
    name: Option<String>,
    /// Optional local file path to download and save the backup to.
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,
  },
  /// Restore the instance from an encrypted .dop backup file.
  ///
  /// Restores all projects, environments, secrets, runner tokens, and admin
  /// accounts from the specified backup. If the server is uninitialized,
  /// completes bootstrap restoration. If already initialized, requires
  /// confirmation and administrator credentials.
  #[command(
    after_help = "Examples:\n  dopbase restore ./backup.dop\n  dopbase restore ./backup.dop --setup-token dbs_... --yes\n"
  )]
  Restore {
    /// Path to the .dop backup file to restore.
    path: PathBuf,
    /// Master key file path or 64-character hex string (required when restoring onto a new server).
    #[arg(short, long, value_name = "KEY")]
    key: Option<String>,
    /// Setup token printed by the target server when restoring an uninitialized instance.
    #[arg(long, value_name = "TOKEN")]
    setup_token: Option<String>,
    /// Skip confirmation prompt.
    #[arg(long)]
    yes: bool,
  },
}

#[derive(Subcommand, Debug)]
pub enum ServerCommand {
  /// Run the server in the foreground. Press Ctrl+C to stop it.
  #[command(
    after_help = "Examples:\n  dopbase server start\n  dopbase server start --port 9000\n  dopbase server start --host 0.0.0.0 --public-url https://dopbase.example.com\n"
  )]
  Start(ServerStartArgs),
  /// Start the server in the background on macOS or Linux.
  #[command(
    after_help = "Examples:\n  dopbase server up\n  dopbase server up --port 9000\n  dopbase server up --docs\n"
  )]
  Up(ServerLaunchArgs),
  /// Stop the managed background server.
  #[command(after_help = "Examples:\n  dopbase server down\n  dopbase server down --timeout 30\n")]
  Down {
    /// Seconds to wait for a graceful shutdown before forcing it.
    #[arg(long, default_value_t = 10)]
    timeout: u64,
  },
  /// Show whether the local server is running in the foreground or background.
  #[command(
    after_help = "Examples:\n  dopbase server status\n  dopbase --data-dir /srv/dopbase server status\n"
  )]
  Status,
  /// Print logs written by the managed background server.
  #[command(
    after_help = "Examples:\n  dopbase server logs\n  dopbase server logs --lines 50\n  dopbase server logs --follow\n"
  )]
  Logs {
    /// Number of recent lines to print.
    #[arg(long, default_value_t = 100, value_name = "COUNT")]
    lines: usize,
    /// Continue printing new lines until Ctrl+C.
    #[arg(short, long)]
    follow: bool,
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
  /// the server. Requires --public-url when binding beyond loopback.
  #[arg(long, value_name = "HOST")]
  pub host: Option<String>,
  /// Public URL clients use to reach this server (banners, generated links).
  /// Optional for loopback binds; required with --host 0.0.0.0.
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
#[derive(Subcommand, Debug)]
pub enum ClientCommand {
  /// Validate a server URL and save it as the active server.
  ///
  /// Remote servers must use HTTPS. Accepts the `local` alias to return to
  /// the implicit local default (http://localhost:8840). Changing servers requires
  /// interactive confirmation, stops the current managed background server,
  /// clears the saved CLI session and default, and then requires a new login.
  #[command(
    after_help = "Examples:\n  dopbase client connect https://dopbase.example.com\n  dopbase client connect local\n"
  )]
  Connect {
    /// Server URL to save, or `local` to use http://localhost:8840.
    server_url: String,
  },
  /// Show the active server, connection state, login, and default environment.
  #[command(after_help = "Examples:\n  dopbase client status\n  dopbase --json client status\n")]
  Status,
}
#[derive(Subcommand, Debug)]
pub enum ProjectCommand {
  /// Create an empty project.
  #[command(after_help = "Examples:\n  dopbase project create payment-service\n")]
  Create {
    /// Project name, unique on the server.
    name: String,
  },
  /// List accessible projects.
  #[command(after_help = "Examples:\n  dopbase project list\n")]
  List,
  /// Show project metadata.
  #[command(after_help = "Examples:\n  dopbase project show payment-service\n")]
  Show {
    /// Project ID or name.
    project: String,
  },
  /// Rename a project.
  #[command(after_help = "Examples:\n  dopbase project rename payment-service payments\n")]
  Rename {
    /// Project ID or name.
    project: String,
    /// New project name.
    new_name: String,
  },
  /// Delete a project with all its environments, secrets, and tokens.
  ///
  /// Asks for confirmation unless --yes is passed.
  #[command(
    after_help = "Examples:\n  dopbase project delete payment-service\n  dopbase project delete payment-service --yes\n"
  )]
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
  /// Set or clear the default environment used by `dopbase run`.
  #[command(
    after_help = "Examples:\n  dopbase env default payment-service/development\n  dopbase env default --clear\n"
  )]
  Default {
    #[arg(
      value_name = "ENVIRONMENT",
      required_unless_present = "clear",
      help = ENVIRONMENT_ARG_HELP
    )]
    environment: Option<String>,
    /// Clear the default environment for the active server.
    #[arg(long, conflicts_with = "environment")]
    clear: bool,
  },
  /// Create an environment inside a project.
  #[command(after_help = "Examples:\n  dopbase env create payment-service production\n")]
  Create {
    /// Project ID or name.
    project: String,
    /// Environment name, unique within the project.
    name: String,
  },
  /// List environments, either for one project or all accessible ones.
  #[command(after_help = "Examples:\n  dopbase env list\n  dopbase env list payment-service\n")]
  List {
    /// Limit the listing to this project (ID or name).
    project: Option<String>,
  },
  /// Show environment metadata.
  #[command(
    after_help = "Examples:\n  dopbase env show payment-service/production\n  dopbase env show env_01ABCDEF\n"
  )]
  Show {
    #[arg(help = ENVIRONMENT_ARG_HELP)]
    environment: String,
  },
  /// Rename an environment.
  #[command(after_help = "Examples:\n  dopbase env rename payment-service/production prod\n")]
  Rename {
    #[arg(help = ENVIRONMENT_ARG_HELP)]
    environment: String,
    /// New environment name.
    new_name: String,
  },
  /// Delete an environment with its secrets and tokens.
  ///
  /// Asks for confirmation unless --yes is passed.
  #[command(
    after_help = "Examples:\n  dopbase env delete payment-service/staging\n  dopbase env delete payment-service/staging --yes\n"
  )]
  Delete {
    #[arg(help = ENVIRONMENT_ARG_HELP)]
    environment: String,
    /// Skip the confirmation prompt (for automation).
    #[arg(long)]
    yes: bool,
  },
}
#[derive(Subcommand, Debug)]
pub enum SecretCommand {
  /// List secret keys in an environment (values are never shown).
  #[command(after_help = SECRET_LIST_HELP)]
  List {
    #[arg(help = ENVIRONMENT_ARG_HELP)]
    environment: String,
  },
  /// Set a secret through a hidden prompt or read it from standard input.
  #[command(after_help = SECRET_SET_HELP)]
  Set {
    #[arg(help = ENVIRONMENT_ARG_HELP)]
    environment: String,
    /// Secret key name.
    key: String,
    /// Read the value from standard input instead of prompting for it.
    #[arg(long)]
    stdin: bool,
  },
  /// Show secret metadata, or print its value with --reveal.
  #[command(after_help = SECRET_GET_HELP)]
  Get {
    #[arg(help = ENVIRONMENT_ARG_HELP)]
    environment: String,
    /// Secret key name.
    key: String,
    /// Print the actual value after interactive password confirmation.
    #[arg(long)]
    reveal: bool,
  },
  /// Delete a secret from an environment.
  ///
  /// Asks for confirmation unless --yes is passed.
  #[command(after_help = SECRET_DELETE_HELP)]
  Delete {
    #[arg(help = ENVIRONMENT_ARG_HELP)]
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
  #[command(
    after_help = "Examples:\n  dopbase token create payment-service/production --name deploy\n"
  )]
  Create {
    #[arg(help = ENVIRONMENT_ARG_HELP)]
    environment: String,
    /// Display name for the token.
    #[arg(long)]
    name: String,
    /// Token role.
    #[arg(long, default_value = "runner")]
    role: String,
  },
  /// List tokens for an environment.
  #[command(after_help = "Examples:\n  dopbase token list payment-service/production\n")]
  List {
    #[arg(help = ENVIRONMENT_ARG_HELP)]
    environment: String,
  },
  /// Revoke a token by ID.
  #[command(after_help = "Examples:\n  dopbase token revoke tok_01ABCDEF\n")]
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
  #[command(after_help = "Examples:\n  dopbase admin reset-password admin@example.com\n")]
  ResetPassword {
    /// Email of the account to reset.
    email: String,
    /// Server config file to load (server.toml).
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,
    /// File containing the server master key.
    #[arg(long, value_name = "FILE")]
    master_key_file: Option<PathBuf>,
  },
}
