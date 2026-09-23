use super::RunArgs;
use super::cache::{self as runtime_cache, RuntimeSource};
use crate::cli::{client, local_config};
use crate::constants::config::ENV_RUN_ENVIRONMENT;
use anyhow::{Context, Result, bail};
use std::env;

pub(crate) async fn execute(
  server: &local_config::ResolvedServer,
  args: RunArgs,
) -> Result<i32> {
  let RunArgs {
    environment,
    token,
    command,
  } = args;
  let api = client::any_authenticated_client(server, token).await?;
  let selection = run_environment(
    environment,
    env::var(ENV_RUN_ENVIRONMENT),
    server.default_environment(),
  )?;
  let loaded = runtime_cache::load(server, &api, &selection.reference).await;
  let loaded = match loaded {
    Err(error) if client::is_authentication_error(&error) => {
      bail!(
        "Dopbase authentication failed. Run `dopbase login` again or set a valid DOPBASE_TOKEN."
      )
    }
    Err(error)
      if selection.source == RunEnvironmentSource::Default
        && error.to_string().contains("ENVIRONMENT_NOT_FOUND") =>
    {
      bail!(
        "Saved default environment {} is unavailable. Set a new one with `dopbase env default <ENVIRONMENT_REF>` or clear it with `dopbase env default --clear`.",
        selection.reference
      )
    }
    result => result?,
  };
  match &loaded.source {
    RuntimeSource::Live { cache_warning } => {
      if let Some(warning) = cache_warning {
        eprintln!("Dopbase warning: {warning}");
      }
    }
    RuntimeSource::Cache {
      fetched_at,
      age,
      reason,
    } => {
      eprintln!(
        "Dopbase warning: {reason}. Using encrypted cache fetched at {fetched_at} ({age} old)."
      );
    }
  }
  eprintln!(
    "Dopbase: {}/{} ({}), {} key(s), source={}",
    loaded.project,
    loaded.environment,
    loaded.environment_id,
    loaded.entries.len(),
    loaded.source.as_str(),
  );
  let program = command.first().context("run requires a child command")?;
  let mut child_command = tokio::process::Command::new(program);
  child_command
    .args(&command[1..])
    .env_remove("DOPBASE_TOKEN");
  for entry in loaded.entries {
    child_command.env(entry.key, entry.value);
  }
  #[cfg(unix)]
  {
    use std::os::unix::process::CommandExt;

    let error = child_command.as_std_mut().exec();
    Err(error).with_context(|| format!("failed to start {program}"))
  }
  #[cfg(not(unix))]
  {
    let mut child = child_command
      .spawn()
      .with_context(|| format!("failed to start {program}"))?;
    Ok(child.wait().await?.code().unwrap_or(1))
  }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RunEnvironmentSource {
  Argument,
  Environment,
  Default,
}

pub struct RunEnvironment {
  pub reference: String,
  source: RunEnvironmentSource,
}

pub fn run_environment(
  argument: Option<String>,
  environment: Result<String, env::VarError>,
  default: Option<&str>,
) -> Result<RunEnvironment> {
  if let Some(reference) = argument {
    return Ok(RunEnvironment {
      reference,
      source: RunEnvironmentSource::Argument,
    });
  }
  match environment {
    Ok(reference) if reference.is_empty() => bail!("DOPBASE_ENV is set but empty"),
    Ok(reference) => {
      return Ok(RunEnvironment {
        reference,
        source: RunEnvironmentSource::Environment,
      });
    }
    Err(env::VarError::NotUnicode(_)) => bail!("DOPBASE_ENV contains invalid Unicode"),
    Err(env::VarError::NotPresent) => {}
  }
  if let Some(reference) = default {
    return Ok(RunEnvironment {
      reference: reference.into(),
      source: RunEnvironmentSource::Default,
    });
  }
  bail!("No default environment is set. Set one with: dopbase env default <ENVIRONMENT_REF>")
}
