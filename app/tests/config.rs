use std::fs;
use std::net::SocketAddr;

use app::config::{
  EnvironmentOverrides, PublicUrlSource, ServerConfig, ServerOverrides, ensure_data_dir,
  resolve_implicit_public_url, sqlite_url,
};
use app::constants::config::{
  DATABASE_FILENAME, DEFAULT_BIND_ADDRESS, MASTER_KEY_FILENAME, SERVER_CONFIG_FILENAME,
};

#[test]
fn defaults_use_the_dopbase_data_directory() {
  let config = ServerConfig::default();
  assert_eq!(
    config.database_url,
    sqlite_url(&config.data_dir.join(DATABASE_FILENAME))
  );
  assert_eq!(
    config.master_key.path,
    config.data_dir.join(MASTER_KEY_FILENAME)
  );
  assert_eq!(
    config.config_path,
    config.data_dir.join(SERVER_CONFIG_FILENAME)
  );
  assert!(config.web_ui_enabled);
  config.validate().unwrap();
}

#[test]
fn cli_overrides_environment_and_config_file() {
  let directory = tempfile::TempDir::new().unwrap();
  let data_dir = directory.path().join("data");
  fs::create_dir_all(&data_dir).unwrap();
  let config_path = data_dir.join(SERVER_CONFIG_FILENAME);
  fs::write(
    &config_path,
    "host = '127.0.0.1'\nport = 1001\n[master_key]\npath = 'file.key'\n",
  )
  .unwrap();
  let overrides = ServerOverrides {
    data_dir: Some(data_dir.clone()),
    port: Some(3003),
    master_key_path: Some(directory.path().join("cli.key")),
    ..Default::default()
  };
  let environment = EnvironmentOverrides {
    port: Some("2002".into()),
    master_key_path: Some(directory.path().join("environment.key")),
    ..Default::default()
  };

  let config = ServerConfig::load_with_environment(&overrides, environment).unwrap();
  assert_eq!(config.data_dir, data_dir);
  assert_eq!(config.bind_address, "127.0.0.1:3003");
  assert_eq!(
    config.database_url,
    sqlite_url(&data_dir.join(DATABASE_FILENAME))
  );
  assert_eq!(config.master_key.path, directory.path().join("cli.key"));
}

#[test]
fn docs_precedence_is_config_then_environment_then_cli() {
  // Default: docs are disabled.
  assert!(!ServerConfig::default().docs_enabled);

  struct Case {
    config_file: Option<&'static str>,
    environment: EnvironmentOverrides,
    cli: Option<bool>,
    expected: bool,
  }

  let cases = [
    Case {
      config_file: None,
      environment: EnvironmentOverrides::default(),
      cli: None,
      expected: false,
    },
    Case {
      config_file: Some("docs = true\n"),
      environment: EnvironmentOverrides::default(),
      cli: None,
      expected: true,
    },
    Case {
      config_file: None,
      environment: EnvironmentOverrides {
        docs: Some("true".into()),
        ..Default::default()
      },
      cli: None,
      expected: true,
    },
    Case {
      config_file: Some("docs = true\n"),
      environment: EnvironmentOverrides {
        docs: Some("true".into()),
        ..Default::default()
      },
      cli: Some(false),
      expected: false,
    },
  ];

  for case in cases {
    let directory = tempfile::TempDir::new().unwrap();
    let data_dir = directory.path().join("data");
    fs::create_dir_all(&data_dir).unwrap();
    if let Some(contents) = case.config_file {
      fs::write(data_dir.join(SERVER_CONFIG_FILENAME), contents).unwrap();
    }
    let config = ServerConfig::load_with_environment(
      &ServerOverrides {
        data_dir: Some(data_dir),
        docs: case.cli,
        ..Default::default()
      },
      case.environment,
    )
    .unwrap();
    assert_eq!(config.docs_enabled, case.expected);
  }

  // An unparseable environment value is rejected.
  let directory = tempfile::TempDir::new().unwrap();
  let data_dir = directory.path().join("data");
  fs::create_dir_all(&data_dir).unwrap();
  let result = ServerConfig::load_with_environment(
    &ServerOverrides {
      data_dir: Some(data_dir),
      ..Default::default()
    },
    EnvironmentOverrides {
      docs: Some("yes".into()),
      ..Default::default()
    },
  );
  assert!(
    result
      .unwrap_err()
      .to_string()
      .contains("invalid DOPBASE_DOCS")
  );
}

