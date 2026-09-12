use super::{output, prompt};
use crate::cli::{
  args::EnvCommand,
  client::{self, ApiClient},
  local_config,
};
use crate::constants::api as api_paths;
use anyhow::{Context, Result};
use reqwest::Method;
use serde_json::{Value, json};

pub(super) async fn resolve_environment(
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
pub(super) async fn execute(
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
    EnvCommand::List { project } => {
      let path = api_paths::environments::list(project.as_deref());
      let data = api.request(Method::GET, &path, None).await?;
      if json_output {
        output::print_json(&data)?;
      } else {
        let rows = output::array(&data)
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
        let empty = project.as_ref().map_or_else(
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

pub(super) fn env_id(value: &Value) -> Result<&str> {
  value
    .get("id")
    .and_then(Value::as_str)
    .context("environment response did not contain an ID")
}
fn array_len(value: &Value) -> usize {
  value.as_array().map_or(0, Vec::len)
}
