use app::cli::args::{Cli, Command};
use clap::Parser;

#[test]
fn parses_every_v0_1_command_shape() {
  let commands: &[&[&str]] = &[
    &["dopbase", "serve"],
    &["dopbase", "serve", "--data-dir", "/tmp/dopbase"],
    &["dopbase", "serve", "--docs"],
    &["dopbase", "serve", "--no-docs"],
    &["dopbase", "serve", "--background"],
    &["dopbase", "serve", "--background", "--docs"],
    &["dopbase", "serve", "--port", "8840"],
    &["dopbase", "serve", "--port", "9000", "--host", "0.0.0.0"],
    &["dopbase", "serve", "--host", "localhost", "--port", "9000"],
    &["dopbase", "serve", "--bind-address", "127.0.0.1:8840"],
    &["dopbase", "stop"],
    &["dopbase", "stop", "--timeout", "30"],
    &["dopbase", "--data-dir", "/tmp/dopbase", "stop"],
    &["dopbase", "--json", "stop"],
    &["dopbase", "client", "connect", "http://localhost:8840"],
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
    &["dopbase", "update"],
    &["dopbase", "--json", "project", "list"],
    &["dopbase", "--data-dir", "/tmp/dopbase", "config"],
  ];

  for command in commands {
    Cli::try_parse_from(*command)
      .unwrap_or_else(|error| panic!("failed to parse {command:?}: {error}"));
  }
}

#[test]
fn docs_flags_last_flag_wins() {
  let cli = Cli::try_parse_from(["dopbase", "serve", "--docs", "--no-docs"]).unwrap();
  let Command::Serve(args) = cli.command else {
    panic!("expected serve");
  };
  assert_eq!(args.docs(), Some(false));
  let cli = Cli::try_parse_from(["dopbase", "serve", "--no-docs", "--docs"]).unwrap();
  let Command::Serve(args) = cli.command else {
    panic!("expected serve");
  };
  assert_eq!(args.docs(), Some(true));
  let cli = Cli::try_parse_from(["dopbase", "serve"]).unwrap();
  let Command::Serve(args) = cli.command else {
    panic!("expected serve");
  };
  assert_eq!(args.docs(), None);
}

#[test]
fn port_and_host_flags_parse() {
  let cli =
    Cli::try_parse_from(["dopbase", "serve", "--port", "9000", "--host", "0.0.0.0"]).unwrap();
  let Command::Serve(args) = cli.command else {
    panic!("expected serve");
  };
  assert_eq!(args.port, Some(9000));
  assert_eq!(args.host.as_deref(), Some("0.0.0.0"));
}

#[test]
fn port_conflicts_with_bind_address() {
  assert!(
    Cli::try_parse_from([
      "dopbase",
      "serve",
      "--port",
      "9000",
      "--bind-address",
      "127.0.0.1:9000",
    ])
    .is_err()
  );
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