#[test]
fn port_override_derives_loopback_public_url() {
  let directory = tempfile::TempDir::new().unwrap();
  let data_dir = directory.path().join("data");
  fs::create_dir_all(&data_dir).unwrap();
  let config = ServerConfig::load_with_environment(
    &ServerOverrides {
      data_dir: Some(data_dir),
      port: Some(9000),
      ..Default::default()
    },
    EnvironmentOverrides::default(),
  )
  .unwrap();
  assert_eq!(config.bind_address, "127.0.0.1:9000");
  assert_eq!(config.public_url, "http://localhost:9000");
}

#[test]
fn host_and_port_compose_bind_address() {
  let directory = tempfile::TempDir::new().unwrap();
  let data_dir = directory.path().join("data");
  fs::create_dir_all(&data_dir).unwrap();
  let config = ServerConfig::load_with_environment(
    &ServerOverrides {
      data_dir: Some(data_dir),
      host: Some("0.0.0.0".into()),
      port: Some(9000),
      public_url: Some("https://dopbase.example.com".into()),
      ..Default::default()
    },
    EnvironmentOverrides::default(),
  )
  .unwrap();
  assert_eq!(config.bind_address, "0.0.0.0:9000");
  assert_eq!(config.public_url, "https://dopbase.example.com");
}

#[test]
fn concrete_network_host_uses_the_server_host_placeholder() {
  let directory = tempfile::TempDir::new().unwrap();
  let data_dir = directory.path().join("data");
  fs::create_dir_all(&data_dir).unwrap();
  let config = ServerConfig::load_with_environment(
    &ServerOverrides {
      data_dir: Some(data_dir),
      host: Some("192.168.1.20".into()),
      port: Some(9000),
      ..Default::default()
    },
    EnvironmentOverrides::default(),
  )
  .unwrap();
  assert_eq!(config.public_url, "http://SERVER_HOST:9000");
  assert_eq!(
    config.public_url_source,
    PublicUrlSource::NetworkPlaceholder
  );
  let warning = config.public_url_warning().unwrap();
  assert!(warning.contains("Replace SERVER_HOST"));
  assert!(warning.contains("does not encrypt credentials, secrets, or setup tokens"));
  assert!(warning.contains("DOPBASE_PUBLIC_URL"));
  assert!(warning.contains("server.toml"));
}

#[test]
fn non_loopback_hosts_use_the_server_host_placeholder() {
  for bind in ["0.0.0.0:8840", "[::]:9000", "192.168.1.20:9100"] {
    let bind: SocketAddr = bind.parse().unwrap();
    let (public_url, source) = resolve_implicit_public_url(bind);
    assert_eq!(public_url, format!("http://SERVER_HOST:{}", bind.port()));
    assert_eq!(source, PublicUrlSource::NetworkPlaceholder);
  }
}

#[test]
fn wildcard_host_loads_without_network_detection() {
  let directory = tempfile::TempDir::new().unwrap();
  let config = ServerConfig::load_with_environment(
    &ServerOverrides {
      data_dir: Some(directory.path().join("data")),
      host: Some("0.0.0.0".into()),
      ..Default::default()
    },
    EnvironmentOverrides::default(),
  )
  .unwrap();
  assert_eq!(config.bind_address, "0.0.0.0:8840");
  assert_eq!(config.public_url, "http://SERVER_HOST:8840");
}

#[test]
fn explicit_public_urls_accept_http_and_https() {
  for public_url in [
    "https://dopbase.example.com",
    "https://203.0.113.10:8840",
    "http://dopbase.example.com",
    "http://203.0.113.10:8840",
    "http://localhost:8840",
    "http://127.0.0.9:8840",
    "http://[::1]:8840",
  ] {
    let directory = tempfile::TempDir::new().unwrap();
    let config = ServerConfig::load_with_environment(
      &ServerOverrides {
        data_dir: Some(directory.path().join("data")),
        public_url: Some(public_url.into()),
        ..Default::default()
      },
      EnvironmentOverrides::default(),
    );
    assert!(config.is_ok(), "expected {public_url} to be accepted");
  }
}

