use super::RunArgs;
use super::cache::{self as runtime_cache, RuntimeSource};
use super::selection::{SelectionOptions, select};
use crate::cli::{client, local_config};
use crate::constants::{config::ENV_RUN_ENVIRONMENT, tokens::RUNNER_TOKEN_PREFIX};
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
  // A runner token reaches one environment, so the token decides it.
  let runner = api.credential_token()?.starts_with(RUNNER_TOKEN_PREFIX);
  let (loaded, saved_default) = if runner {
    let expected = runner_environment(environment, env::var(ENV_RUN_ENVIRONMENT))?;
    (
      runtime_cache::load_runner(server, &api, expected.as_deref()).await,
      None,
    )
  } else {
    let selection = run_environment(
      environment,
      env::var(ENV_RUN_ENVIRONMENT),
      server.default_environment(),
    )?;
    let saved_default =
      (selection.source == RunEnvironmentSource::Default).then(|| selection.reference.clone());
    (
      runtime_cache::load(server, &api, &selection.reference).await,
      saved_default,
    )
  };
  let loaded = match (loaded, saved_default) {
    (Err(error), _) if client::is_authentication_error(&error) => {
      bail!(
        "Dopbase authentication failed. Run `dopbase login` again or set a valid DOPBASE_TOKEN."
      )
    }
    (Err(error), Some(reference)) if error.to_string().contains("ENVIRONMENT_NOT_FOUND") => {
      bail!(
        "Saved default environment {reference} is unavailable. Set a new one with `dopbase env default <ENVIRONMENT_REF>` or clear it with `dopbase env default --clear`."
      )
    }
    (result, _) => result?,
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
  // A runner token's output usually lands in CI logs, so it keeps the
  // token's environment to itself.
  if runner {
    eprintln!("Dopbase: {injected}, source={}", loaded.source.as_str());
  } else {
    eprintln!(
      "Dopbase: {}/{} ({}), {injected}, source={}",
      loaded.project,
      loaded.environment,
      loaded.environment_id,
      loaded.source.as_str(),
    );
  }
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
  let source = if argument.is_some() {
    RunEnvironmentSource::Argument
  } else {
    RunEnvironmentSource::Environment
  };
  if let Some(reference) = runner_environment(argument, environment)? {
    return Ok(RunEnvironment { reference, source });
  }
  if let Some(reference) = default {
    return Ok(RunEnvironment {
      reference: reference.into(),
      source: RunEnvironmentSource::Default,
    });
  }
  bail!("No default environment is set. Set one with: dopbase env default <ENVIRONMENT_REF>")
}

/// The reference the argument or `DOPBASE_ENV` names, if any. With a runner
/// token this is all there is: it only checks the token's environment, and
/// the saved default, which belongs to people, is ignored.
pub fn runner_environment(
  argument: Option<String>,
  environment: Result<String, env::VarError>,
) -> Result<Option<String>> {
  if argument.is_some() {
    return Ok(argument);
  }
  match environment {
    Ok(reference) if reference.is_empty() => bail!("DOPBASE_ENV is set but empty"),
    Ok(reference) => Ok(Some(reference)),
    Err(env::VarError::NotUnicode(_)) => bail!("DOPBASE_ENV contains invalid Unicode"),
    Err(env::VarError::NotPresent) => Ok(None),
  }
}
