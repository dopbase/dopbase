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
use url::Url;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ClientConfig {
    #[serde(default = "version")]
    pub version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_url: Option<String>,
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
pub fn resolve(argument: Option<&str>, data_dir: Option<&Path>) -> Result<ResolvedServer> {
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
        });
    }
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    toml::from_str(&text).context("failed to parse client configuration")
}
pub fn write(path: &Path, config: &ClientConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        ensure_data_dir(parent)?;
    }
    let temporary = path.with_extension("toml.tmp");
    let text = toml::to_string_pretty(config)?;
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    use std::io::Write;
    let mut file = options.open(&temporary)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    fs::rename(temporary, path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}
pub fn normalize(value: &str) -> Result<String> {
    let mut url = Url::parse(value).context("server URL must be absolute")?;
    if !matches!(url.scheme(), "http" | "https") {
        bail!("server URL must use HTTP or HTTPS");
    }
    if !url.username().is_empty() || url.password().is_some() {
        bail!("server URL must not contain credentials");
    }
    if url.query().is_some() || url.fragment().is_some() {
        bail!("server URL must not contain a query or fragment");
    }
    let path = url.path().trim_end_matches('/').to_owned();
    url.set_path(&path);
    Ok(url.to_string().trim_end_matches('/').to_owned())
}
fn version() -> u32 {
    1
}