#[test]
fn remote_http_public_urls_have_a_security_warning() {
  for public_url in ["http://dopbase.example.com", "http://10.0.0.8:8840"] {
    let directory = tempfile::TempDir::new().unwrap();
    let config = ServerConfig::load_with_environment(
      &ServerOverrides {
        data_dir: Some(directory.path().join("data")),
        public_url: Some(public_url.into()),
        ..Default::default()
      },
      EnvironmentOverrides::default(),
    )
    .unwrap();
    let warning = config.public_url_warning().unwrap();
    assert!(warning.contains(public_url), "{warning}");
    assert!(warning.contains("Use HTTPS for deployments"), "{warning}");
  }

  let https = ServerConfig::load_with_environment(
    &ServerOverrides {
      data_dir: Some(tempfile::TempDir::new().unwrap().path().join("data")),
      public_url: Some("https://dopbase.example.com".into()),
      ..Default::default()
    },
    EnvironmentOverrides::default(),
  )
  .unwrap();
  assert!(https.public_url_warning().is_none());

  let loopback = ServerConfig::load_with_environment(
    &ServerOverrides {
      data_dir: Some(tempfile::TempDir::new().unwrap().path().join("data")),
      public_url: Some("http://localhost:8840".into()),
      ..Default::default()
    },
    EnvironmentOverrides::default(),
  )
  .unwrap();
  assert!(loopback.public_url_warning().is_none());
}

#[test]
fn localhost_host_maps_to_loopback() {
  let directory = tempfile::TempDir::new().unwrap();
  let data_dir = directory.path().join("data");
  fs::create_dir_all(&data_dir).unwrap();
  let config = ServerConfig::load_with_environment(
    &ServerOverrides {
      data_dir: Some(data_dir),
      host: Some("localhost".into()),
      ..Default::default()
    },
    EnvironmentOverrides::default(),
  )
  .unwrap();
  assert_eq!(config.bind_address, "127.0.0.1:8840");
  assert_eq!(config.public_url, "http://localhost:8840");
}

#[test]
fn legacy_config_keys_are_ignored() {
  let directory = tempfile::TempDir::new().unwrap();
  let data_dir = directory.path().join("data");
  fs::create_dir_all(&data_dir).unwrap();
  let config_path = data_dir.join(SERVER_CONFIG_FILENAME);
  fs::write(
    &config_path,
    "bind_address = '0.0.0.0:1001'\ndatabase_url = 'sqlite://elsewhere.db'\n",
  )
  .unwrap();
  let config = ServerConfig::load_with_environment(
    &ServerOverrides {
      data_dir: Some(data_dir.clone()),
      ..Default::default()
    },
    EnvironmentOverrides::default(),
  )
  .unwrap();
  assert_eq!(config.bind_address, DEFAULT_BIND_ADDRESS);
  assert_eq!(
    config.database_url,
    sqlite_url(&data_dir.join(DATABASE_FILENAME))
  );
}

#[test]
fn environment_port_and_host_are_used() {
  let directory = tempfile::TempDir::new().unwrap();
  let data_dir = directory.path().join("data");
  fs::create_dir_all(&data_dir).unwrap();
  let config = ServerConfig::load_with_environment(
    &ServerOverrides {
      data_dir: Some(data_dir),
      ..Default::default()
    },
    EnvironmentOverrides {
      port: Some("9100".into()),
      host: Some("127.0.0.1".into()),
      ..Default::default()
    },
  )
  .unwrap();
  assert_eq!(config.bind_address, "127.0.0.1:9100");
  assert_eq!(config.public_url, "http://localhost:9100");
}

#[test]
fn invalid_environment_port_is_rejected() {
  let directory = tempfile::TempDir::new().unwrap();
  let data_dir = directory.path().join("data");
  fs::create_dir_all(&data_dir).unwrap();
  let result = ServerConfig::load_with_environment(
    &ServerOverrides {
      data_dir: Some(data_dir),
      ..Default::default()
    },
    EnvironmentOverrides {
      port: Some("not-a-port".into()),
      ..Default::default()
    },
  );
  assert!(
    result
      .unwrap_err()
      .to_string()
      .contains("invalid DOPBASE_PORT")
  );
}

