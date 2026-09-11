use app::cli::local_config::{
  ClientConfig, DefaultEnvironment, ResolvedServer, ServerSource, clear_default_environment,
  normalize, normalize_connect_target, read, save_default_environment,
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

#[test]
fn explicit_server_urls_accept_http_and_https() {
  for url in [
    "https://dopbase.example.com",
    "http://dopbase.example.com",
    "http://localhost:8840",
    "http://127.0.0.42:8840",
    "http://[::1]:8840",
    "http://192.168.1.10:8840",
    "http://[2001:db8::1]:8840",
  ] {
    assert!(normalize(url).is_ok(), "expected {url} to be accepted");
  }
}

#[test]
fn bare_ip_addresses_default_to_http() {
  assert_eq!(
    normalize_connect_target("192.168.1.20").unwrap(),
    "http://192.168.1.20"
  );
  assert_eq!(
    normalize_connect_target("192.168.1.20:8840").unwrap(),
    "http://192.168.1.20:8840"
  );
  assert_eq!(
    normalize_connect_target("2001:db8::1").unwrap(),
    "http://[2001:db8::1]"
  );
  assert_eq!(
    normalize_connect_target("[2001:db8::1]:8840").unwrap(),
    "http://[2001:db8::1]:8840"
  );
}

#[test]
fn bare_domains_require_an_explicit_scheme() {
  assert_eq!(
    normalize_connect_target("dopbase.example.com")
      .unwrap_err()
      .to_string(),
    "Server domains must include http:// or https://. For example: https://dopbase.example.com"
  );
}

#[test]
fn invalid_server_input_explains_the_accepted_formats() {
  let error = normalize_connect_target("sdkfsdf").unwrap_err().to_string();
  assert!(error.contains("is not valid"), "{error}");
  assert!(error.contains("https://dopbase.example.com"), "{error}");
  assert!(error.contains("192.168.1.20:8840"), "{error}");

  for value in [
    " http://dopbase.example.com",
    "http://sdkfsdf",
    "192.168.1.999",
  ] {
    assert!(
      normalize_connect_target(value)
        .unwrap_err()
        .to_string()
        .contains("is not valid"),
      "{value}"
    );
  }
}

#[test]
fn invalid_server_ports_have_a_focused_error() {
  for value in [
    "192.168.1.20:0",
    "192.168.1.20:65536",
    "https://dopbase.example.com:0",
    "https://dopbase.example.com:65536",
  ] {
    assert_eq!(
      normalize_connect_target(value).unwrap_err().to_string(),
      "The server port must be between 1 and 65535. For example: 192.168.1.20:8840",
      "{value}"
    );
  }
}

#[test]
fn unsupported_schemes_and_url_metadata_are_rejected() {
  assert_eq!(
    normalize_connect_target("ftp://dopbase.example.com")
      .unwrap_err()
      .to_string(),
    "The server address must use http:// or https://. For example: https://dopbase.example.com"
  );

  assert!(
    normalize_connect_target("https://user:pass@dopbase.example.com")
      .unwrap_err()
      .to_string()
      .contains("must not contain credentials")
  );
  assert!(
    normalize_connect_target("https://dopbase.example.com?mode=test")
      .unwrap_err()
      .to_string()
      .contains("must not contain a query or fragment")
  );
}
