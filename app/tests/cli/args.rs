use app::cli::args::{Cli, Command, ServerCommand};
use clap::{CommandFactory, Parser, error::ErrorKind};
use std::process::Command as ProcessCommand;

fn contextual_help(arguments: &[&str]) -> String {
  let error = Cli::try_parse_with_help_from(arguments).unwrap_err();
  assert!(
    matches!(
      error.kind(),
      ErrorKind::DisplayHelp | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    ),
    "unexpected error for {arguments:?}: {error}"
  );
  error.to_string()
}

#[test]
fn parses_every_v0_1_command_shape() {
  let commands: &[&[&str]] = &[
    &["dopbase", "server", "start"],
    &["dopbase", "server", "start", "--data-dir", "/tmp/dopbase"],
    &["dopbase", "server", "start", "--docs"],
    &["dopbase", "server", "start", "--no-docs"],
    &["dopbase", "server", "up"],
    &["dopbase", "server", "up", "--docs"],
    &["dopbase", "server", "start", "--port", "8840"],
    &[
      "dopbase", "server", "up", "--port", "9000", "--host", "0.0.0.0",
    ],
    &[
      "dopbase",
      "server",
      "start",
      "--host",
      "localhost",
      "--port",
      "9000",
    ],
    &["dopbase", "server", "down"],
    &["dopbase", "server", "down", "--timeout", "30"],
    &["dopbase", "--data-dir", "/tmp/dopbase", "server", "status"],
    &["dopbase", "--json", "server", "status"],
    &["dopbase", "server", "logs", "--lines", "50"],
    &["dopbase", "server", "logs", "--watch"],
    &["dopbase", "server", "logs", "-w"],
    &["dopbase", "server", "logs", "--clean"],
    &["dopbase", "server", "logs", "--clean", "--watch"],
    &["dopbase", "client", "connect", "http://localhost:8840"],
    &["dopbase", "login"],
    &["dopbase", "logout"],
    &["dopbase", "status"],
    &["dopbase", "client", "status"],
    &["dopbase", "init", "billing", "production", "--from", ".env"],
    &["dopbase", "project", "create", "billing"],
    &["dopbase", "project", "list"],
    &["dopbase", "project", "show", "billing"],
    &["dopbase", "project", "rename", "billing", "payments"],
    &["dopbase", "project", "delete", "billing", "--yes"],
    &["dopbase", "env", "create", "billing", "production"],
    &["dopbase", "env", "list", "billing"],
    &["dopbase", "env", "show", "billing/production"],
    &["dopbase", "env", "default", "billing/production"],
    &["dopbase", "env", "default", "--clear"],
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
    &["dopbase", "run", "env_482731", "--", "printenv"],
    &["dopbase", "admin", "reset-password", "admin@example.com"],
    &["dopbase", "admin", "factory-reset"],
    &["dopbase", "update"],
    &["dopbase", "backup"],
    &["dopbase", "backup", "my-backup"],
    &["dopbase", "backup", "--output", "/tmp/backup.dop"],
    &[
      "dopbase",
      "backup",
      "my-backup",
      "--output",
      "/tmp/backup.dop",
    ],
    &["dopbase", "restore", "/tmp/backup.dop"],
    &["dopbase", "restore", "/tmp/backup.dop", "--yes"],
    &["dopbase", "--json", "project", "list"],
    &["dopbase", "--data-dir", "/tmp/dopbase", "status"],
  ];

  for command in commands {
    Cli::try_parse_from(*command)
      .unwrap_or_else(|error| panic!("failed to parse {command:?}: {error}"));
  }
}

#[test]
fn server_logs_clean_flag_parses() {
  let cli = Cli::try_parse_from(["dopbase", "server", "logs", "--clean"]).unwrap();
  let Command::Server {
    command: ServerCommand::Logs {
      lines,
      clean,
      watch,
    },
  } = cli.command
  else {
    panic!("expected server logs");
  };
  assert_eq!(lines, 100);
  assert!(clean);
  assert!(!watch);
}

#[test]
fn server_logs_rejects_follow_flags() {
  assert!(Cli::try_parse_from(["dopbase", "server", "logs", "--follow"]).is_err());
  assert!(Cli::try_parse_from(["dopbase", "server", "logs", "-f"]).is_err());
}

#[test]
fn default_environment_requires_a_value_or_clear() {
  assert!(Cli::try_parse_from(["dopbase", "env", "default"]).is_err());
  assert!(
    Cli::try_parse_from(["dopbase", "env", "default", "billing/production", "--clear",]).is_err()
  );
}

#[test]
fn status_replaces_config_command() {
  let cli = Cli::try_parse_from(["dopbase", "status"]).unwrap();
  assert!(matches!(cli.command, Command::Status));
  assert!(Cli::try_parse_from(["dopbase", "config"]).is_err());
}

