use crate::{
  config::ServerConfig,
  constants::config::ENV_ROOT_EMAIL,
  modules::{bootstrap::service, common},
  services::db::DbClient,
};
use anyhow::{Result, bail};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, path::PathBuf};
use zeroize::Zeroizing;

#[derive(Debug)]
pub struct SetupCommittedError {
  pub email: String,
  pub data_dir: PathBuf,
  pub(crate) reason: anyhow::Error,
}

impl fmt::Display for SetupCommittedError {
  fn fmt(
    &self,
    formatter: &mut fmt::Formatter<'_>,
  ) -> fmt::Result {
    write!(
      formatter,
      "The instance is initialized, but setup could not finish: {}.\nData: {}\nIf you did not receive the password, run `dopbase admin reset-password {}` using the same instance options. Then run `dopbase server start`.",
      self.reason,
      self.data_dir.display(),
      self.email
    )
  }
}
impl std::error::Error for SetupCommittedError {}

/// Credentials returned only to the command that initializes an instance.
#[derive(Serialize, Deserialize)]
pub(crate) struct GeneratedSetup {
  pub initialized: bool,
  pub admin_id: String,
  pub email: String,
  pub data_dir: PathBuf,
  #[serde(
    serialize_with = "serialize_password",
    deserialize_with = "deserialize_password"
  )]
  pub password: Zeroizing<String>,
}

fn serialize_password<S: Serializer>(
  password: &Zeroizing<String>,
  serializer: S,
) -> Result<S::Ok, S::Error> {
  serializer.serialize_str(password)
}
fn deserialize_password<'de, D: Deserializer<'de>>(
  deserializer: D
) -> Result<Zeroizing<String>, D::Error> {
  String::deserialize(deserializer).map(Zeroizing::new)
}

impl GeneratedSetup {
  pub(crate) fn committed_error(
    &self,
    reason: anyhow::Error,
  ) -> anyhow::Error {
    SetupCommittedError {
      email: self.email.clone(),
      data_dir: self.data_dir.clone(),
      reason,
    }
    .into()
  }

  pub(crate) fn human_output(&self) -> Zeroizing<String> {
    Zeroizing::new(format!(
      "Server setup complete.\nData:  {}\nEmail: {}\nPassword (shown once): {}\nSave this password before closing the terminal.\n\nRun `dopbase login` to sign in.\n",
      self.data_dir.display(),
      self.email,
      self.password.as_str()
    ))
  }
}

pub(crate) fn resolve_email(explicit: Option<String>) -> Result<Option<String>> {
  let (value, source) = match explicit {
    Some(value) => (value, "--email"),
    None => match std::env::var(ENV_ROOT_EMAIL) {
      Ok(value) if value.trim().is_empty() => return Ok(None),
      Ok(value) => (value, ENV_ROOT_EMAIL),
      Err(std::env::VarError::NotPresent) => return Ok(None),
      Err(std::env::VarError::NotUnicode(_)) => {
        bail!("{ENV_ROOT_EMAIL} must contain a valid UTF-8 email address")
      }
    },
  };
  common::validate_email(&value)
    .map(Some)
    .map_err(|_| anyhow::anyhow!("{source} must contain a valid email address."))
}

pub(crate) fn cli_error(error: crate::http::HttpError) -> anyhow::Error {
  anyhow::anyhow!(error.errors.into_values().collect::<Vec<_>>().join(" "))
}

pub(crate) async fn commit_root(
  db: &DbClient,
  credentials: &service::RootCredentials,
) -> Result<String> {
  let mut tx = db.pool().begin().await?;
  let admin_id = service::create_root(&mut tx, credentials, &chrono::Utc::now())
    .await
    .map_err(cli_error)?;
  tx.commit().await?;
  Ok(admin_id)
}

pub(crate) enum StartupMode {
  Runtime,
  Web,
  Generated(String),
}

/// Inspect storage before reading bootstrap input or writing instance files.
pub(crate) async fn startup_mode(config: &ServerConfig) -> Result<StartupMode> {
  if config
    .data_dir
    .join(".factory-reset.pending")
    .try_exists()?
  {
    bail!(
      "A factory reset was interrupted. Run `dopbase server setup` using the same --data-dir and --config options to complete it before starting the server.\nData: {}",
      config.data_dir.display()
    );
  }
  if !super::instance::initialization_required(config).await? {
    return Ok(StartupMode::Runtime);
  }
  match resolve_email(None)? {
    Some(email) => Ok(StartupMode::Generated(email)),
    None if config.web_ui_enabled => Ok(StartupMode::Web),
    None => super::require_initialized(config)
      .await
      .map(|()| StartupMode::Runtime),
  }
}
