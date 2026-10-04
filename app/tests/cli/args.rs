use app::cli::args::{
  AdminCommand, Cli, Command, ExportArgs, ImportArgs, InitArgs, RunArgs, ServerCommand,
};
use app::cli::secret_format::{ExportFormat, SecretFormat};
use app::constants::config::executable_environment_names;
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
fn factory_reset_parses_no_backup() {
  let cli = Cli::try_parse_from(["dopbase", "admin", "factory-reset", "--no-backup"]).unwrap();
  let Command::Admin {
    command: AdminCommand::FactoryReset { no_backup, .. },
  } = cli.command
  else {
    panic!("expected factory-reset command");
  };
  assert!(no_backup);
}

#[test]
fn run_token_must_appear_before_the_child_command_separator() {
  let cli = Cli::try_parse_from([
    "dopbase",
    "run",
    "env_482731",
    "--token",
    "dbs_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "--",
    "printenv",
    "--token",
    "child-value",
  ])
  .unwrap();
  let Command::Run(RunArgs { token, command, .. }) = cli.command else {
    panic!("expected run command");
  };
  assert_eq!(
    token.as_deref(),
    Some("dbs_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")
  );
  assert_eq!(command, ["printenv", "--token", "child-value"]);
}

#[test]
fn run_accepts_the_short_token_flag() {
  let cli = Cli::try_parse_from([
    "dopbase",
    "run",
    "env_482731",
    "-t",
    "dbs_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "--",
    "printenv",
    "-t",
    "child-value",
  ])
  .unwrap();
  let Command::Run(RunArgs { token, command, .. }) = cli.command else {
    panic!("expected run command");
  };
  assert_eq!(
    token.as_deref(),
    Some("dbs_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")
  );
  assert_eq!(command, ["printenv", "-t", "child-value"]);
}

#[test]
fn login_does_not_accept_the_short_token_flag() {
  assert!(Cli::try_parse_from(["dopbase", "login", "-t"]).is_err());
}

#[test]
fn version_flags_print_only_the_prefixed_version() {
  let expected = format!("v{}\n", env!("CARGO_PKG_VERSION"));
  for flag in ["-v", "-V", "--version"] {
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_dopbase"))
      .arg(flag)
      .output()
      .unwrap();
    assert!(output.status.success(), "{flag}: {output:?}");
    assert_eq!(output.stdout, expected.as_bytes(), "{flag}");
    assert!(output.stderr.is_empty(), "{flag}: {output:?}");
  }
}