#[test]
fn docs_flags_last_flag_wins() {
  let cli = Cli::try_parse_from(["dopbase", "server", "start", "--docs", "--no-docs"]).unwrap();
  let Command::Server {
    command: ServerCommand::Start(args),
  } = cli.command
  else {
    panic!("expected server start");
  };
  assert_eq!(args.launch.docs(), Some(false));
  let cli = Cli::try_parse_from(["dopbase", "server", "up", "--no-docs", "--docs"]).unwrap();
  let Command::Server {
    command: ServerCommand::Up(args),
  } = cli.command
  else {
    panic!("expected server up");
  };
  assert_eq!(args.docs(), Some(true));
  let cli = Cli::try_parse_from(["dopbase", "server", "start"]).unwrap();
  let Command::Server {
    command: ServerCommand::Start(args),
  } = cli.command
  else {
    panic!("expected server start");
  };
  assert_eq!(args.launch.docs(), None);
}

#[test]
fn port_and_host_flags_parse() {
  let cli = Cli::try_parse_from([
    "dopbase", "server", "start", "--port", "9000", "--host", "0.0.0.0",
  ])
  .unwrap();
  let Command::Server {
    command: ServerCommand::Start(args),
  } = cli.command
  else {
    panic!("expected server start");
  };
  assert_eq!(args.launch.port, Some(9000));
  assert_eq!(args.launch.host.as_deref(), Some("0.0.0.0"));
}

