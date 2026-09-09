use super::{output, prompt};
use crate::cli::{args::ProjectCommand, client, local_config};
use anyhow::Result;
use reqwest::Method;
use serde_json::{Value, json};

pub(super) async fn execute(
  command: ProjectCommand,
  server: &local_config::ResolvedServer,
  json_output: bool,
) -> Result<i32> {
  let api = client::human_client(server).await?;
  let data = match command {
    ProjectCommand::Create { name } => {
      api
        .request(Method::POST, "/api/v1/projects", Some(json!({"name":name})))
        .await?
    }
    ProjectCommand::List => api.request(Method::GET, "/api/v1/projects", None).await?,
    ProjectCommand::Show { project } => {
      api
        .request(Method::GET, &format!("/api/v1/projects/{project}"), None)
        .await?
    }
    ProjectCommand::Rename { project, new_name } => {
      api
        .request(
          Method::PATCH,
          &format!("/api/v1/projects/{project}"),
          Some(json!({"name":new_name})),
        )
        .await?
    }
    ProjectCommand::Delete { project, yes } => {
      let detail = api
        .request(Method::GET, &format!("/api/v1/projects/{project}"), None)
        .await?;
      let name = detail
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(&project);
      let environments = api
        .request(
          Method::GET,
          &format!(
            "/api/v1/environments?project={}",
            client::encode_query(&project)
          ),
          None,
        )
        .await?;
      let count = environments.as_array().map_or(0, Vec::len);
      prompt::confirm(
        &format!("Delete project {name} and its {count} environment(s)?"),
        yes,
      )?;
      api
        .request(Method::DELETE, &format!("/api/v1/projects/{project}"), None)
        .await?
    }
  };
  output::print_value(json_output, &data);
  Ok(0)
}
