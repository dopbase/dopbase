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
}

#[derive(Args, Debug, Default)]
pub struct ServeArgs {
    #[arg(long)]
    pub config: Option<PathBuf>,
    #[arg(long)]
    pub bind_address: Option<String>,
    #[arg(long)]
    pub public_url: Option<String>,
    #[arg(long)]
    pub database_url: Option<String>,
    #[arg(long)]
    pub shutdown_grace_seconds: Option<u64>,
    #[arg(long)]
    pub master_key_file: Option<PathBuf>,
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

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::Parser;

    #[test]
    fn parses_every_v0_1_command_shape() {
        let commands: &[&[&str]] = &[
            &["dopbase", "serve"],
            &["dopbase", "serve", "--data-dir", "/tmp/dopbase"],
            &["dopbase", "client", "connect", "http://localhost:8376"],
            &["dopbase", "login"],
            &["dopbase", "logout"],
            &["dopbase", "config"],
            &["dopbase", "init", "billing", "production", "--from", ".env"],
            &["dopbase", "project", "create", "billing"],
            &["dopbase", "project", "list"],
            &["dopbase", "project", "show", "billing"],
            &["dopbase", "project", "rename", "billing", "payments"],
            &["dopbase", "project", "delete", "billing", "--yes"],
            &["dopbase", "env", "create", "billing", "production"],
            &["dopbase", "env", "list", "billing"],
            &["dopbase", "env", "show", "billing/production"],
            &["dopbase", "env", "rename", "env_01", "staging"],
            &["dopbase", "env", "delete", "env_01", "--yes"],
            &["dopbase", "secret", "list", "billing/production"],
            &[
                "dopbase",
                "secret",
                "set",
                "billing/production",
                "API_KEY",
                "--stdin",
            ],
            &[
                "dopbase",
                "secret",
                "get",
                "billing/production",
                "API_KEY",
                "--reveal",
            ],
            &[
                "dopbase",
                "secret",
                "delete",
                "billing/production",
                "API_KEY",
                "--yes",
            ],
            &[
                "dopbase",
                "import",
                "billing/production",
                ".env",
                "--dry-run",
            ],
            &["dopbase", "export", "billing/production", "--stdout"],
            &[
                "dopbase",
                "token",
                "create",
                "billing/production",
                "--name",
                "server",
            ],
            &["dopbase", "token", "list", "billing/production"],
            &["dopbase", "token", "revoke", "tok_01"],
            &["dopbase", "run", "billing/production", "--", "printenv"],
            &["dopbase", "admin", "reset-password", "admin@example.com"],
            &["dopbase", "--json", "project", "list"],
            &["dopbase", "--data-dir", "/tmp/dopbase", "config"],
        ];

        for command in commands {
            Cli::try_parse_from(*command)
                .unwrap_or_else(|error| panic!("failed to parse {command:?}: {error}"));
        }
    }

    #[test]
    fn rejects_conflicting_file_options() {
        assert!(
            Cli::try_parse_from([
                "dopbase",
                "import",
                "billing/production",
                ".env",
                "--dry-run",
                "--replace",
            ])
            .is_err()
        );
        assert!(
            Cli::try_parse_from([
                "dopbase",
                "export",
                "billing/production",
                "--output",
                "secrets.env",
                "--stdout",
            ])
            .is_err()
        );
    }
}
