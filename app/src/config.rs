use std::{
  env, fs,
  net::SocketAddr,
  path::{Path, PathBuf},
  str::FromStr,
};

use anyhow::{Context, Result, bail};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::constants::config::{
  CLIENT_CONFIG_FILENAME, DATA_DIRECTORY_NAME, DATABASE_FILENAME, ENV_BIND_ADDRESS, ENV_DATA_DIR,
  ENV_DATABASE_URL, ENV_MASTER_KEY_PATH, ENV_PUBLIC_URL, ENV_SHUTDOWN_GRACE_SECONDS,
  MASTER_KEY_FILENAME, SERVER_CONFIG_FILENAME,
};
pub use crate::constants::config::{DEFAULT_BIND_ADDRESS, DEFAULT_PUBLIC_URL};

#[derive(Clone, Debug, Serialize)]
pub struct MasterKeyConfig {
  pub provider: String,
  pub path: PathBuf,
}

#[derive(Clone, Debug, Serialize)]
pub struct ServerConfig {
  #[serde(skip)]
  pub data_dir: PathBuf,
  #[serde(skip)]
  pub config_path: PathBuf,
  pub version: u32,
  pub bind_address: String,
  pub public_url: String,
  pub database_url: String,
  pub shutdown_grace_seconds: u64,
  pub master_key: MasterKeyConfig,
}

#[derive(Clone, Debug, Default)]
pub struct ServerOverrides {
  pub data_dir: Option<PathBuf>,
  pub config_path: Option<PathBuf>,
  pub bind_address: Option<String>,
  pub public_url: Option<String>,
  pub database_url: Option<String>,
  pub shutdown_grace_seconds: Option<u64>,
  pub master_key_path: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
struct ServerConfigFile {
  version: Option<u32>,
  bind_address: Option<String>,
  public_url: Option<String>,
  database_url: Option<String>,
  shutdown_grace_seconds: Option<u64>,
  master_key: Option<MasterKeyConfigFile>,
}

#[derive(Debug, Default, Deserialize)]
struct MasterKeyConfigFile {
  provider: Option<String>,
  path: Option<PathBuf>,
}

#[derive(Debug, Default)]
struct EnvironmentOverrides {
  data_dir: Option<PathBuf>,
  bind_address: Option<String>,
  public_url: Option<String>,
  database_url: Option<String>,
  shutdown_grace_seconds: Option<String>,
  master_key_path: Option<PathBuf>,
}

impl EnvironmentOverrides {
  fn read() -> Self {
    Self {
      data_dir: env::var_os(ENV_DATA_DIR).map(PathBuf::from),
      bind_address: env::var(ENV_BIND_ADDRESS).ok(),
      public_url: env::var(ENV_PUBLIC_URL).ok(),
      database_url: env::var(ENV_DATABASE_URL).ok(),
      shutdown_grace_seconds: env::var(ENV_SHUTDOWN_GRACE_SECONDS).ok(),
      master_key_path: env::var_os(ENV_MASTER_KEY_PATH).map(PathBuf::from),
    }
  }
}

impl Default for MasterKeyConfig {
  fn default() -> Self {
    Self {
      provider: "file".into(),
      path: dopbase_home().join(MASTER_KEY_FILENAME),
    }
  }
}

impl Default for ServerConfig {
  fn default() -> Self {
    Self::for_data_dir(dopbase_home())
  }
}

impl ServerConfig {
  fn for_data_dir(data_dir: PathBuf) -> Self {
    Self {
      config_path: data_dir.join(SERVER_CONFIG_FILENAME),
      database_url: sqlite_url(&data_dir.join(DATABASE_FILENAME)),
      master_key: MasterKeyConfig {
        provider: "file".into(),
        path: data_dir.join(MASTER_KEY_FILENAME),
      },
      data_dir,
      version: 1,
      bind_address: DEFAULT_BIND_ADDRESS.into(),
      public_url: DEFAULT_PUBLIC_URL.into(),
      shutdown_grace_seconds: 10,
    }
  }

  pub fn load(overrides: &ServerOverrides) -> Result<Self> {
    Self::load_with_environment(overrides, EnvironmentOverrides::read())
  }

  fn load_with_environment(
    overrides: &ServerOverrides,
    environment: EnvironmentOverrides,
  ) -> Result<Self> {
    let fallback = dopbase_home();
    let selected_data_dir = overrides
      .data_dir
      .as_deref()
      .or(environment.data_dir.as_deref())
      .unwrap_or(&fallback);
    let data_dir = resolve_path(selected_data_dir)?;
    let config_path = overrides
      .config_path
      .as_deref()
      .map(resolve_path)
      .transpose()?
      .unwrap_or_else(|| data_dir.join(SERVER_CONFIG_FILENAME));
    let mut config = Self::for_data_dir(data_dir);
    config.config_path.clone_from(&config_path);

    if config_path.exists() {
      let contents = fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
      let file: ServerConfigFile = toml::from_str(&contents)
        .with_context(|| format!("failed to parse {}", config_path.display()))?;
      config.apply_file(file);
    }

    config.apply_environment(environment)?;
    config.apply_overrides(overrides);
    config.master_key.path = resolve_path(&config.master_key.path)?;
    config.validate()?;
    Ok(config)
  }

  fn apply_file(
    &mut self,
    file: ServerConfigFile,
  ) {
    if let Some(value) = file.version {
      self.version = value;
    }
    if let Some(value) = file.bind_address {
      self.bind_address = value;
    }
    if let Some(value) = file.public_url {
      self.public_url = value;
    }
    if let Some(value) = file.database_url {
      self.database_url = value;
    }
    if let Some(value) = file.shutdown_grace_seconds {
      self.shutdown_grace_seconds = value;
    }
    if let Some(master_key) = file.master_key {
      if let Some(value) = master_key.provider {
        self.master_key.provider = value;
      }
      if let Some(value) = master_key.path {
        self.master_key.path = value;
      }
    }
  }

