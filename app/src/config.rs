use std::{
  env, fs,
  net::{IpAddr, SocketAddr},
  path::{Path, PathBuf},
  str::FromStr,
};

use anyhow::{Context, Result, bail};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::constants::config::{
  CLIENT_CONFIG_FILENAME, DATA_DIRECTORY_NAME, DATABASE_FILENAME, DEFAULT_PORT, ENV_BIND_ADDRESS,
  ENV_DATA_DIR, ENV_DATABASE_URL, ENV_DOCS, ENV_HOST, ENV_MASTER_KEY_PATH, ENV_PORT,
  ENV_PUBLIC_URL, ENV_SHUTDOWN_GRACE_SECONDS, MASTER_KEY_FILENAME, SERVER_CONFIG_FILENAME,
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
  #[serde(skip)]
  pub docs_enabled: bool,
  /// True when this process runs as a supervised background server and must
  /// manage the PID file.
  #[serde(skip)]
  pub daemonized: bool,
  pub master_key: MasterKeyConfig,
  /// True when bind_address came from an explicit source (file, environment,
  /// or `--bind-address`). The ergonomic `port`/`host` selectors refuse to
  /// mix with it so there is exactly one source of truth for the socket.
  #[serde(skip)]
  pub bind_address_explicit: bool,
  /// True when public_url came from an explicit source. Otherwise it is
  /// derived from the bind address (`http://localhost:{port}` for loopback;
  /// remote binds fail closed and require an explicit value).
  #[serde(skip)]
  pub public_url_explicit: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ServerOverrides {
  pub data_dir: Option<PathBuf>,
  pub config_path: Option<PathBuf>,
  pub bind_address: Option<String>,
  pub public_url: Option<String>,
  pub port: Option<u16>,
  pub host: Option<String>,
  pub database_url: Option<String>,
  pub shutdown_grace_seconds: Option<u64>,
  pub docs: Option<bool>,
  pub background: bool,
  pub supervised: bool,
  pub master_key_path: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
struct ServerConfigFile {
  version: Option<u32>,
  bind_address: Option<String>,
  public_url: Option<String>,
  port: Option<u16>,
  host: Option<String>,
  database_url: Option<String>,
  shutdown_grace_seconds: Option<u64>,
  docs: Option<bool>,
  master_key: Option<MasterKeyConfigFile>,
}

#[derive(Debug, Default, Deserialize)]
struct MasterKeyConfigFile {
  provider: Option<String>,
  path: Option<PathBuf>,
}

/// `DOPBASE_*` environment overrides, structured so tests (and any embedder)
/// can resolve configuration from injected values instead of process state.
#[derive(Debug, Default)]
pub struct EnvironmentOverrides {
  pub data_dir: Option<PathBuf>,
  pub bind_address: Option<String>,
  pub public_url: Option<String>,
  pub port: Option<String>,
  pub host: Option<String>,
  pub database_url: Option<String>,
  pub shutdown_grace_seconds: Option<String>,
  pub docs: Option<String>,
  pub master_key_path: Option<PathBuf>,
}

impl EnvironmentOverrides {
  fn read() -> Self {
    Self {
      data_dir: env::var_os(ENV_DATA_DIR).map(PathBuf::from),
      bind_address: env::var(ENV_BIND_ADDRESS).ok(),
      public_url: env::var(ENV_PUBLIC_URL).ok(),
      port: env::var(ENV_PORT).ok(),
      host: env::var(ENV_HOST).ok(),
      database_url: env::var(ENV_DATABASE_URL).ok(),
      shutdown_grace_seconds: env::var(ENV_SHUTDOWN_GRACE_SECONDS).ok(),
      docs: env::var(ENV_DOCS).ok(),
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
      docs_enabled: false,
      daemonized: false,
      bind_address_explicit: false,
      public_url_explicit: false,
    }
  }

  pub fn load(overrides: &ServerOverrides) -> Result<Self> {
    Self::load_with_environment(overrides, EnvironmentOverrides::read())
  }

  pub fn load_with_environment(
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

    let file_port;
    let file_host;
    if config_path.exists() {
      let contents = fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
      let file: ServerConfigFile = toml::from_str(&contents)
        .with_context(|| format!("failed to parse {}", config_path.display()))?;
      file_port = file.port;
      file_host = file.host.clone();
      config.apply_file(file);
    } else {
      file_port = None;
      file_host = None;
    }

    let environment_port = environment
      .port
      .clone()
      .map(|value| value.parse::<u16>().context("invalid DOPBASE_PORT"))
      .transpose()?;
    let environment_host = environment.host.clone();
    config.apply_environment(environment)?;
    config.apply_overrides(overrides);

    // The ergonomic `port`/`host` selectors (CLI > environment > file) compose
    // the bind address; an explicit `bind_address` from any source excludes
    // them so there is exactly one source of truth for the socket.
    config.select_bind_address(
      overrides.port.or(environment_port).or(file_port),
      overrides.host.clone().or(environment_host).or(file_host),
    )?;
    config.derive_public_url()?;
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
      self.bind_address_explicit = true;
    }
    if let Some(value) = file.public_url {
      self.public_url = value;
      self.public_url_explicit = true;
    }
    if let Some(value) = file.database_url {
      self.database_url = value;
    }
    if let Some(value) = file.shutdown_grace_seconds {
      self.shutdown_grace_seconds = value;
    }
    if let Some(value) = file.docs {
      self.docs_enabled = value;
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
      self.bind_address_explicit = true;
    }
    if let Some(value) = environment.public_url {
      self.public_url = value;
      self.public_url_explicit = true;
    }
    if let Some(value) = environment.database_url {
      self.database_url = value;
    }
    if let Some(value) = environment.shutdown_grace_seconds {
      self.shutdown_grace_seconds = value
        .parse()
        .context("invalid DOPBASE_SHUTDOWN_GRACE_SECONDS")?;
    }
    if let Some(value) = environment.docs {
      self.docs_enabled = value.parse().context("invalid DOPBASE_DOCS")?;
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
      self.bind_address_explicit = true;
    }
    if let Some(value) = &overrides.public_url {
      self.public_url.clone_from(value);
      self.public_url_explicit = true;
    }
    if let Some(value) = &overrides.database_url {
      self.database_url.clone_from(value);
    }
    if let Some(value) = overrides.shutdown_grace_seconds {
      self.shutdown_grace_seconds = value;
    }
    if let Some(value) = overrides.docs {
      self.docs_enabled = value;
    }
    self.daemonized = overrides.background || overrides.supervised;
    if let Some(value) = &overrides.master_key_path {
      self.master_key.path.clone_from(value);
    }
  }

  pub fn bind_addr(&self) -> Result<SocketAddr> {
    self.bind_address.parse().context("invalid bind address")
  }

  /// Compose the bind address from the ergonomic `port`/`host` selectors.
  /// Either may be omitted and falls back to the default. An explicit
  /// `bind_address` (file, environment, or `--bind-address`) excludes these
  /// selectors entirely so the socket has exactly one source of truth.
  fn select_bind_address(
    &mut self,
    port: Option<u16>,
    host: Option<String>,
  ) -> Result<()> {
    if port.is_none() && host.is_none() {
      return Ok(());
    }
    if self.bind_address_explicit {
      bail!("configure either port/host or bind_address, not both");
    }
    let host = host.as_deref().unwrap_or("127.0.0.1");
    let ip = match host {
      "localhost" => IpAddr::from([127, 0, 0, 1]),
      other => other.parse::<IpAddr>().with_context(|| {
        format!("invalid host \"{other}\": use an IP address like 127.0.0.1 or 0.0.0.0")
      })?,
    };
    self.bind_address = format!("{ip}:{}", port.unwrap_or(DEFAULT_PORT));
    Ok(())
  }

  /// When public_url was not configured explicitly, derive it from the bind
  /// address so `--port` alone is enough for local development. Remote binds
  /// fail closed: Dopbase does not trust the Host header and will not guess
  /// its public address.
  fn derive_public_url(&mut self) -> Result<()> {
    if self.public_url_explicit {
      return Ok(());
    }
    let bind: SocketAddr = self.bind_address.parse().context("invalid bind_address")?;
    if !bind.ip().is_loopback() {
      bail!(
        "public_url is required when binding beyond loopback (bind_address = \"{}\").\n\
         Dopbase does not trust the Host header, so it cannot guess its public address.\n\
         Set --public-url (or public_url in server.toml) to the URL clients will use,\n\
         e.g. --public-url https://dopbase.example.com",
        self.bind_address
      );
    }
    self.public_url = format!("http://localhost:{}", bind.port());
    Ok(())
  }

  pub fn validate(&self) -> Result<()> {
    if self.version != 1 {
      bail!("unsupported server configuration version {}", self.version);
    }
    let bind = SocketAddr::from_str(&self.bind_address).context("invalid bind_address")?;
    if self.public_url_explicit {
      let public = Url::parse(&self.public_url).context("invalid public_url")?;
      if !matches!(public.scheme(), "http" | "https") || public.host_str().is_none() {
        bail!("public_url must be an absolute HTTP or HTTPS URL");
      }
    } else if !bind.ip().is_loopback() {
      // load() derives public_url for loopback binds before validating, so a
      // non-loopback bind reaching here means the value was never configured.
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