#[test]
fn rejects_a_placeholder_source_with_a_loopback_bind() {
  let config = ServerConfig {
    public_url: "http://SERVER_HOST:8840".into(),
    public_url_source: PublicUrlSource::NetworkPlaceholder,
    ..Default::default()
  };
  let error = config.validate().unwrap_err().to_string();
  assert!(error.contains("loopback bind address"), "{error}");
}

#[cfg(unix)]
#[test]
fn creates_owner_only_data_directory() {
  use std::os::unix::fs::PermissionsExt;
  let directory = tempfile::TempDir::new().unwrap();
  let data_dir = directory.path().join("private");
  ensure_data_dir(&data_dir).unwrap();
  assert_eq!(
    fs::metadata(data_dir).unwrap().permissions().mode() & 0o777,
    0o700
  );
}

fn uncomment_server_examples(reference: &str) -> String {
  reference
    .lines()
    .filter_map(|line| {
      line
        .strip_prefix("# ")
        .or((!line.starts_with('#')).then_some(line))
    })
    .filter(|line| {
      line.starts_with("[master_key]")
        || [
          "version",
          "host",
          "port",
          "public_url",
          "shutdown_grace_seconds",
          "audit_retention_days",
          "trusted_proxies",
          "docs",
          "web_ui",
          "provider",
          "path",
        ]
        .iter()
        .any(|key| line.starts_with(&format!("{key} = ")))
        || line.starts_with('"')
    })
    .collect::<Vec<_>>()
    .join("\n")
}

#[test]
fn generated_references_load_defaults_and_uncommented_server_examples() {
  let directory = tempfile::TempDir::new().unwrap();
  let data_dir = directory.path().join("data with 'quotes' and \\\"slashes");
  let overrides = ServerOverrides {
    data_dir: Some(data_dir.clone()),
    port: Some(9000),
    docs: Some(true),
    web_ui: Some(false),
    master_key_path: Some(directory.path().join("temporary.key")),
    ..Default::default()
  };
  let config =
    ServerConfig::load_with_environment(&overrides, EnvironmentOverrides::default()).unwrap();
  assert!(
    !data_dir.exists(),
    "loading configuration must stay read-only"
  );
  config.ensure_reference_files().unwrap();

  for filename in [
    SERVER_CONFIG_FILENAME,
    app::constants::config::CLIENT_CONFIG_FILENAME,
  ] {
    let contents = fs::read_to_string(data_dir.join(filename)).unwrap();
    // Everything is a commented example except the audit retention that new
    // instances start with.
    let active = toml::from_str::<toml::Table>(&contents).unwrap();
    if filename == SERVER_CONFIG_FILENAME {
      assert_eq!(
        active,
        toml::from_str::<toml::Table>("audit_retention_days = 365").unwrap()
      );
    } else {
      assert!(active.is_empty());
    }
  }
  let client = app::cli::local_config::read(&data_dir.join("config.toml")).unwrap();
  assert_eq!(client.version, 1);
  assert!(client.server_url.is_none());
  assert!(client.default_environment.is_none());

  let reference = fs::read_to_string(&config.config_path).unwrap();
  assert!(!reference.contains("temporary.key"));
  fs::write(&config.config_path, uncomment_server_examples(&reference)).unwrap();
  let reloaded = ServerConfig::load_with_environment(
    &ServerOverrides {
      data_dir: Some(data_dir.clone()),
      ..Default::default()
    },
    EnvironmentOverrides::default(),
  )
  .unwrap();
  assert_eq!(reloaded.bind_address, DEFAULT_BIND_ADDRESS);
  assert_eq!(
    reloaded.public_url,
    app::constants::config::DEFAULT_PUBLIC_URL
  );
  assert_eq!(reloaded.shutdown_grace_seconds, 10);
  assert_eq!(reloaded.audit_retention_days, Some(365));
  assert_eq!(
    reloaded
      .trusted_proxies
      .iter()
      .map(ToString::to_string)
      .collect::<Vec<_>>(),
    ["127.0.0.1/32", "10.0.0.0/8"]
  );
  assert!(!reloaded.docs_enabled);
  assert!(reloaded.web_ui_enabled);
  assert_eq!(reloaded.master_key.provider, "file");
  assert_eq!(reloaded.master_key.path, data_dir.join(MASTER_KEY_FILENAME));
}

