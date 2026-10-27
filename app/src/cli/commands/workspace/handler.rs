use super::WorkspaceCommand;
use crate::cli::{client, output, prompt};
use crate::{cli::local_config, constants::api as api_paths};
use anyhow::{Context, Result, bail};
use reqwest::Method;
use serde_json::{Value, json};

pub(crate) async fn execute(
  command: WorkspaceCommand,
  server: &local_config::ResolvedServer,
  json_output: bool,
) -> Result<i32> {
  let api = client::human_client(server).await?;
  match command {
    WorkspaceCommand::Create { name, root_path } => {
      let body = match root_path {
        Some(path) => json!({"name": name, "rootPath": path}),
        None => json!({"name": name}),
      };
      let data = api
        .request(Method::POST, api_paths::workspaces::COLLECTION, Some(body))
        .await?;
      if json_output {
        output::print_json(&data)?;
      } else {
        output::print_success(&format!(
          "Created workspace {} ({}).",
          output::string(&data, "name"),
          output::string(&data, "id")
        ));
        output::print_fields(&[("Root:", output::string(&data, "rootPath"))]);
      }
    }
    WorkspaceCommand::List => {
      let data = api
        .request(Method::GET, api_paths::workspaces::COLLECTION, None)
        .await?;
      if json_output {
        output::print_json(&data)?;
      } else {
        let rows = output::array(&data)
          .iter()
          .map(|workspace| {
            vec![
              output::string(workspace, "name"),
              output::string(workspace, "id"),
              output::string(workspace, "rootPath"),
              output::timestamp(workspace, "updatedAt"),
            ]
          })
          .collect::<Vec<_>>();
        output::print_table(
          &["NAME", "ID", "ROOT", "UPDATED"],
          &rows,
          "No workspaces found. Create one with: dopbase workspace create <name>",
          &format!("{} workspace(s)", rows.len()),
        );
      }
    }
    WorkspaceCommand::Show { workspace } => {
      let data = resolve_workspace(&api, &workspace).await?;
      if json_output {
        output::print_json(&data)?;
      } else {
        output::print_fields(&[
          ("Name:", output::string(&data, "name")),
          ("ID:", output::string(&data, "id")),
          ("Root:", output::string(&data, "rootPath")),
          ("Created:", output::timestamp(&data, "createdAt")),
          ("Updated:", output::timestamp(&data, "updatedAt")),
        ]);
      }
    }
    WorkspaceCommand::Update {
      workspace,
      name,
      root_path,
    } => {
      let current = resolve_workspace(&api, &workspace).await?;
      let id = workspace_id(&current)?;
      let data = api
        .request(
          Method::PATCH,
          &api_paths::workspaces::item(id),
          Some(json!({"name": name, "rootPath": root_path})),
        )
        .await?;
      if json_output {
        output::print_json(&data)?;
      } else {
        output::print_success(&format!(
          "Updated workspace {} ({}).",
          output::string(&data, "name"),
          output::string(&data, "id")
        ));
      }
    }
    WorkspaceCommand::Delete { workspace, yes } => {
      let current = resolve_workspace(&api, &workspace).await?;
      let id = workspace_id(&current)?;
      let name = output::string(&current, "name");
      prompt::confirm(&format!("Delete workspace {name}?"), yes)?;
      let data = api
        .request(Method::DELETE, &api_paths::workspaces::item(id), None)
        .await?;
      if json_output {
        output::print_json(&data)?;
      } else {
        output::print_success(&format!("Deleted workspace {name}."));
      }
    }
  }
  Ok(0)
}

async fn list_workspaces(api: &client::ApiClient) -> Result<Value> {
  api
    .request(Method::GET, api_paths::workspaces::COLLECTION, None)
    .await
}

async fn resolve_workspace(
  api: &client::ApiClient,
  reference: &str,
) -> Result<Value> {
  let workspaces = list_workspaces(api).await?;
  let reference = reference.trim();
  let items = output::array(&workspaces);
  let matches = items
    .iter()
    .filter(|workspace| {
      workspace.get("id").and_then(Value::as_str) == Some(reference)
        || workspace.get("name").and_then(Value::as_str) == Some(reference)
    })
    .collect::<Vec<_>>();
  match matches.len() {
    0 => bail!("workspace not found: {reference}"),
    1 => Ok(matches[0].clone()),
    _ => bail!("multiple workspaces match {reference}; use a workspace ID"),
  }
}

pub(crate) async fn resolve_workspace_id(
  api: &client::ApiClient,
  reference: &str,
) -> Result<String> {
  workspace_id(&resolve_workspace(api, reference).await?).map(str::to_owned)
}

fn workspace_id(value: &Value) -> Result<&str> {
  value
    .get("id")
    .and_then(Value::as_str)
    .context("workspace response did not contain an ID")
}
