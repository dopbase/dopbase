use super::EnvCommand;
use crate::cli::{
  client::{self, ApiClient},
  environment_target::EnvironmentTarget,
  local_config,
};
use crate::cli::{output, prompt};
use crate::constants::api as api_paths;
use crate::models::SecretInput;
use anyhow::{Context, Result, bail};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use zeroize::Zeroize;

#[derive(Deserialize)]
struct CloneExport {
  entries: Vec<SecretInput>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CloneImport<'a> {
  mode: &'static str,
  dry_run: bool,
  entries: &'a [SecretInput],
}

struct SensitiveEntries(Vec<SecretInput>);

impl SensitiveEntries {
  fn len(&self) -> usize {
    self.0.len()
  }

  fn as_slice(&self) -> &[SecretInput] {
    &self.0
  }
}

impl Drop for SensitiveEntries {
  fn drop(&mut self) {
    for entry in &mut self.0 {
      entry.value.zeroize();
    }
  }
}

pub(crate) async fn resolve_environment(
  api: &ApiClient,
  reference: &str,
) -> Result<Value> {
  api
    .request(
      Method::GET,
      &api_paths::environments::resolve(reference),
      None,
    )
    .await
}
pub(crate) async fn execute(
  command: EnvCommand,
  server: &local_config::ResolvedServer,
  json_output: bool,
) -> Result<i32> {
  let command = match command {
    EnvCommand::Default { environment, clear } => {
      if clear {
        let cleared = local_config::clear_default_environment(server)?;
        let data = json!({"server_url":server.url,"environment":Value::Null,"cleared":cleared});
        if json_output {
          output::print_json(&data)?;
        } else if cleared {
          output::print_success("Cleared the default environment.");
        } else {
          output::print_text("No default environment was set.");
        }
        return Ok(0);
      }
      let reference = environment.context("default environment is required")?;
      let api = client::human_client(server).await?;
      let environment = resolve_environment(&api, &reference).await?;
      let id = env_id(&environment)?;
      local_config::save_default_environment(server, id)?;
      let data = json!({"server_url":server.url,"environment":environment,"default":true});
      if json_output {
        output::print_json(&data)?;
      } else {
        output::print_success(&format!("Set {reference} as the default environment."));
      }
      return Ok(0);
    }
    command => command,
  };
  let api = client::human_client(server).await?;
  match command {
    EnvCommand::Default { .. } => unreachable!(),
    EnvCommand::Create { target } => {
      let (project, name) = target.into_parts();
      let data = api
        .request(
          Method::POST,
          &api_paths::projects::environments(&project),
          Some(json!({"name":name})),
        )
        .await?;
      if json_output {
        output::print_json(&data)?;
      } else {
        output::print_success(&format!(
          "Created environment {}/{} ({}).",
          output::string(&data, "projectName"),
          output::string(&data, "name"),
          output::string(&data, "id")
        ));
      }
    }
    EnvCommand::Clone {
      source,
      new_name,
      yes,
    } => clone_environment(&api, source, &new_name, yes, json_output).await?,
    EnvCommand::List { project } => {
      let data = list_environments(&api, project.as_deref()).await?;
      if json_output {
        output::print_json(&data)?;
      } else {
        print_environments(&data, project.as_deref());
      }
    }
    EnvCommand::Show { environment } => {
      let env = resolve_environment(&api, &environment).await?;
      let id = env_id(&env)?;
      let data = api
        .request(Method::GET, &api_paths::environments::item(id), None)
        .await?;
      if json_output {
        output::print_json(&data)?;
      } else {
        print_environment(&data);
      }
    }
    EnvCommand::Rename {
      environment,
      new_name,
    } => {
      let env = resolve_environment(&api, &environment).await?;
      let id = env_id(&env)?;
      let data = api
        .request(
          Method::PATCH,
          &api_paths::environments::item(id),
          Some(json!({"name":new_name})),
        )
        .await?;
      if json_output {
        output::print_json(&data)?;
      } else {
        output::print_success(&format!(
          "Renamed environment to {}/{} ({}).",
          output::string(&data, "projectName"),
          output::string(&data, "name"),
          output::string(&data, "id")
        ));
      }
    }
    EnvCommand::Delete { environment, yes } => {
      let env = resolve_environment(&api, &environment).await?;
      let id = env_id(&env)?;
      let secrets = api
        .request(Method::GET, &api_paths::secrets::collection(id), None)
        .await?;
      let tokens = api
        .request(Method::GET, &api_paths::tokens::collection(id), None)
        .await?;
      prompt::confirm(
        &format!(
          "Delete environment {environment}, {} secret(s), and {} token(s)?",
          array_len(&secrets),
          array_len(&tokens)
        ),
        yes,
      )?;
      let data = api
        .request(Method::DELETE, &api_paths::environments::item(id), None)
        .await?;
      if json_output {
        output::print_json(&data)?;
      } else {
        output::print_success(&format!("Deleted environment {environment}."));
        print_affected(&data);
      }
    }
  }
  Ok(0)
}

