use std::fs;
use std::net::SocketAddr;

use app::config::{
  EnvironmentOverrides, ServerConfig, ServerOverrides, ensure_data_dir, sqlite_url,
};
use app::constants::config::{
  DATABASE_FILENAME, DEFAULT_BIND_ADDRESS, DEFAULT_PORT, DEFAULT_PUBLIC_URL, MASTER_KEY_FILENAME,
  SERVER_CONFIG_FILENAME, daemon_environment_names,
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
    "bind_address = '127.0.0.1:1001'\ndatabase_url = 'sqlite://file.db'\n[master_key]\npath = 'file.key'\n",
  )
  .unwrap();
  let overrides = ServerOverrides {
    data_dir: Some(data_dir.clone()),
    bind_address: Some("127.0.0.1:3003".into()),
    database_url: Some("sqlite://cli.db".into()),
    master_key_path: Some(directory.path().join("cli.key")),
    ..Default::default()
  };
  let environment = EnvironmentOverrides {
    bind_address: Some("127.0.0.1:2002".into()),
    database_url: Some("sqlite://environment.db".into()),
    master_key_path: Some(directory.path().join("environment.key")),
    ..Default::default()
  };

  let config = ServerConfig::load_with_environment(&overrides, environment).unwrap();
  assert_eq!(config.data_dir, data_dir);
  assert_eq!(config.bind_address, "127.0.0.1:3003");
  assert_eq!(config.database_url, "sqlite://cli.db");
  assert_eq!(config.master_key.path, directory.path().join("cli.key"));
}

#[test]
fn docs_are_disabled_by_default() {
  let config = ServerConfig::default();
  assert!(!config.docs_enabled);
}

#[test]
fn config_file_enables_docs() {
  let directory = tempfile::TempDir::new().unwrap();
  let data_dir = directory.path().join("data");
  fs::create_dir_all(&data_dir).unwrap();
  fs::write(data_dir.join(SERVER_CONFIG_FILENAME), "docs = true\n").unwrap();
  let config = ServerConfig::load_with_environment(
    &ServerOverrides {
      data_dir: Some(data_dir),
      ..Default::default()
    },
    EnvironmentOverrides::default(),
  )
  .unwrap();
  assert!(config.docs_enabled);
}

#[test]
fn cli_overrides_docs_environment_and_config_file() {
  let directory = tempfile::TempDir::new().unwrap();
  let data_dir = directory.path().join("data");
  fs::create_dir_all(&data_dir).unwrap();
  fs::write(data_dir.join(SERVER_CONFIG_FILENAME), "docs = true\n").unwrap();
  let overrides = ServerOverrides {
    data_dir: Some(data_dir),
    docs: Some(false),
    ..Default::default()
  };
  let environment = EnvironmentOverrides {
    docs: Some("true".into()),
    ..Default::default()
  };
  let config = ServerConfig::load_with_environment(&overrides, environment).unwrap();
  assert!(!config.docs_enabled);
}

#[test]
fn environment_overrides_docs_config_file() {
  let directory = tempfile::TempDir::new().unwrap();
  let data_dir = directory.path().join("data");
  fs::create_dir_all(&data_dir).unwrap();
  let config = ServerConfig::load_with_environment(
    &ServerOverrides {
      data_dir: Some(data_dir),
      ..Default::default()
    },
    EnvironmentOverrides {
      docs: Some("true".into()),
      ..Default::default()
    },
  )
  .unwrap();
  assert!(config.docs_enabled);
}

#[test]
fn invalid_docs_environment_is_rejected() {
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
fn remote_host_without_public_url_fails_with_guidance() {
  let directory = tempfile::TempDir::new().unwrap();
  let data_dir = directory.path().join("data");
  fs::create_dir_all(&data_dir).unwrap();
  let result = ServerConfig::load_with_environment(
    &ServerOverrides {
      data_dir: Some(data_dir),
      host: Some("0.0.0.0".into()),
      ..Default::default()
    },
    EnvironmentOverrides::default(),
  );
  let error = result.unwrap_err().to_string();
  assert!(error.contains("public_url is required"), "{error}");
  assert!(error.contains("--public-url"), "{error}");
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
fn port_conflicts_with_explicit_bind_address() {
  let directory = tempfile::TempDir::new().unwrap();
  let data_dir = directory.path().join("data");
  fs::create_dir_all(&data_dir).unwrap();
  let config_path = data_dir.join(SERVER_CONFIG_FILENAME);
  fs::write(&config_path, "bind_address = '127.0.0.1:1001'\n").unwrap();
  let result = ServerConfig::load_with_environment(
    &ServerOverrides {
      data_dir: Some(data_dir),
      port: Some(9000),
      ..Default::default()
    },
    EnvironmentOverrides::default(),
  );
  let error = result.unwrap_err().to_string();
  assert!(
    error.contains("either port/host or bind_address"),
    "{error}"
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
fn rejects_remote_bind_without_public_url() {
  let config = ServerConfig {
    bind_address: "0.0.0.0:8840".into(),
    ..Default::default()
  };
  assert!(config.validate().is_err());
}

#[test]
fn default_constants_agree_on_the_default_port() {
  let bind: SocketAddr = DEFAULT_BIND_ADDRESS.parse().unwrap();
  assert_eq!(bind.port(), DEFAULT_PORT);
  assert!(
    DEFAULT_PUBLIC_URL.ends_with(&format!(":{DEFAULT_PORT}")),
    "DEFAULT_PUBLIC_URL ({DEFAULT_PUBLIC_URL}) must use DEFAULT_PORT ({DEFAULT_PORT})"
  );
}

#[test]
fn daemon_environment_names_are_unique() {
  let names = daemon_environment_names();
  let mut sorted = names.to_vec();
  sorted.sort_unstable();
  let count = sorted.len();
  sorted.dedup();
  assert_eq!(sorted.len(), count, "duplicate environment variable names");
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
