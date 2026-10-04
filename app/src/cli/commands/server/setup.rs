use super::{ServerSetupArgs, handler::load_server_config};
use crate::{
  cli::{client::CliCancelled, output, prompt},
  config::ensure_data_dir,
  modules::{bootstrap::service, common},
  server::{InstanceLock, prepare_instance, require_uninitialized},
  services::token,
};
use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::{
  fmt,
  io::{self, IsTerminal},
  path::{Path, PathBuf},
};
use zeroize::Zeroizing;

#[derive(Debug)]
pub struct SetupCommittedError {
  pub email: String,
  pub data_dir: PathBuf,
  reason: anyhow::Error,
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

#[derive(Serialize)]
struct SetupOutput<'a> {
  initialized: bool,
  admin_id: &'a str,
  email: &'a str,
  data_dir: &'a Path,
  #[serde(skip_serializing_if = "Option::is_none")]
  password: Option<&'a str>,
}

pub(super) async fn execute(
  args: ServerSetupArgs,
  data_dir: Option<PathBuf>,
  json_output: bool,
) -> Result<i32> {
  if args.web && json_output {
    bail!("--json cannot be used with `dopbase server setup --web`");
  }
  if !args.web
    && (args.launch.port.is_some()
      || args.launch.host.is_some()
      || args.launch.public_url.is_some()
      || args.launch.shutdown_grace_seconds.is_some()
      || args.launch.docs
      || args.launch.no_docs)
  {
    bail!("listener options require `dopbase server setup --web`");
  }
  if !args.web && args.email.is_none() {
    if json_output {
      bail!(
        "guided setup does not support --json. Pass --email to generate a password without prompting"
      );
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
      bail!(
        "guided setup requires a terminal. Pass --email to generate a password without prompting"
      );
    }
  }
  let config = load_server_config(args.launch, data_dir, false, false)?;
  if args.web {
    crate::server::serve_setup(config).await?;
    return Ok(0);
  }
  require_uninitialized(&config).await?;
  if InstanceLock::is_held(&config.database_url)? {
    bail!("The instance is in use. Stop the running server or setup process before running setup.");
  }
  let generated = args.email.is_some();
  let email = match args.email {
    Some(value) => {
      common::validate_email(&value).map_err(|_| anyhow::anyhow!("Enter a valid email address."))?
    }
    None => {
      eprintln!("Initialize local instance: {}", config.data_dir.display());
      prompt::email("Root email:", CliCancelled::Setup)?
    }
  };
  let password = Zeroizing::new(if generated {
    token::generate("")?
  } else {
    prompt::new_password(
      "Root password:",
      "Confirm root password:",
      CliCancelled::Setup,
    )?
  });
  let credentials = service::prepare_root(&email, password.to_string())
    .await
    .map_err(cli_error)?;
  ensure_data_dir(&config.data_dir)?;
  let _lock = InstanceLock::acquire(&config.database_url)?;
  require_uninitialized(&config).await?;
  let (db, _crypto) = prepare_instance(&config).await?;
  let provision = async {
    config.ensure_reference_files()?;
    let mut tx = db.pool().begin().await?;
    let admin_id = service::create_root(&mut tx, &credentials, &chrono::Utc::now())
      .await
      .map_err(cli_error)?;
    tx.commit().await?;
    Ok::<String, anyhow::Error>(admin_id)
  }
  .await;
  let admin_id = match provision {
    Ok(admin_id) => admin_id,
    Err(error) => {
      db.close().await;
      return Err(error);
    }
  };
  let finish = async {
    db.checkpoint()
      .await
      .context("failed to checkpoint initialized storage")?;
    let result = SetupOutput {
      initialized: true,
      admin_id: &admin_id,
      email: &email,
      data_dir: &config.data_dir,
      password: generated.then_some(password.as_str()),
    };
    let mut rendered = Zeroizing::new(if json_output {
      serde_json::to_string_pretty(&result)?
    } else {
      let mut text = format!(
        "Server setup complete.\nData:  {}\nEmail: {}\n",
        config.data_dir.display(),
        email
      );
      if generated {
        text.push_str("Password (shown once): ");
        text.push_str(password.as_str());
        text.push_str("\nSave this password before closing the terminal.\n");
      }
      text.push_str("\nStart this instance with `dopbase server start` using the same --data-dir and --config options.\nThen run `dopbase login` to sign in.\n");
      text
    });
    if json_output {
      rendered.push('\n');
    }
    output::print_raw(&rendered).context("failed to deliver setup output")
  }
  .await;
  db.close().await;
  finish.map_err(|reason| {
    anyhow::Error::from(SetupCommittedError {
      email,
      data_dir: config.data_dir,
      reason,
    })
  })?;
  Ok(0)
}

fn cli_error(error: crate::http::HttpError) -> anyhow::Error {
  anyhow::anyhow!(error.errors.into_values().collect::<Vec<_>>().join(" "))
}