#[test]
fn reference_creation_preserves_existing_files_and_custom_config_paths() {
  let directory = tempfile::TempDir::new().unwrap();
  let data_dir = directory.path().join("data");
  let config_path = directory.path().join("custom/server.toml");
  let config = ServerConfig::load_with_environment(
    &ServerOverrides {
      data_dir: Some(data_dir.clone()),
      config_path: Some(config_path.clone()),
      ..Default::default()
    },
    EnvironmentOverrides::default(),
  )
  .unwrap();
  fs::create_dir_all(&data_dir).unwrap();
  let client_path = data_dir.join("config.toml");
  let saved_client = "# My connection\nversion = 1\nserver_url = 'https://example.com'\n";
  fs::write(&client_path, saved_client).unwrap();
  config.ensure_reference_files().unwrap();
  assert!(config_path.exists());
  assert!(!data_dir.join(SERVER_CONFIG_FILENAME).exists());
  assert_eq!(fs::read_to_string(&client_path).unwrap(), saved_client);

  let saved_server = "# My server\nport = 9123\ndocs = true\n";
  fs::write(&config_path, saved_server).unwrap();
  config.ensure_reference_files().unwrap();
  assert_eq!(fs::read_to_string(config_path).unwrap(), saved_server);
  assert_eq!(fs::read_to_string(client_path).unwrap(), saved_client);
}

#[cfg(unix)]
#[test]
fn generated_reference_files_are_private() {
  use std::os::unix::fs::PermissionsExt;
  let directory = tempfile::TempDir::new().unwrap();
  let config = ServerConfig::load_with_environment(
    &ServerOverrides {
      data_dir: Some(directory.path().join("data")),
      ..Default::default()
    },
    EnvironmentOverrides::default(),
  )
  .unwrap();
  config.ensure_reference_files().unwrap();
  for path in [&config.config_path, &config.data_dir.join("config.toml")] {
    assert_eq!(
      fs::metadata(path).unwrap().permissions().mode() & 0o777,
      0o600
    );
  }
}

#[test]
fn web_ui_precedence_is_cli_then_environment_then_file_then_default() {
  for (file, environment, cli, expected) in [
    (None, None, None, true),
    (Some(false), None, None, false),
    (Some(true), None, None, true),
    (Some(true), Some("false"), None, false),
    (Some(false), Some("true"), None, true),
    (Some(true), Some("true"), Some(false), false),
    (Some(false), Some("false"), Some(true), true),
  ] {
    let directory = tempfile::TempDir::new().unwrap();
    if let Some(value) = file {
      fs::write(
        directory.path().join(SERVER_CONFIG_FILENAME),
        format!("web_ui = {value}\n"),
      )
      .unwrap();
    }
    let config = ServerConfig::load_with_environment(
      &ServerOverrides {
        data_dir: Some(directory.path().into()),
        web_ui: cli,
        ..Default::default()
      },
      EnvironmentOverrides {
        web_ui: environment.map(str::to_owned),
        ..Default::default()
      },
    )
    .unwrap();
    assert_eq!(
      config.web_ui_enabled, expected,
      "file={file:?}, env={environment:?}, cli={cli:?}"
    );
  }
}

#[test]
fn invalid_web_ui_settings_are_rejected() {
  let directory = tempfile::TempDir::new().unwrap();
  let overrides = ServerOverrides {
    data_dir: Some(directory.path().into()),
    ..Default::default()
  };
  for value in ["yes", "0", "", "TRUE"] {
    let error = ServerConfig::load_with_environment(
      &overrides,
      EnvironmentOverrides {
        web_ui: Some(value.into()),
        ..Default::default()
      },
    )
    .unwrap_err();
    assert!(format!("{error:#}").contains("invalid DOPBASE_WEB_UI"));
  }
  for value in ["'false'", "0"] {
    fs::write(
      directory.path().join(SERVER_CONFIG_FILENAME),
      format!("web_ui = {value}\n"),
    )
    .unwrap();
    let error =
      ServerConfig::load_with_environment(&overrides, EnvironmentOverrides::default()).unwrap_err();
    assert!(format!("{error:#}").contains("failed to parse"));
  }
}

