use clap::{ArgAction, Parser, Subcommand, error::ErrorKind};
use std::{
  ffi::{OsStr, OsString},
  io::{self, Write},
  path::PathBuf,
};

use crate::{
  cli::commands::{
    auth, client, environment, export, import, init, project, run, secret, server, token,
  },
  constants::help::*,
};

pub use auth::LoginArgs;
pub use client::ClientCommand;
pub use environment::EnvCommand;
pub use export::ExportArgs;
pub use import::ImportArgs;
pub use init::InitArgs;
pub use project::ProjectCommand;
pub use run::RunArgs;
pub use secret::SecretCommand;
pub use server::{ServerCommand, ServerLaunchArgs, ServerStartArgs};
pub use token::TokenCommand;

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
  #[command(after_help = client::HELP)]
  Client {
    #[command(subcommand)]
    command: ClientCommand,
  },
  /// Authenticate with the active server.
  #[command(after_help = auth::LOGIN_HELP)]
  Login(LoginArgs),
  /// Remove the saved credential for the active server.
  #[command(after_help = auth::LOGOUT_HELP)]
  Logout,
  /// Alias for `dopbase client status`.
  #[command(after_help = client::STATUS_ALIAS_HELP)]
  Status,
  /// Create a project, its first environment, and import secrets.
  #[command(after_help = init::HELP)]
  Init(InitArgs),
  /// Manage projects (create, list, show, rename, delete).
  #[command(after_help = project::HELP)]
  Project {
    #[command(subcommand)]
    command: ProjectCommand,
  },
  /// Manage environments inside a project (create, list, rename, delete).
  #[command(after_help = environment::HELP)]
  Env {
    #[command(subcommand)]
    command: EnvCommand,
  },
  /// Manage secrets in an environment (list, set, get, delete).
  #[command(after_help = secret::HELP)]
  Secret {
    #[command(subcommand)]
    command: SecretCommand,
  },
  /// Bulk-import secrets into an environment from a dotenv, JSON, or YAML source.
  ///
  /// Existing keys are kept unless --replace is passed. Use --dry-run to
  /// preview the result without changing anything.
  #[command(after_help = import::HELP)]
  Import(ImportArgs),
  /// Export an environment's secrets as dotenv, JSON, or YAML.
  ///
  /// Requires --output <FILE> or --stdout. --force overwrites an existing
  /// file. Every export requires interactive password confirmation.
  #[command(after_help = export::HELP)]
  Export(ExportArgs),
  /// Manage CI/runner access tokens for an environment.
  #[command(after_help = token::HELP)]
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
  #[command(after_help = run::HELP)]
  Run(RunArgs),
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