  fn apply_environment(
    &mut self,
    environment: EnvironmentOverrides,
  ) -> Result<()> {
    if let Some(value) = environment.bind_address {
      self.bind_address = value;
    }
    if let Some(value) = environment.public_url {
      self.public_url = value;
    }
    if let Some(value) = environment.database_url {
      self.database_url = value;
    }
    if let Some(value) = environment.shutdown_grace_seconds {
      self.shutdown_grace_seconds = value
        .parse()
        .context("invalid DOPBASE_SHUTDOWN_GRACE_SECONDS")?;
    }
    if let Some(value) = environment.master_key_path {
      self.master_key.path = value;
    }
    Ok(())
  }

  fn apply_overrides(
    &mut self,
    overrides: &ServerOverrides,
  ) {
    if let Some(value) = &overrides.bind_address {
      self.bind_address.clone_from(value);
    }
    if let Some(value) = &overrides.public_url {
      self.public_url.clone_from(value);
    }
    if let Some(value) = &overrides.database_url {
      self.database_url.clone_from(value);
    }
    if let Some(value) = overrides.shutdown_grace_seconds {
      self.shutdown_grace_seconds = value;
    }
    if let Some(value) = &overrides.master_key_path {
      self.master_key.path.clone_from(value);
    }
  }

  pub fn bind_addr(&self) -> Result<SocketAddr> {
    self.bind_address.parse().context("invalid bind address")
  }

  pub fn validate(&self) -> Result<()> {
    if self.version != 1 {
      bail!("unsupported server configuration version {}", self.version);
    }
    let bind = SocketAddr::from_str(&self.bind_address).context("invalid bind_address")?;
    let public = Url::parse(&self.public_url).context("invalid public_url")?;
    if !matches!(public.scheme(), "http" | "https") || public.host_str().is_none() {
      bail!("public_url must be an absolute HTTP or HTTPS URL");
    }
    if !bind.ip().is_loopback() && self.public_url == DEFAULT_PUBLIC_URL {
      bail!("public_url must be configured for a non-loopback bind address");
    }
    if !self.database_url.starts_with("sqlite:") {
      bail!("database_url must use SQLite");
    }
    if !(1..=300).contains(&self.shutdown_grace_seconds) {
      bail!("shutdown_grace_seconds must be between 1 and 300");
    }
    if self.master_key.provider != "file" {
      bail!("only the file master-key provider is supported");
    }
    if self.master_key.path.as_os_str().is_empty() {
      bail!("master_key.path is required");
    }
    Ok(())
  }
}

pub fn resolve_data_dir(argument: Option<&Path>) -> Result<PathBuf> {
  let environment = env::var_os(ENV_DATA_DIR).map(PathBuf::from);
  let fallback = dopbase_home();
  resolve_path(argument.or(environment.as_deref()).unwrap_or(&fallback))
}

pub fn default_server_config_path() -> PathBuf {
  dopbase_home().join(SERVER_CONFIG_FILENAME)
}

pub fn default_client_config_path() -> PathBuf {
  dopbase_home().join(CLIENT_CONFIG_FILENAME)
}

pub fn client_config_path(data_dir: Option<&Path>) -> Result<PathBuf> {
  Ok(resolve_data_dir(data_dir)?.join(CLIENT_CONFIG_FILENAME))
}

pub fn dopbase_home() -> PathBuf {
  BaseDirs::new()
    .map(|dirs| dirs.home_dir().join(DATA_DIRECTORY_NAME))
    .unwrap_or_else(|| PathBuf::from(DATA_DIRECTORY_NAME))
}

pub fn ensure_data_dir(path: &Path) -> Result<()> {
  let existed = path.exists();
  fs::create_dir_all(path)
    .with_context(|| format!("failed to create data directory {}", path.display()))?;
  #[cfg(unix)]
  if !existed {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
      .with_context(|| format!("failed to secure data directory {}", path.display()))?;
  }
  Ok(())
}

pub fn sqlite_url(path: &Path) -> String {
  format!("sqlite://{}", path.to_string_lossy().replace('\\', "/"))
}

pub fn database_path(url: &str) -> Result<PathBuf> {
  let path = url
    .strip_prefix("sqlite://")
    .or_else(|| url.strip_prefix("sqlite:"))
    .context("database URL is not SQLite")?;
  Ok(PathBuf::from(path.split('?').next().unwrap_or(path)))
}

fn resolve_path(path: &Path) -> Result<PathBuf> {
  let expanded = expand_home(path);
  if expanded.is_absolute() {
    Ok(expanded)
  } else {
    Ok(
      env::current_dir()
        .context("failed to resolve current directory")?
        .join(expanded),
    )
  }
}

fn expand_home(path: &Path) -> PathBuf {
  let value = path.to_string_lossy();
  let Some(base) = BaseDirs::new() else {
    return path.to_path_buf();
  };
  if value == "~" {
    return base.home_dir().to_path_buf();
  }
  if let Some(rest) = value.strip_prefix("~/") {
    return base.home_dir().join(rest);
  }
  path.to_path_buf()
}

#[cfg(test)]
mod tests {
  use super::*;

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
  fn rejects_remote_bind_without_public_url() {
    let config = ServerConfig {
      bind_address: "0.0.0.0:8376".into(),
      ..Default::default()
    };
    assert!(config.validate().is_err());
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
}