#[test]
fn audit_retention_days_reads_from_file_and_environment() {
  let directory = tempfile::TempDir::new().unwrap();
  let data_dir = directory.path().join("data");
  fs::create_dir_all(&data_dir).unwrap();
  let config_path = data_dir.join(SERVER_CONFIG_FILENAME);
  let overrides = ServerOverrides {
    data_dir: Some(data_dir.clone()),
    ..Default::default()
  };

  // Unset keeps the whole log.
  assert_eq!(ServerConfig::default().audit_retention_days, None);

  fs::write(&config_path, "audit_retention_days = 90\n").unwrap();
  let from_file =
    ServerConfig::load_with_environment(&overrides, EnvironmentOverrides::default()).unwrap();
  assert_eq!(from_file.audit_retention_days, Some(90));

  // The environment variable wins over the file.
  let from_environment = ServerConfig::load_with_environment(
    &overrides,
    EnvironmentOverrides {
      audit_retention_days: Some("30".into()),
      ..Default::default()
    },
  )
  .unwrap();
  assert_eq!(from_environment.audit_retention_days, Some(30));

  // A window of zero days would remove every record on the first write, so
  // it is rejected as a misconfiguration rather than accepted as a policy.
  fs::write(&config_path, "audit_retention_days = 0\n").unwrap();
  let error =
    ServerConfig::load_with_environment(&overrides, EnvironmentOverrides::default()).unwrap_err();
  assert!(
    format!("{error:#}").contains("audit_retention_days"),
    "{error}"
  );
  fs::write(&config_path, "audit_retention_days = 90\n").unwrap();
  let error = ServerConfig::load_with_environment(
    &overrides,
    EnvironmentOverrides {
      audit_retention_days: Some("0".into()),
      ..Default::default()
    },
  )
  .unwrap_err();
  assert!(
    format!("{error:#}").contains("audit_retention_days"),
    "{error}"
  );
  let error = ServerConfig::load_with_environment(
    &overrides,
    EnvironmentOverrides {
      audit_retention_days: Some("soon".into()),
      ..Default::default()
    },
  )
  .unwrap_err();
  assert!(
    format!("{error:#}").contains("invalid DOPBASE_AUDIT_RETENTION_DAYS"),
    "{error}"
  );
}

#[test]
fn trusted_proxies_read_from_file_and_environment() {
  let directory = tempfile::TempDir::new().unwrap();
  let data_dir = directory.path().join("data");
  fs::create_dir_all(&data_dir).unwrap();
  let config_path = data_dir.join(SERVER_CONFIG_FILENAME);
  let overrides = ServerOverrides {
    data_dir: Some(data_dir.clone()),
    ..Default::default()
  };
  let networks = |config: &ServerConfig| {
    config
      .trusted_proxies
      .iter()
      .map(ToString::to_string)
      .collect::<Vec<_>>()
  };

  // Unset trusts no proxy.
  assert!(ServerConfig::default().trusted_proxies.is_empty());

  fs::write(
    &config_path,
    "trusted_proxies = [\"10.0.0.0/8\", \"::1\"]\n",
  )
  .unwrap();
  let from_file =
    ServerConfig::load_with_environment(&overrides, EnvironmentOverrides::default()).unwrap();
  assert_eq!(networks(&from_file), ["10.0.0.0/8", "::1/128"]);

  // The environment variable wins over the file.
  let from_environment = ServerConfig::load_with_environment(
    &overrides,
    EnvironmentOverrides {
      trusted_proxies: Some("172.16.0.0/12, 127.0.0.1".into()),
      ..Default::default()
    },
  )
  .unwrap();
  assert_eq!(
    networks(&from_environment),
    ["172.16.0.0/12", "127.0.0.1/32"]
  );

  fs::write(&config_path, "trusted_proxies = [\"10.0.0.0/33\"]\n").unwrap();
  let error =
    ServerConfig::load_with_environment(&overrides, EnvironmentOverrides::default()).unwrap_err();
  assert!(format!("{error:#}").contains("trusted_proxies"), "{error}");
  fs::remove_file(&config_path).unwrap();
  let error = ServerConfig::load_with_environment(
    &overrides,
    EnvironmentOverrides {
      trusted_proxies: Some("proxy.internal".into()),
      ..Default::default()
    },
  )
  .unwrap_err();
  assert!(
    format!("{error:#}").contains("invalid DOPBASE_TRUSTED_PROXIES"),
    "{error}"
  );
}