#[test]
fn global_output_flag_works_before_or_after_the_command() {
  let before = Cli::try_parse_from(["dopbase", "--json", "env", "list", "billing"]).unwrap();
  let after = Cli::try_parse_from(["dopbase", "env", "list", "billing", "--json"]).unwrap();
  assert!(before.json);
  assert!(after.json);
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
  let cli =
    Cli::try_parse_from(["dopbase", "server", "start", "-b", "--no-docs", "--docs"]).unwrap();
  let Command::Server {
    command: ServerCommand::Start(args),
  } = cli.command
  else {
    panic!("expected background server start");
  };
  assert!(args.background);
  assert_eq!(args.launch.docs(), Some(true));
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
    vec!["dopbase", "server", "start", "--background", "--supervised"],
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
fn secret_commands_parse_format_options() {
  let interactive_init = Cli::try_parse_from(["dopbase", "init"]).unwrap();
  assert!(matches!(
    interactive_init.command,
    Command::Init(InitArgs {
      target: None,
      from: None,
      format: None,
    })
  ));

  let init = Cli::try_parse_from([
    "dopbase",
    "init",
    "storefront/development",
    "--from",
    "-",
    "--format",
    "yaml",
  ])
  .unwrap();
  assert!(matches!(
    init.command,
    Command::Init(InitArgs {
      format: Some(SecretFormat::Yaml),
      ..
    })
  ));

  let import = Cli::try_parse_from([
    "dopbase",
    "import",
    "storefront/development",
    "secrets.data",
    "--format",
    "json",
  ])
  .unwrap();
  assert!(matches!(
    import.command,
    Command::Import(ImportArgs {
      format: Some(SecretFormat::Json),
      ..
    })
  ));

  let toml_init = Cli::try_parse_from([
    "dopbase",
    "init",
    "storefront/development",
    "--from",
    "secrets.toml",
    "--format",
    "toml",
  ])
  .unwrap();
  assert!(matches!(
    toml_init.command,
    Command::Init(InitArgs {
      format: Some(SecretFormat::Toml),
      ..
    })
  ));

  let toml_import = Cli::try_parse_from([
    "dopbase",
    "import",
    "storefront/development",
    "secrets.toml",
    "--format",
    "toml",
  ])
  .unwrap();
  assert!(matches!(
    toml_import.command,
    Command::Import(ImportArgs {
      format: Some(SecretFormat::Toml),
      ..
    })
  ));

  let export = Cli::try_parse_from([
    "dopbase",
    "export",
    "storefront/development",
    "--output",
    "secrets.yml",
    "--format",
    "dotenv",
  ])
  .unwrap();
  assert!(matches!(
    export.command,
    Command::Export(ExportArgs {
      format: Some(ExportFormat::Dotenv),
      ..
    })
  ));

  let toml_export = Cli::try_parse_from([
    "dopbase",
    "export",
    "storefront/development",
    "--output",
    "secrets.toml",
    "--format",
    "toml",
  ])
  .unwrap();
  assert!(matches!(
    toml_export.command,
    Command::Export(ExportArgs {
      format: Some(ExportFormat::Toml),
      ..
    })
  ));

  let docker_export = Cli::try_parse_from([
    "dopbase",
    "export",
    "storefront/development",
    "--stdout",
    "--format",
    "docker",
  ])
  .unwrap();
  assert!(matches!(
    docker_export.command,
    Command::Export(ExportArgs {
      format: Some(ExportFormat::Docker),
      ..
    })
  ));

  for command in ["init", "import"] {
    let mut args = vec!["dopbase", command, "storefront/development"];
    if command == "init" {
      args.extend(["--from", ".env"]);
    } else {
      args.push(".env");
    }
    args.extend(["--format", "docker"]);
    assert!(Cli::try_parse_from(args).is_err());
  }

  assert!(
    Cli::try_parse_from([
      "dopbase",
      "init",
      "storefront",
      "development",
      "--from",
      ".env",
    ])
    .is_err()
  );
  assert!(Cli::try_parse_from(["dopbase", "env", "create", "storefront", "development"]).is_err());

  for arguments in [
    vec!["dopbase", "init", "storefront/development"],
    vec!["dopbase", "init", "--from", ".env"],
    vec!["dopbase", "init", "--format", "dotenv"],
  ] {
    assert!(Cli::try_parse_from(arguments).is_err());
  }
}

#[test]
fn validates_qualified_environment_creation_targets() {
  for target in ["storefront/development", "prj_01JTEST/development"] {
    Cli::try_parse_from(["dopbase", "env", "create", target]).unwrap();
  }

  for target in [
    "storefront",
    "/development",
    "storefront/",
    "storefront/dev/extra",
    "storefront/Development",
  ] {
    let error = Cli::try_parse_from(["dopbase", "env", "create", target]).unwrap_err();
    assert_eq!(
      error.kind(),
      ErrorKind::ValueValidation,
      "{target}: {error}"
    );
  }

  for target in ["prj_01JTEST/development", "env_482731/development"] {
    let error = Cli::try_parse_from(["dopbase", "init", target, "--from", ".env"]).unwrap_err();
    assert_eq!(
      error.kind(),
      ErrorKind::ValueValidation,
      "{target}: {error}"
    );
  }
}

#[test]
fn top_level_help_lists_common_server_options() {
  let mut command = Cli::command();
  let version = command
    .get_arguments()
    .find(|argument| argument.get_long() == Some("version"))
    .unwrap();
  assert_eq!(version.get_short(), Some('v'));
  assert!(
    version
      .get_visible_short_aliases()
      .is_some_and(|aliases| aliases.contains(&'V'))
  );
  let help = command.render_long_help().to_string();
  assert!(help.contains("Common server options:"), "{help}");
  assert!(help.contains("--host <HOST>"), "{help}");
  assert!(help.contains("--port <PORT>"), "{help}");
  assert!(help.contains("--background"), "{help}");
}

#[test]
fn top_level_help_lists_every_executable_environment_variable() {
  let help = Cli::command().render_long_help().to_string();
  assert!(help.contains("Environment variables:"), "{help}");
  let environment_help = help
    .split_once("Environment variables:")
    .unwrap()
    .1
    .split_once("Run 'dopbase help <command>'")
    .unwrap()
    .0;
  for name in executable_environment_names() {
    assert!(
      environment_help.contains(name),
      "missing {name} from environment variable help:\n{help}"
    );
  }
  assert!(
    environment_help.contains("Bearer token for a machine runner or AI agent"),
    "{help}"
  );
}

#[test]
fn every_public_command_help_has_examples() {
  fn check(
    command: &mut clap::Command,
    parent: &str,
  ) {
    for subcommand in command.get_subcommands_mut() {
      if subcommand.get_name() == "help" || subcommand.is_hide_set() {
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
      if subcommand.get_name() == "help" || subcommand.is_hide_set() {
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
    (&["dopbase", "cache"], "dopbase cache clean --all --yes"),
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
fn token_create_expiry_flag_accepts_hours_days_and_never() {
  for value in ["1h", "1d", "never"] {
    assert!(
      Cli::try_parse_from([
        "dopbase",
        "token",
        "create",
        "env_123456",
        "--name",
        "deploy",
        "--expires-in",
        value
      ])
      .is_ok(),
      "rejected {value}"
    );
  }
  assert!(
    Cli::try_parse_from([
      "dopbase",
      "token",
      "create",
      "env_123456",
      "--name",
      "deploy",
      "--expires-in",
      "30m"
    ])
    .is_err()
  );
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
        "uses a masked prompt and displays * for each character",
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
      help.contains("PROJECT_REF/ENVIRONMENT_NAME"),
      "{arguments:?}: {help}"
    );
    for text in *expected {
      assert!(help.contains(text), "{arguments:?}: {help}");
    }
  }
}

#[test]
fn resource_parameters_use_consistent_value_names() {
  let cases: &[(&[&str], &str)] = &[
    (
      &["dopbase", "init", "--help"],
      "[PROJECT_NAME/ENVIRONMENT_NAME]",
    ),
    (
      &["dopbase", "project", "create", "--help"],
      "<PROJECT_NAME>",
    ),
    (&["dopbase", "project", "show", "--help"], "<PROJECT_REF>"),
    (
      &["dopbase", "project", "rename", "--help"],
      "<PROJECT_REF> <NEW_PROJECT_NAME>",
    ),
    (&["dopbase", "project", "delete", "--help"], "<PROJECT_REF>"),
    (
      &["dopbase", "env", "create", "--help"],
      "<PROJECT_REF/ENVIRONMENT_NAME>",
    ),
    (
      &["dopbase", "env", "clone", "--help"],
      "<PROJECT_REF/SOURCE_ENVIRONMENT_NAME> <NEW_ENVIRONMENT_NAME>",
    ),
    (&["dopbase", "env", "list", "--help"], "[PROJECT_REF]"),
    (
      &["dopbase", "env", "default", "--help"],
      "[ENVIRONMENT_REF]",
    ),
    (&["dopbase", "env", "show", "--help"], "<ENVIRONMENT_REF>"),
    (
      &["dopbase", "env", "rename", "--help"],
      "<ENVIRONMENT_REF> <NEW_ENVIRONMENT_NAME>",
    ),
    (&["dopbase", "env", "delete", "--help"], "<ENVIRONMENT_REF>"),
    (
      &["dopbase", "secret", "list", "--help"],
      "<ENVIRONMENT_REF>",
    ),
    (
      &["dopbase", "secret", "set", "--help"],
      "<ENVIRONMENT_REF> <KEY>",
    ),
    (
      &["dopbase", "secret", "get", "--help"],
      "<ENVIRONMENT_REF> <KEY>",
    ),
    (
      &["dopbase", "secret", "delete", "--help"],
      "<ENVIRONMENT_REF> <KEY>",
    ),
    (&["dopbase", "import", "--help"], "<ENVIRONMENT_REF> <PATH>"),
    (&["dopbase", "export", "--help"], "<ENVIRONMENT_REF>"),
    (
      &["dopbase", "token", "create", "--help"],
      "<ENVIRONMENT_REF>",
    ),
    (&["dopbase", "token", "list", "--help"], "<ENVIRONMENT_REF>"),
    (&["dopbase", "run", "--help"], "[ENVIRONMENT_REF]"),
  ];

  for (arguments, expected) in cases {
    let help = contextual_help(arguments);
    assert!(help.contains(expected), "{arguments:?}: {help}");
  }
}

#[test]
fn other_incomplete_commands_show_full_help() {
  let cases: &[&[&str]] = &[
    &["dopbase", "client", "connect"],
    &["dopbase", "project", "rename", "payment-service"],
    &["dopbase", "env", "show"],
    &["dopbase", "env", "clone", "payment-service/local"],
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
fn env_clone_requires_a_qualified_source_and_one_new_name() {
  assert!(
    Cli::try_parse_from([
      "dopbase",
      "env",
      "clone",
      "payment-service/local",
      "production"
    ])
    .is_ok()
  );
  assert!(
    Cli::try_parse_from(["dopbase", "env", "clone", "prj_01JTEST/local", "preview-42"]).is_ok()
  );
  for arguments in [
    vec!["dopbase", "env", "clone", "env_482731", "production"],
    vec![
      "dopbase",
      "env",
      "clone",
      "payment-service/local",
      "payment-service",
      "production",
    ],
    vec![
      "dopbase",
      "env",
      "clone",
      "payment-service/local",
      "Production",
    ],
  ] {
    assert!(Cli::try_parse_from(arguments).is_err());
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
fn parses_each_public_command_and_detects_missing_cases() {
  fn leaves(
    command: &clap::Command,
    parent: &str,
    paths: &mut std::collections::BTreeSet<String>,
  ) {
    let children = command
      .get_subcommands()
      .filter(|child| !child.is_hide_set() && child.get_name() != "help")
      .collect::<Vec<_>>();
    if children.is_empty() {
      if !parent.is_empty() {
        paths.insert(parent.to_owned());
      }
    } else {
      for child in children {
        let path = if parent.is_empty() {
          child.get_name().to_owned()
        } else {
          format!("{parent} {}", child.get_name())
        };
        leaves(child, &path, paths);
      }
    }
  }
  let cases: &[&[&str]] = &[
    &["dopbase", "server", "setup"],
    &["dopbase", "server", "start"],
    &["dopbase", "server", "stop"],
    &["dopbase", "server", "restart"],
    &["dopbase", "server", "status"],
    &["dopbase", "server", "logs"],
    &[
      "dopbase",
      "client",
      "connect",
      "https://dopbase.example.com",
    ],
    &["dopbase", "client", "status"],
    &["dopbase", "login"],
    &["dopbase", "logout"],
    &["dopbase", "status"],
    &["dopbase", "init", "billing/local", "--from", ".env"],
    &["dopbase", "project", "create", "billing"],
    &["dopbase", "project", "list"],
    &["dopbase", "project", "show", "billing"],
    &["dopbase", "project", "rename", "billing", "payments"],
    &["dopbase", "project", "delete", "billing"],
    &["dopbase", "env", "default", "billing/local"],
    &["dopbase", "env", "create", "billing/local"],
    &["dopbase", "env", "clone", "billing/local", "production"],
    &["dopbase", "env", "list"],
    &["dopbase", "env", "show", "billing/local"],
    &["dopbase", "env", "rename", "billing/local", "production"],
    &["dopbase", "env", "delete", "billing/local"],
    &["dopbase", "secret", "list", "billing/local"],
    &["dopbase", "secret", "set", "billing/local", "API_KEY"],
    &["dopbase", "secret", "get", "billing/local", "API_KEY"],
    &["dopbase", "secret", "delete", "billing/local", "API_KEY"],
    &["dopbase", "import", "billing/local", ".env"],
    &["dopbase", "export", "billing/local", "--stdout"],
    &[
      "dopbase",
      "token",
      "create",
      "billing/local",
      "--name",
      "deploy",
    ],
    &["dopbase", "token", "list", "billing/local"],
    &["dopbase", "token", "revoke", "tok_01"],
    &["dopbase", "run", "--", "true"],
    &["dopbase", "cache", "list"],
    &["dopbase", "cache", "clean"],
    &["dopbase", "admin", "reset-password", "admin@example.com"],
    &["dopbase", "admin", "factory-reset"],
    &["dopbase", "update"],
    &["dopbase", "backup"],
    &["dopbase", "restore", "backup.dop"],
  ];
  let mut covered = std::collections::BTreeSet::new();
  for args in cases {
    let matches = Cli::command().try_get_matches_from(*args).unwrap();
    <Cli as clap::FromArgMatches>::from_arg_matches(&matches).unwrap();
    let mut current = &matches;
    let mut path = Vec::new();
    while let Some((name, child)) = current.subcommand() {
      path.push(name);
      current = child;
    }
    assert!(
      covered.insert(path.join(" ")),
      "duplicate canonical command: {args:?}"
    );
  }
  let mut expected = std::collections::BTreeSet::new();
  leaves(&Cli::command(), "", &mut expected);
  assert_eq!(
    covered, expected,
    "update canonical cases when commands change"
  );
}
