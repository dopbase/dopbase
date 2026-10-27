use super::RunArgs;
use super::cache::{self as runtime_cache, RuntimeSource};
use super::selection::{SelectionOptions, select};
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
    only,
    allow_missing,
    preserve_env,
    command,
  } = args;
  let options = SelectionOptions {
    only,
    allow_missing,
    preserve_env,
  };
  let api = client::any_authenticated_client(server).await?;
  let environment = match environment {
    Some(reference) => {
      Some(crate::cli::project_resolve::complete_environment_reference(&api, &reference).await?)
    }
    None => None,
  };
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
  let loaded_count = loaded.entries.len();
  let selection = select(loaded.entries, &options, |key| env::var_os(key).is_some())?;
  if !selection.skipped.is_empty() {
    eprintln!(
      "Dopbase warning: skipped reserved name(s): {}",
      selection.skipped.join(", ")
    );
  }
  if !selection.blocked.is_empty() {
    eprintln!(
      "Dopbase warning: skipped name(s) that can load code into the child: {}. Name them in --only to inject them.",
      selection.blocked.join(", ")
    );
  }
  if !selection.missing.is_empty() {
    eprintln!(
      "Dopbase warning: --only key(s) not stored: {}",
      selection.missing.join(", ")
    );
  }
  if !selection.dangerous.is_empty() {
    eprintln!(
      "Dopbase warning: these names can change how programs load or run: {}",
      selection.dangerous.join(", ")
    );
  }
  if !selection.preserved.is_empty() {
    eprintln!(
      "Dopbase: kept existing value for: {}",
      selection.preserved.join(", ")
    );
  }
  let injected = if selection.inject.len() == loaded_count {
    format!("{loaded_count} key(s)")
  } else {
    format!("{} of {loaded_count} key(s)", selection.inject.len())
  };
  eprintln!(
    "Dopbase: {}/{} ({}), {injected}, source={}",
    loaded.project,
    loaded.environment,
    loaded.environment_id,
    loaded.source.as_str(),
  );
  let program = command.first().context("run requires a child command")?;
  let mut child_command = tokio::process::Command::new(program);
  child_command
    .args(&command[1..])
    .env_remove("DOPBASE_TOKEN");
  for entry in selection.inject {
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
