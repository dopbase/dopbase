use crate::{
  config::{client_config_path, ensure_data_dir},
  constants::config::{DEFAULT_PUBLIC_URL, ENV_SERVER_URL},
};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
  env, fs,
  path::{Path, PathBuf},
};
use url::{Host, ParseError, Url};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ClientConfig {
  #[serde(default = "version")]
  pub version: u32,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub server_url: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub default_environment: Option<DefaultEnvironment>,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct DefaultEnvironment {
  pub server_url: String,
  pub environment_id: String,
}
#[derive(Clone, Copy, Debug)]
pub enum ServerSource {
  Argument,
  Environment,
  Config,
  Default,
}
impl ServerSource {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Argument => "argument",
      Self::Environment => "environment",
      Self::Config => "config",
      Self::Default => "default",
    }
  }
}
pub struct ResolvedServer {
  pub url: String,
  pub source: ServerSource,
  pub config_path: PathBuf,
  pub config: ClientConfig,
}
impl ResolvedServer {
  pub fn default_environment(&self) -> Option<&str> {
    self
      .config
      .default_environment
      .as_ref()
      .filter(|default| default.server_url == self.url)
      .map(|default| default.environment_id.as_str())
  }
}
pub fn resolve(
  argument: Option<&str>,
  data_dir: Option<&Path>,
) -> Result<ResolvedServer> {
  let path = client_config_path(data_dir)?;
  let config = read(&path)?;
  if config.version != 1 {
    bail!(
      "unsupported client configuration version {}",
      config.version
    );
  }
  let (url, source) = if let Some(value) = argument {
    (normalize(value)?, ServerSource::Argument)
  } else if let Ok(value) = env::var(ENV_SERVER_URL) {
    (normalize(&value)?, ServerSource::Environment)
  } else if let Some(value) = &config.server_url {
    (normalize(value)?, ServerSource::Config)
  } else {
    (DEFAULT_PUBLIC_URL.into(), ServerSource::Default)
  };
  Ok(ResolvedServer {
    url,
    source,
    config_path: path,
    config,
  })
}
pub fn read(path: &Path) -> Result<ClientConfig> {
  if !path.exists() {
    return Ok(ClientConfig {
      version: 1,
      server_url: None,
      default_environment: None,
    });
  }
  let text =
    fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
  toml::from_str(&text).context("failed to parse client configuration")
}
pub fn save_default_environment(
  server: &ResolvedServer,
  environment_id: &str,
) -> Result<()> {
  let mut config = server.config.clone();
  config.default_environment = Some(DefaultEnvironment {
    server_url: server.url.clone(),
    environment_id: environment_id.into(),
  });
  write(&server.config_path, &config)
}
pub fn clear_default_environment(server: &ResolvedServer) -> Result<bool> {
  let matches_server = server
    .config
    .default_environment
    .as_ref()
    .is_some_and(|default| default.server_url == server.url);
  if !matches_server {
    return Ok(false);
  }
  let mut config = server.config.clone();
  config.default_environment = None;
  write(&server.config_path, &config)?;
  Ok(true)
}
pub fn write(
  path: &Path,
  config: &ClientConfig,
) -> Result<()> {
  if let Some(parent) = path.parent() {
    ensure_data_dir(parent)?;
  }
  let text = toml::to_string_pretty(config)?;
  crate::utils::private_file::write(path, text.as_bytes(), true)
}
pub fn normalize(value: &str) -> Result<String> {
  if value != value.trim() {
    bail!(invalid_server_address(value));
  }
  let mut url = Url::parse(value).map_err(|error| {
    if error == ParseError::InvalidPort {
      invalid_server_port()
    } else {
      invalid_server_address(value)
    }
  })?;
  if !matches!(url.scheme(), "http" | "https") {
    bail!(
      "The server address must use http:// or https://. For example: https://dopbase.example.com"
    );
  }
  validate_host(value, &url)?;
  if !url.username().is_empty() || url.password().is_some() {
    bail!("server URL must not contain credentials");
  }
  if url.query().is_some() || url.fragment().is_some() {
    bail!("server URL must not contain a query or fragment");
  }
  if url.port() == Some(0) {
    bail!(invalid_server_port());
  }
  let path = url.path().trim_end_matches('/').to_owned();
  url.set_path(&path);
  Ok(url.to_string().trim_end_matches('/').to_owned())
}

pub fn normalize_connect_target(value: &str) -> Result<String> {
  if value != value.trim() || value.is_empty() {
    bail!(invalid_server_address(value));
  }
  if value.contains("://") {
    return normalize(value);
  }

  if let Ok(ip) = value.parse::<std::net::IpAddr>() {
    return normalize(&match ip {
      std::net::IpAddr::V4(ip) => format!("http://{ip}"),
      std::net::IpAddr::V6(ip) => format!("http://[{ip}]"),
    });
  }

  match Url::parse(&format!("http://{value}")) {
    Ok(url) => {
      if matches!(url.host(), Some(Host::Ipv4(_) | Host::Ipv6(_)))
        && url.path() == "/"
        && url.query().is_none()
        && url.fragment().is_none()
        && url.username().is_empty()
        && url.password().is_none()
      {
        return normalize(url.as_str());
      }
      if matches!(url.host(), Some(Host::Domain(host)) if valid_domain(host)) {
        bail!(
          "Server domains must include http:// or https://. For example: https://dopbase.example.com"
        );
      }
    }
    Err(ParseError::InvalidPort) => bail!(invalid_server_port()),
    Err(_) => {}
  }

  bail!(invalid_server_address(value))
}

fn validate_host(
  value: &str,
  url: &Url,
) -> Result<()> {
  match url.host() {
    Some(Host::Domain("localhost")) | Some(Host::Ipv4(_) | Host::Ipv6(_)) => Ok(()),
    Some(Host::Domain(host)) if valid_domain(host) => Ok(()),
    _ => bail!(invalid_server_address(value)),
  }
}

fn valid_domain(host: &str) -> bool {
  host.len() <= 253
    && host.contains('.')
    && host.split('.').all(|label| {
      !label.is_empty()
        && label.len() <= 63
        && label
          .bytes()
          .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        && label
          .as_bytes()
          .first()
          .is_some_and(u8::is_ascii_alphanumeric)
        && label
          .as_bytes()
          .last()
          .is_some_and(u8::is_ascii_alphanumeric)
    })
}

fn invalid_server_address(value: &str) -> anyhow::Error {
  anyhow::anyhow!(
    "The server address \"{value}\" is not valid. Enter an HTTP or HTTPS domain, or an IP address with an optional port. For example: https://dopbase.example.com or 192.168.1.20:8840"
  )
}

fn invalid_server_port() -> anyhow::Error {
  anyhow::anyhow!("The server port must be between 1 and 65535. For example: 192.168.1.20:8840")
}
fn version() -> u32 {
  1
}