async fn clone_environment(
  api: &ApiClient,
  source: EnvironmentTarget,
  new_name: &str,
  yes: bool,
  json_output: bool,
) -> Result<()> {
  let (source_project, source_name) = source.into_parts();
  let source_reference = format!("{source_project}/{source_name}");
  let source = resolve_environment(api, &source_reference).await?;
  let source_id = required_string(&source, "id", "source environment")?;
  let project_id = required_string(&source, "projectId", "source environment")?;
  let project_name = required_string(&source, "projectName", "source environment")?;
  let resolved_source_name = required_string(&source, "name", "source environment")?;
  let resolved_source = format!("{project_name}/{resolved_source_name}");
  let destination = format!("{project_name}/{new_name}");

  let environments = list_environments(api, Some(project_id)).await?;
  if output::array(&environments)
    .iter()
    .any(|environment| environment.get("name").and_then(Value::as_str) == Some(new_name))
  {
    bail!("Environment {destination} already exists.");
  }
  let secrets = api
    .request(
      Method::GET,
      &api_paths::secrets::collection(source_id),
      None,
    )
    .await?;
  let secret_count = array_len(&secrets);
  print_clone_summary(json_output, &resolved_source, &destination, secret_count);
  prompt::confirm(
    &format!(
      "Clone {} from {resolved_source} to {destination}?",
      secret_count_label(secret_count)
    ),
    yes,
  )?;
  client::ensure_recent_authentication(api).await?;

  let created = api
    .request(
      Method::POST,
      &api_paths::projects::environments(project_id),
      Some(json!({"name":new_name})),
    )
    .await?;
  let destination_id = env_id(&created)?.to_owned();
  let export = api
    .request(Method::POST, &api_paths::secrets::export(source_id), None)
    .await
    .and_then(parse_clone_export);
  let entries = match export {
    Ok(entries) => SensitiveEntries(entries),
    Err(error) => {
      return Err(
        rollback_clone(
          api,
          &destination_id,
          &destination,
          "Could not read the source secrets",
          error,
        )
        .await,
      );
    }
  };
  let import = CloneImport {
    mode: "merge",
    dry_run: false,
    entries: entries.as_slice(),
  };
  if let Err(error) = api
    .request_json(
      Method::POST,
      &api_paths::secrets::import(&destination_id),
      &import,
    )
    .await
  {
    return Err(
      rollback_clone(
        api,
        &destination_id,
        &destination,
        "Could not copy the source secrets",
        error,
      )
      .await,
    );
  }
  let copied = entries.len();
  drop(entries);

  let environments = list_environments(api, Some(project_id)).await.map_err(|error| {
    anyhow::anyhow!(
      "Cloned {destination}, but could not refresh the environment list: {error:#}\nRun `dopbase env list {project_name}` to view the project environments."
    )
  })?;
  if json_output {
    output::print_json(&environments)?;
  } else {
    output::print_success(&format!(
      "Cloned {} from {resolved_source} to {destination}.",
      secret_count_label(copied)
    ));
    output::print_text("");
    print_environments(&environments, Some(project_name));
  }
  Ok(())
}

