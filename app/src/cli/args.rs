use clap::{ArgAction, Parser, Subcommand, error::ErrorKind};
use std::{
  ffi::{OsStr, OsString},
  io::{self, Write},
  path::PathBuf,
};

use crate::{
  cli::{
    commands::server,
    environment_target,
    environment_target::EnvironmentTarget,
    secret_format::SecretFormat,
  },
  constants::help::*,
};

pub use server::{ServerCommand, ServerLaunchArgs, ServerStartArgs};

#[derive(Parser, Debug)]
#[command(
  name = "dopbase",
  version,
  disable_version_flag = true,
  about = "Secrets management in one binary",
  long_about = "Dopbase keeps application secrets in one binary: run a server, store \
secrets per project and environment, and inject them into any command with `run`.",
  after_help = AFTER_HELP
)]
pub struct Cli {
  /// Print the installed Dopbase version.
  #[arg(
    short = 'v',
    visible_short_alias = 'V',
    long = "version",
    action = ArgAction::Version
  )]
  version: Option<bool>,
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
      Err(error) if error.kind() == ErrorKind::DisplayVersion => Self::exit_with_version(),
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

  fn exit_with_version() -> ! {
    let mut stdout = io::stdout().lock();
    if let Err(write_error) = writeln!(stdout, "v{}", env!("CARGO_PKG_VERSION")) {
      clap::Error::raw(ErrorKind::Io, write_error).exit();
    }
    drop(stdout);
    std::process::exit(0)
  }
}