fn load_with_server_config(contents: &str) -> anyhow::Result<ServerConfig> {
  let directory = tempfile::TempDir::new().unwrap();
  let data_dir = directory.path().join("data");
  fs::create_dir_all(&data_dir).unwrap();
  fs::write(data_dir.join(SERVER_CONFIG_FILENAME), contents).unwrap();
  ServerConfig::load_with_environment(
    &ServerOverrides {
      data_dir: Some(data_dir),
      ..Default::default()
    },
    EnvironmentOverrides::default(),
  )
}

#[test]
fn oidc_is_empty_by_default() {
  assert!(ServerConfig::default().oidc.issuers.is_empty());
}

#[test]
fn oidc_trust_policy_reads_issuers_from_server_toml() {
  let config = load_with_server_config(
    r#"
[oidc]
[[oidc.issuers]]
issuer = "https://auth.example.com"
audiences = ["dopbase", "dopbase-cli"]
algorithms = ["RS256", "ES256"]
jwks_uri = "https://auth.example.com/.well-known/jwks.json"
"#,
  )
  .unwrap();

  assert_eq!(config.oidc.issuers.len(), 1);
  let issuer = &config.oidc.issuers[0];
  assert_eq!(issuer.issuer, "https://auth.example.com");
  assert_eq!(issuer.audiences, ["dopbase", "dopbase-cli"]);
  assert_eq!(
    issuer.algorithms,
    [
      app::services::oidc::Algorithm::RS256,
      app::services::oidc::Algorithm::ES256
    ]
  );
  assert_eq!(
    issuer.jwks_uri.as_deref(),
    Some("https://auth.example.com/.well-known/jwks.json")
  );
}

#[test]
fn oidc_defaults_algorithms_when_omitted() {
  let config = load_with_server_config(
    r#"
[oidc]
[[oidc.issuers]]
issuer = "https://auth.example.com"
audiences = ["dopbase"]
"#,
  )
  .unwrap();

  assert_eq!(
    config.oidc.issuers[0].algorithms,
    [app::services::oidc::Algorithm::RS256]
  );
  assert!(config.oidc.issuers[0].jwks_uri.is_none());
}

#[test]
fn oidc_accepts_a_loopback_http_issuer() {
  let config = load_with_server_config(
    r#"
[oidc]
[[oidc.issuers]]
issuer = "http://localhost:8080"
audiences = ["dopbase"]
"#,
  );
  assert!(
    config.is_ok(),
    "{:?}",
    config.err().map(|e| format!("{e:#}"))
  );
}

#[test]
fn oidc_rejects_a_remote_http_issuer() {
  let error = load_with_server_config(
    r#"
[oidc]
[[oidc.issuers]]
issuer = "http://auth.example.com"
audiences = ["dopbase"]
"#,
  )
  .unwrap_err();
  let message = format!("{error:#}");
  assert!(message.contains("invalid OIDC trust policy"), "{message}");
  assert!(message.contains("HTTPS"), "{message}");
}

#[test]
fn oidc_rejects_symmetric_algorithms() {
  let error = load_with_server_config(
    r#"
[oidc]
[[oidc.issuers]]
issuer = "https://auth.example.com"
audiences = ["dopbase"]
algorithms = ["HS256"]
"#,
  )
  .unwrap_err();
  assert!(format!("{error:#}").contains("symmetric"), "{error:#}");
}

#[test]
fn oidc_rejects_a_blank_audience_list() {
  let error = load_with_server_config(
    r#"
[oidc]
[[oidc.issuers]]
issuer = "https://auth.example.com"
audiences = []
"#,
  )
  .unwrap_err();
  assert!(format!("{error:#}").contains("audience"), "{error:#}");
}
