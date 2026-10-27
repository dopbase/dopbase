use super::DockerCommand;
use crate::cli::{
  client,
  commands::environment,
  container, docker_env, local_config, output,
  project_resolve::{complete_environment_reference, ensure_docker_available},
};
use crate::{constants::api, models::SecretInput};
use anyhow::{Context, Result, bail};
use reqwest::Method;
use serde_json::Value;
use std::{
  fs,
  io::IsTerminal,
  process::{Command as ProcessCommand, Stdio},
};

pub(crate) async fn execute(
  server: &local_config::ResolvedServer,
  command: DockerCommand,
  json_output: bool,
) -> Result<i32> {
  ensure_docker_available()?;
  match command {
    DockerCommand::EnvFile {
      environment,
      output,
    } => {
      let api = client::recently_authenticated_client(server).await?;
      let reference = complete_environment_reference(&api, &environment).await?;
      let entries = load_entries(&api, &reference).await?;
      let rendered = docker_env::render(&entries)?;
      if let Some(path) = output {
        output::write_private(&path, rendered.as_bytes(), false)?;
        if json_output {
          output::print_json(&serde_json::json!({
            "output": path,
            "secretCount": entries.len(),
          }))?;
        } else {
          output::print_success(&format!(
            "Wrote {} secret(s) to {}.",
            entries.len(),
            path.display()
          ));
        }
      } else {
        print!("{rendered}");
      }
      Ok(0)
    }
    DockerCommand::Run {
      environment,
      command,
    } => run_with_env_file(server, &environment, "run", command, json_output).await,
    DockerCommand::Compose {
      environment,
      command,
    } => run_with_env_file(server, &environment, "compose", command, json_output).await,
    DockerCommand::Exec {
      environment,
      command,
    } => {
      let api = client::recently_authenticated_client(server).await?;
      let reference = complete_environment_reference(&api, &environment).await?;
      let entries = load_entries(&api, &reference).await?;
      let env_file = write_temp_env(&entries)?;
      let mut arguments = vec!["exec".to_string()];
      if std::io::stdin().is_terminal() {
        arguments.push("-it".to_string());
      }
      arguments.push("--env-file".to_string());
      arguments.push(env_file.display().to_string());
      arguments.push(container::container_name());
      arguments.extend(exec_argv(&command));
      let status = ProcessCommand::new("docker")
        .args(&arguments)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("failed to run `docker exec`")?;
      Ok(status.code().unwrap_or(1))
    }
  }
}

async fn run_with_env_file(
  server: &local_config::ResolvedServer,
  environment: &str,
  docker_subcommand: &str,
  command: Vec<String>,
  json_output: bool,
) -> Result<i32> {
  if command.is_empty() || command[0] != "docker" {
    bail!("pass the full Docker command after `--`, starting with `docker`");
  }
  let api = client::recently_authenticated_client(server).await?;
  let reference = complete_environment_reference(&api, environment).await?;
  let entries = load_entries(&api, &reference).await?;
  let env_file = write_temp_env(&entries)?;
  let mut arguments = command;
  let insert_at = arguments
    .iter()
    .position(|part| part == docker_subcommand)
    .map(|index| index + 1)
    .context("could not locate the Docker subcommand in the forwarded command")?;
  arguments.insert(insert_at, env_file.display().to_string());
  arguments.insert(insert_at, "--env-file".to_string());
  if json_output {
    output::print_json(&serde_json::json!({
      "command": arguments,
      "secretCount": entries.len(),
      "environment": reference,
    }))?;
    return Ok(0);
  }
  let program = arguments.first().context("docker command is empty")?;
  let status = ProcessCommand::new(program)
    .args(&arguments[1..])
    .stdin(Stdio::inherit())
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit())
    .status()
    .with_context(|| format!("failed to run `{}`", arguments.join(" ")))?;
  Ok(status.code().unwrap_or(1))
}

/// argv for `docker exec …`: prepend `dopbase` when the user passes
/// `admin` or `server` subcommands without repeating the binary name.
fn exec_argv(command: &[String]) -> Vec<String> {
  if command.is_empty() {
    return vec!["dopbase".into()];
  }
  if command[0] == "dopbase" {
    return command.to_vec();
  }
  if matches!(command[0].as_str(), "admin" | "server") {
    return std::iter::once("dopbase".into())
      .chain(command.iter().cloned())
      .collect();
  }
  command.to_vec()
}

async fn load_entries(
  api: &client::ApiClient,
  reference: &str,
) -> Result<Vec<SecretInput>> {
  let env = environment::resolve_environment(api, reference).await?;
  let data = api
    .request(
      Method::POST,
      &api::secrets::export(environment::env_id(&env)?),
      None,
    )
    .await?;
  data
    .get("entries")
    .and_then(Value::as_array)
    .context("response did not contain entries")?
    .iter()
    .map(|entry| {
      Ok(SecretInput {
        key: entry
          .get("key")
          .and_then(Value::as_str)
          .context("entry has no key")?
          .into(),
        value: entry
          .get("value")
          .and_then(Value::as_str)
          .context("entry has no value")?
          .into(),
      })
    })
    .collect()
}

fn write_temp_env(entries: &[SecretInput]) -> Result<std::path::PathBuf> {
  let rendered = docker_env::render(entries)?;
  let file = tempfile::Builder::new()
    .prefix("dopbase-docker-")
    .suffix(".env")
    .tempfile()
    .context("could not create a temporary Docker env file")?;
  fs::write(file.path(), rendered)
    .with_context(|| format!("could not write {}", file.path().display()))?;
  let (_handle, path) = file
    .keep()
    .map_err(|error| error.error)
    .context("could not persist the temporary Docker env file")?;
  Ok(path)
}

#[cfg(test)]
mod tests {
  use super::exec_argv;

  #[test]
  fn prepends_dopbase_for_admin_and_server() {
    let command = exec_argv(&[
      "admin".to_string(),
      "reset-password".to_string(),
      "a@example.com".to_string(),
    ]);
    assert_eq!(command[0], "dopbase");
    assert_eq!(command[1], "admin");
  }

  #[test]
  fn leaves_other_commands_unchanged() {
    assert_eq!(
      exec_argv(&["printenv".to_string(), "API_KEY".to_string()]),
      vec!["printenv".to_string(), "API_KEY".to_string()],
    );
  }
}