#[derive(Subcommand, Debug)]
pub enum Command {
  /// Start, stop, inspect, and read logs from a local Dopbase server.
  #[command(after_help = server::HELP)]
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
  #[command(after_help = LOGIN_HELP)]
  Login {
    /// Save a runner token instead of signing in with email and password.
    ///
    /// In a terminal, Dopbase prompts for the token without echoing it. When
    /// standard input is piped, Dopbase reads the token from standard input.
    #[arg(long)]
    token: bool,
  },
  /// Remove the saved credential for the active server.
  #[command(after_help = LOGOUT_HELP)]
  Logout,
  /// Alias for `dopbase client status`.
  #[command(after_help = STATUS_HELP)]
  Status,
  /// Create a project, its first environment, and import secrets.
  #[command(after_help = INIT_HELP)]
  Init {
    /// New project and first environment, written as PROJECT_NAME/ENVIRONMENT_NAME.
    #[arg(
      value_name = "PROJECT_NAME/ENVIRONMENT_NAME",
      value_parser = environment_target::parse_init
    )]
    target: EnvironmentTarget,
    /// Secret file to import, or - to read from stdin.
    #[arg(long, value_name = "FILE")]
    from: PathBuf,
    /// Input format. Required for stdin; otherwise inferred from the filename.
    #[arg(long, value_enum)]
    format: Option<SecretFormat>,
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
  /// Bulk-import secrets into an environment from a dotenv, JSON, or YAML source.
  ///
  /// Existing keys are kept unless --replace is passed. Use --dry-run to
  /// preview the result without changing anything.
  #[command(after_help = IMPORT_HELP)]
  Import {
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
    environment: String,
    /// Secret file to import, or - to read from stdin.
    path: PathBuf,
    /// Input format. Required for stdin; otherwise inferred from the filename.
    #[arg(long, value_enum)]
    format: Option<SecretFormat>,
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
  /// Export an environment's secrets as dotenv, JSON, or YAML.
  ///
  /// Requires --output <FILE> or --stdout. --force overwrites an existing
  /// file. Every export requires interactive password confirmation.
  #[command(after_help = EXPORT_HELP)]
  Export {
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
    environment: String,
    /// File to write. The format is inferred from its filename by default.
    #[arg(
      long,
      value_name = "FILE",
      conflicts_with = "stdout",
      required_unless_present = "stdout"
    )]
    output: Option<PathBuf>,
    /// Print plaintext secrets to stdout. Defaults to dotenv format.
    #[arg(long, conflicts_with = "output", required_unless_present = "output")]
    stdout: bool,
    /// Output format. Overrides filename inference.
    #[arg(long, value_enum)]
    format: Option<SecretFormat>,
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
  /// encrypted local cache. If the server is unavailable, the last cache for
  /// the same server, environment, and credential is used. Everything after
  /// `--` is the command to run.
  #[command(after_help = RUN_HELP)]
  Run {
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
    environment: Option<String>,
    /// Runner token for this invocation. Overrides DOPBASE_TOKEN and the saved credential.
    #[arg(short = 't', long, value_name = "TOKEN")]
    token: Option<String>,
    /// Command to run with the injected secrets.
    #[arg(last = true, required = true)]
    command: Vec<String>,
  },
  /// Offline server administration.
  #[command(after_help = ADMIN_HELP)]
  Admin {
    #[command(subcommand)]
    command: AdminCommand,
  },
  /// Check GitHub for a newer Dopbase release (informational only).
  #[command(after_help = UPDATE_HELP)]
  Update,
  /// Create an encrypted backup snapshot of the Dopbase instance.
  ///
  /// Backs up projects, environments, secrets, runner tokens, and admin
  /// accounts into an XChaCha20-Poly1305 encrypted .dop archive. Without [name],
  /// a timestamped name is generated automatically. Stored on the server by
  /// default. Specify -o/--output to also download and save locally.
  #[command(after_help = BACKUP_HELP)]
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
  #[command(after_help = RESTORE_HELP)]
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
pub enum ClientCommand {
  /// Validate a server URL and save it as the active server.
  ///
  /// Domains must include http:// or https://. Bare IP addresses use HTTP.
  /// Accepts the `local` alias to return to the implicit local default
  /// (http://localhost:8840). Changing servers requires interactive
  /// confirmation, stops the current managed background server, clears the
  /// saved CLI session and default, and then requires a new login.
  #[command(after_help = CLIENT_CONNECT_HELP)]
  Connect {
    /// Server URL or bare IP to save, or `local` to use http://localhost:8840.
    server_url: String,
  },
  /// Show the active server, connection state, login, and default environment.
  #[command(after_help = CLIENT_STATUS_HELP)]
  Status,
}
#[derive(Subcommand, Debug)]
pub enum ProjectCommand {
  /// Create an empty project.
  #[command(after_help = PROJECT_CREATE_HELP)]
  Create {
    /// Project name, unique on the server.
    #[arg(value_name = "PROJECT_NAME")]
    name: String,
  },
  /// List accessible projects.
  #[command(after_help = PROJECT_LIST_HELP)]
  List,
  /// Show project metadata.
  #[command(after_help = PROJECT_SHOW_HELP)]
  Show {
    /// Project ID or name.
    #[arg(value_name = "PROJECT_REF")]
    project: String,
  },
  /// Rename a project.
  #[command(after_help = PROJECT_RENAME_HELP)]
  Rename {
    /// Project ID or name.
    #[arg(value_name = "PROJECT_REF")]
    project: String,
    /// New project name.
    #[arg(value_name = "NEW_PROJECT_NAME")]
    new_name: String,
  },
  /// Delete a project with all its environments, secrets, and tokens.
  ///
  /// Asks for confirmation unless --yes is passed.
  #[command(after_help = PROJECT_DELETE_HELP)]
  Delete {
    /// Project ID or name.
    #[arg(value_name = "PROJECT_REF")]
    project: String,
    /// Skip the confirmation prompt (for automation).
    #[arg(long)]
    yes: bool,
  },
}
#[derive(Subcommand, Debug)]
pub enum EnvCommand {
  /// Set or clear the default environment used by `dopbase run`.
  #[command(after_help = ENV_DEFAULT_HELP)]
  Default {
    #[arg(
      value_name = "ENVIRONMENT_REF",
      required_unless_present = "clear",
      help = ENVIRONMENT_ARG_HELP
    )]
    environment: Option<String>,
    /// Clear the default environment for the active server.
    #[arg(long, conflicts_with = "environment")]
    clear: bool,
  },
  /// Create an environment inside a project.
  #[command(after_help = ENV_CREATE_HELP)]
  Create {
    /// Existing project and new environment, written as PROJECT_REF/ENVIRONMENT_NAME.
    #[arg(
      value_name = "PROJECT_REF/ENVIRONMENT_NAME",
      value_parser = environment_target::parse_create
    )]
    target: EnvironmentTarget,
  },
  /// List environments, either for one project or all accessible ones.
  #[command(after_help = ENV_LIST_HELP)]
  List {
    /// Limit the listing to this project (ID or name).
    #[arg(value_name = "PROJECT_REF")]
    project: Option<String>,
  },
  /// Show environment metadata.
  #[command(after_help = ENV_SHOW_HELP)]
  Show {
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
    environment: String,
  },
  /// Rename an environment.
  #[command(after_help = ENV_RENAME_HELP)]
  Rename {
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
    environment: String,
    /// New environment name.
    #[arg(value_name = "NEW_ENVIRONMENT_NAME")]
    new_name: String,
  },
  /// Delete an environment with its secrets and tokens.
  ///
  /// Asks for confirmation unless --yes is passed.
  #[command(after_help = ENV_DELETE_HELP)]
  Delete {
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
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
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
    environment: String,
  },
  /// Set a secret through a masked prompt or read it from standard input.
  #[command(after_help = SECRET_SET_HELP)]
  Set {
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
    environment: String,
    /// Secret key name.
    key: String,
    /// Read until EOF. In a terminal, finish with Ctrl+D (Ctrl+Z then Enter on Windows).
    #[arg(long)]
    stdin: bool,
  },
  /// Show secret metadata, or print its value with --reveal.
  #[command(after_help = SECRET_GET_HELP)]
  Get {
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
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
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
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
  /// The token value is shown once at creation. Pass it to client commands
  /// via the DOPBASE_TOKEN environment variable.
  #[command(after_help = TOKEN_CREATE_HELP)]
  Create {
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
    environment: String,
    /// Display name for the token.
    #[arg(long)]
    name: String,
    /// Token role.
    #[arg(long, default_value = "runner")]
    role: String,
  },
  /// List tokens for an environment.
  #[command(after_help = TOKEN_LIST_HELP)]
  List {
    #[arg(value_name = "ENVIRONMENT_REF", help = ENVIRONMENT_ARG_HELP)]
    environment: String,
  },
  /// Revoke a token by ID.
  #[command(after_help = TOKEN_REVOKE_HELP)]
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
  #[command(after_help = ADMIN_RESET_PASSWORD_HELP)]
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
  /// Remove a local instance from the server machine.
  ///
  /// Removes the entire Dopbase data directory from its active location. A ZIP
  /// backup is created first unless --no-backup is passed. The server must be
  /// stopped and an interactive root password confirmation is required.
  #[command(after_help = ADMIN_FACTORY_RESET_HELP)]
  FactoryReset {
    /// Server config file to load (server.toml).
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,
    /// Permanently reset the instance without creating a ZIP backup.
    #[arg(long)]
    no_backup: bool,
  },
}