async fn list_environments(
  api: &ApiClient,
  project: Option<&str>,
) -> Result<Value> {
  api
    .request(Method::GET, &api_paths::environments::list(project), None)
    .await
}

fn print_environments(
  data: &Value,
  project: Option<&str>,
) {
  let rows = output::array(data)
    .iter()
    .map(|environment| {
      vec![
        output::string(environment, "projectName"),
        output::string(environment, "name"),
        output::string(environment, "id"),
        output::timestamp(environment, "updatedAt"),
      ]
    })
    .collect::<Vec<_>>();
  let empty = project.map_or_else(
    || "No environments found.".to_owned(),
    |project| format!("No environments found for {project}."),
  );
  output::print_table(
    &["PROJECT", "ENVIRONMENT", "ID", "UPDATED"],
    &rows,
    &empty,
    &format!("{} environment(s)", rows.len()),
  );
}

fn print_clone_summary(
  json_output: bool,
  source: &str,
  destination: &str,
  secret_count: usize,
) {
  let fields = [
    ("Source:", source.to_owned()),
    ("Destination:", destination.to_owned()),
    ("Secrets:", secret_count.to_string()),
  ];
  if json_output {
    eprintln!("{}", crate::cli::commands::render_fields(&fields));
  } else {
    output::print_fields(&fields);
  }
}

async fn rollback_clone(
  api: &ApiClient,
  destination_id: &str,
  destination: &str,
  message: &str,
  error: anyhow::Error,
) -> anyhow::Error {
  match api
    .request(
      Method::DELETE,
      &api_paths::environments::item(destination_id),
      None,
    )
    .await
  {
    Ok(_) => {
      anyhow::anyhow!("{message}: {error:#}\nCleanup: removed the new environment {destination}.")
    }
    Err(cleanup_error) => {
      anyhow::anyhow!("{message}: {error:#}\nCleanup failed for {destination}: {cleanup_error:#}")
    }
  }
}

fn parse_clone_export(value: Value) -> Result<Vec<SecretInput>> {
  serde_json::from_value::<CloneExport>(value)
    .map(|export| export.entries)
    .context("source export response did not contain valid entries")
}

fn required_string<'a>(
  value: &'a Value,
  field: &str,
  subject: &str,
) -> Result<&'a str> {
  value
    .get(field)
    .and_then(Value::as_str)
    .with_context(|| format!("{subject} response did not contain {field}"))
}

fn secret_count_label(count: usize) -> String {
  if count == 1 {
    "1 secret".into()
  } else {
    format!("{count} secrets")
  }
}

fn print_environment(value: &Value) {
  output::print_fields(&[
    ("Project:", output::string(value, "projectName")),
    ("Environment:", output::string(value, "name")),
    ("ID:", output::string(value, "id")),
    ("Created:", output::timestamp(value, "createdAt")),
    ("Updated:", output::timestamp(value, "updatedAt")),
  ]);
}

fn print_affected(value: &Value) {
  let affected = value.get("affected").unwrap_or(&Value::Null);
  output::print_fields(&[
    (
      "Environments:",
      output::number(affected, "environments").to_string(),
    ),
    ("Secrets:", output::number(affected, "secrets").to_string()),
    ("Tokens:", output::number(affected, "tokens").to_string()),
  ]);
}

pub(crate) fn env_id(value: &Value) -> Result<&str> {
  value
    .get("id")
    .and_then(Value::as_str)
    .context("environment response did not contain an ID")
}
fn array_len(value: &Value) -> usize {
  value.as_array().map_or(0, Vec::len)
}
