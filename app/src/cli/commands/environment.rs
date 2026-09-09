use super::{output, prompt};
use crate::cli::{
  args::EnvCommand,
  client::{self, ApiClient},
  local_config,
};
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
      &format!(
        "/api/v1/environments/resolve?reference={}",
        client::encode_query(reference)
      ),
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
        output::print_value(
          json_output,
          &json!({"server_url":server.url,"environment":Value::Null,"cleared":cleared}),
        );
        return Ok(0);
      }
      let reference = environment.context("default environment is required")?;
      let api = client::human_client(server).await?;
      let environment = resolve_environment(&api, &reference).await?;
      let id = env_id(&environment)?;
      local_config::save_default_environment(server, id)?;
      output::print_value(
        json_output,
        &json!({"server_url":server.url,"environment":environment,"default":true}),
      );
      return Ok(0);
    }
    command => command,
  };
  let api = client::human_client(server).await?;
  let data = match command {
    EnvCommand::Default { .. } => unreachable!(),
    EnvCommand::Create { project, name } => {
      api
        .request(
          Method::POST,
          &format!("/api/v1/projects/{project}/environments"),
          Some(json!({"name":name})),
        )
        .await?
    }
    EnvCommand::List { project } => {
      let path = project.map_or_else(
        || "/api/v1/environments".into(),
        |value| {
          format!(
            "/api/v1/environments?project={}",
            client::encode_query(&value)
          )
        },
      );
      api.request(Method::GET, &path, None).await?
    }
    EnvCommand::Show { environment } => {
      let env = resolve_environment(&api, &environment).await?;
      let id = env_id(&env)?;
      api
        .request(Method::GET, &format!("/api/v1/environments/{id}"), None)
        .await?
    }
    EnvCommand::Rename {
      environment,
      new_name,
    } => {
      let env = resolve_environment(&api, &environment).await?;
      let id = env_id(&env)?;
      api
        .request(
          Method::PATCH,
          &format!("/api/v1/environments/{id}"),
          Some(json!({"name":new_name})),
        )
        .await?
    }
    EnvCommand::Delete { environment, yes } => {
      let env = resolve_environment(&api, &environment).await?;
      let id = env_id(&env)?;
      let secrets = api
        .request(
          Method::GET,
          &format!("/api/v1/environments/{id}/secrets"),
          None,
        )
        .await?;
      let tokens = api
        .request(
          Method::GET,
          &format!("/api/v1/environments/{id}/tokens"),
          None,
        )
        .await?;
      prompt::confirm(
        &format!(
          "Delete environment {environment}, {} secret(s), and {} token(s)?",
          array_len(&secrets),
          array_len(&tokens)
        ),
        yes,
      )?;
      api
        .request(Method::DELETE, &format!("/api/v1/environments/{id}"), None)
        .await?
    }
  };
  output::print_value(json_output, &data);
  Ok(0)
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