#[test]
fn removed_server_forms_are_rejected() {
  for arguments in [
    vec!["dopbase", "serve"],
    vec!["dopbase", "stop"],
    vec!["dopbase", "server", "run"],
    vec!["dopbase", "server", "start", "--background"],
    vec![
      "dopbase",
      "server",
      "start",
      "--bind-address",
      "127.0.0.1:9000",
    ],
    vec![
      "dopbase",
      "server",
      "start",
      "--database-url",
      "sqlite://other.db",
    ],
  ] {
    assert!(Cli::try_parse_from(arguments).is_err());
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

#[test]
fn top_level_help_lists_common_server_options() {
  let help = Cli::command().render_long_help().to_string();
  assert!(help.contains("Common server options:"), "{help}");
  assert!(help.contains("--host <HOST>"), "{help}");
  assert!(help.contains("--port <PORT>"), "{help}");
  assert!(!help.contains("--background"), "{help}");
}

#[test]
fn every_command_help_has_examples() {
  fn check(
    command: &mut clap::Command,
    parent: &str,
  ) {
    for subcommand in command.get_subcommands_mut() {
      if subcommand.get_name() == "help" {
        continue;
      }
      let path = format!("{parent} {}", subcommand.get_name());
      let help = subcommand.render_long_help().to_string();
      assert!(help.contains("Examples:"), "{path}: {help}");
      check(subcommand, &path);
    }
  }

  check(&mut Cli::command(), "dopbase");
}

#[test]
fn every_visible_argument_has_a_description() {
  fn check(
    command: &clap::Command,
    parent: &str,
  ) {
    for argument in command.get_arguments() {
      if argument.is_hide_set() || matches!(argument.get_id().as_str(), "help" | "version") {
        continue;
      }
      assert!(
        argument.get_help().is_some(),
        "{parent}: argument '{}' has no help text",
        argument.get_id()
      );
    }

    for subcommand in command.get_subcommands() {
      if subcommand.get_name() == "help" {
        continue;
      }
      check(subcommand, &format!("{parent} {}", subcommand.get_name()));
    }
  }

  check(&Cli::command(), "dopbase");
}

#[test]
fn missing_subcommands_show_contextual_help() {
  let cases: &[(&[&str], &str)] = &[
    (&["dopbase"], "Quickstart:"),
    (&["dopbase", "server"], "dopbase server logs --watch"),
    (&["dopbase", "client"], "dopbase client connect local"),
    (
      &["dopbase", "project"],
      "dopbase project create payment-service",
    ),
    (
      &["dopbase", "env"],
      "dopbase env show payment-service/production",
    ),
    (
      &["dopbase", "secret"],
      "dopbase secret list payment-service/production",
    ),
    (
      &["dopbase", "token"],
      "dopbase token create payment-service/production --name deploy",
    ),
    (
      &["dopbase", "admin"],
      "dopbase admin reset-password admin@example.com",
    ),
  ];

  for (arguments, expected) in cases {
    let help = contextual_help(arguments);
    assert!(help.contains("Usage:"), "{arguments:?}: {help}");
    assert!(help.contains(expected), "{arguments:?}: {help}");
  }
}

#[test]
fn incomplete_secret_commands_show_examples_and_environment_help() {
  let cases: &[(&[&str], &[&str])] = &[
    (
      &["dopbase", "secret", "list"],
      &[
        "Usage: dopbase secret list",
        "payment-service/production",
        "env_482731",
        "dopbase env list",
      ],
    ),
    (
      &["dopbase", "secret", "set"],
      &[
        "Usage: dopbase secret set",
        "dopbase secret set payment-service/production API_KEY",
        "prompts for the value without showing it on screen",
      ],
    ),
    (
      &["dopbase", "secret", "set", "payment-service/production"],
      &[
        "Usage: dopbase secret set",
        "dopbase secret set payment-service/production API_KEY",
      ],
    ),
    (
      &["dopbase", "secret", "get"],
      &[
        "Usage: dopbase secret get",
        "dopbase secret get payment-service/production API_KEY --reveal",
      ],
    ),
    (
      &["dopbase", "secret", "delete"],
      &[
        "Usage: dopbase secret delete",
        "dopbase secret delete payment-service/production API_KEY --yes",
      ],
    ),
  ];

  for (arguments, expected) in cases {
    let help = contextual_help(arguments);
    assert!(help.contains("Examples:"), "{arguments:?}: {help}");
    assert!(
      help.contains("project/environment reference"),
      "{arguments:?}: {help}"
    );
    for text in *expected {
      assert!(help.contains(text), "{arguments:?}: {help}");
    }
  }
}

#[test]
fn other_incomplete_commands_show_full_help() {
  let cases: &[&[&str]] = &[
    &["dopbase", "client", "connect"],
    &["dopbase", "init"],
    &["dopbase", "project", "rename", "payment-service"],
    &["dopbase", "env", "show"],
    &["dopbase", "import"],
    &["dopbase", "export", "payment-service/production"],
    &["dopbase", "token", "create", "payment-service/production"],
    &["dopbase", "run"],
    &["dopbase", "run", "--"],
    &["dopbase", "admin", "reset-password"],
    &["dopbase", "restore"],
  ];

  for arguments in cases {
    let help = contextual_help(arguments);
    assert!(help.contains("Usage:"), "{arguments:?}: {help}");
    assert!(help.contains("Example"), "{arguments:?}: {help}");
  }
}

#[test]
fn non_missing_argument_errors_are_preserved() {
  let unknown =
    Cli::try_parse_with_help_from(["dopbase", "secret", "list", "--unknown"]).unwrap_err();
  assert_eq!(unknown.kind(), ErrorKind::UnknownArgument);

  let conflict = Cli::try_parse_with_help_from([
    "dopbase",
    "export",
    "payment-service/production",
    "--output",
    ".env",
    "--stdout",
  ])
  .unwrap_err();
  assert_eq!(conflict.kind(), ErrorKind::ArgumentConflict);
}

#[test]
fn binary_prints_contextual_help_and_keeps_the_usage_error_exit_code() {
  let output = ProcessCommand::new(env!("CARGO_BIN_EXE_dopbase"))
    .args(["secret", "list"])
    .output()
    .unwrap();

  assert_eq!(output.status.code(), Some(2));
  let help = String::from_utf8(output.stdout).unwrap();
  assert!(help.contains("Usage: dopbase secret list"), "{help}");
  assert!(
    help.contains("dopbase secret list payment-service/production"),
    "{help}"
  );
  assert!(output.stderr.is_empty());
}

#[test]
fn server_status_reports_a_stopped_data_directory() {
  let directory = tempfile::TempDir::new().unwrap();
  let output = ProcessCommand::new(env!("CARGO_BIN_EXE_dopbase"))
    .args([
      "--data-dir",
      directory.path().to_str().unwrap(),
      "--json",
      "server",
      "status",
    ])
    .output()
    .unwrap();

  assert_eq!(output.status.code(), Some(1));
  let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
  assert_eq!(value["status"], "stopped");
  assert_eq!(value["mode"], serde_json::Value::Null);
  assert!(!directory.path().join("dopbase.db.lock").exists());
}

#[test]
fn server_commands_reject_inapplicable_global_options() {
  let directory = tempfile::TempDir::new().unwrap();
  let data_dir = directory.path().to_str().unwrap();
  let cases: &[&[&str]] = &[
    &[
      "--data-dir",
      data_dir,
      "--server",
      "http://localhost:8840",
      "server",
      "status",
    ],
    &["--data-dir", data_dir, "--json", "server", "start"],
    &[
      "--data-dir",
      data_dir,
      "--json",
      "server",
      "logs",
      "--watch",
    ],
  ];

  for arguments in cases {
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_dopbase"))
      .args(*arguments)
      .output()
      .unwrap();
    assert!(!output.status.success(), "{arguments:?}");
  }
}
