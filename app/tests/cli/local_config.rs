use app::cli::local_config::{
  ClientConfig, DefaultEnvironment, ResolvedServer, ServerSource, clear_default_environment, read,
  save_default_environment,
};
use tempfile::TempDir;

fn server(
  directory: &TempDir,
  url: &str,
) -> ResolvedServer {
  ResolvedServer {
    url: url.into(),
    source: ServerSource::Argument,
    config_path: directory.path().join("config.toml"),
    config: ClientConfig {
      version: 1,
      server_url: Some(url.into()),
      default_environment: None,
    },
  }
}

#[test]
fn default_environment_is_saved_as_an_id_scoped_to_the_server() {
  let directory = TempDir::new().unwrap();
  let server = server(&directory, "https://one.example.com");

  save_default_environment(&server, "env_01TEST").unwrap();
  let config = read(&server.config_path).unwrap();

  assert_eq!(
    config.default_environment,
    Some(DefaultEnvironment {
      server_url: server.url.clone(),
      environment_id: "env_01TEST".into(),
    })
  );
}

#[test]
fn resolved_default_must_match_the_active_server() {
  let directory = TempDir::new().unwrap();
  let mut server = server(&directory, "https://one.example.com");
  server.config.default_environment = Some(DefaultEnvironment {
    server_url: server.url.clone(),
    environment_id: "env_01TEST".into(),
  });
  assert_eq!(server.default_environment(), Some("env_01TEST"));

  server.url = "https://two.example.com".into();
  assert_eq!(server.default_environment(), None);
}

#[test]
fn clearing_only_removes_the_active_servers_default() {
  let directory = TempDir::new().unwrap();
  let mut server = server(&directory, "https://one.example.com");
  server.config.default_environment = Some(DefaultEnvironment {
    server_url: server.url.clone(),
    environment_id: "env_01TEST".into(),
  });
  app::cli::local_config::write(&server.config_path, &server.config).unwrap();

  let other = ResolvedServer {
    url: "https://two.example.com".into(),
    source: ServerSource::Argument,
    config_path: server.config_path.clone(),
    config: server.config.clone(),
  };
  assert!(!clear_default_environment(&other).unwrap());
  assert!(
    read(&server.config_path)
      .unwrap()
      .default_environment
      .is_some()
  );

  assert!(clear_default_environment(&server).unwrap());
  assert!(
    read(&server.config_path)
      .unwrap()
      .default_environment
      .is_none()
  );
}

#[test]
fn old_configuration_without_a_default_still_loads() {
  let config: ClientConfig =
    toml::from_str("version = 1\nserver_url = 'http://localhost:8840'\n").unwrap();
  assert!(config.default_environment.is_none());
}
