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
  match command {
    ProjectCommand::Create { name } => {
      let data = api
        .request(Method::POST, "/api/v1/projects", Some(json!({"name":name})))
        .await?;
      if json_output {
        output::print_json(&data)?;
      } else {
        output::print_success(&format!(
          "Created project {} ({}).",
          output::string(&data, "name"),
          output::string(&data, "id")
        ));
      }
    }
    ProjectCommand::List => {
      let data = api.request(Method::GET, "/api/v1/projects", None).await?;
      if json_output {
        output::print_json(&data)?;
      } else {
        let rows = output::array(&data)
          .iter()
          .map(|project| {
            vec![
              output::string(project, "name"),
              output::string(project, "id"),
              output::timestamp(project, "updatedAt"),
            ]
          })
          .collect::<Vec<_>>();
        output::print_table(
          &["NAME", "ID", "UPDATED"],
          &rows,
          "No projects found. Create one with: dopbase project create <name>",
          &format!("{} project(s)", rows.len()),
        );
      }
    }
    ProjectCommand::Show { project } => {
      let data = api
        .request(Method::GET, &format!("/api/v1/projects/{project}"), None)
        .await?;
      if json_output {
        output::print_json(&data)?;
      } else {
        output::print_fields(&[
          ("Name:", output::string(&data, "name")),
          ("ID:", output::string(&data, "id")),
          ("Created:", output::timestamp(&data, "createdAt")),
          ("Updated:", output::timestamp(&data, "updatedAt")),
        ]);
      }
    }
    ProjectCommand::Rename { project, new_name } => {
      let data = api
        .request(
          Method::PATCH,
          &format!("/api/v1/projects/{project}"),
          Some(json!({"name":new_name})),
        )
        .await?;
      if json_output {
        output::print_json(&data)?;
      } else {
        output::print_success(&format!(
          "Renamed project to {} ({}).",
          output::string(&data, "name"),
          output::string(&data, "id")
        ));
      }
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
      let data = api
        .request(Method::DELETE, &format!("/api/v1/projects/{project}"), None)
        .await?;
      if json_output {
        output::print_json(&data)?;
      } else {
        output::print_success(&format!("Deleted project {name}."));
        let affected = data.get("affected").unwrap_or(&Value::Null);
        output::print_fields(&[
          (
            "Projects:",
            output::number(affected, "projects").to_string(),
          ),
          (
            "Environments:",
            output::number(affected, "environments").to_string(),
          ),
          ("Secrets:", output::number(affected, "secrets").to_string()),
          ("Tokens:", output::number(affected, "tokens").to_string()),
        ]);
      }
    }
  }
  Ok(0)
}
