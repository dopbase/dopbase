use crate::cli::client::ApiClient;
use crate::constants::{api as api_paths, tokens::ENVIRONMENT_ID_PREFIX};
use anyhow::{Context, Result, bail};
use reqwest::Method;
use serde_json::Value;
use std::{env, path::Path};

#[derive(Clone, Debug)]
pub struct ResolvedProjectLocation {
  pub name: String,
  pub workspace_name: String,
  pub workspace_root: String,
  pub relative_path: String,
}

pub async fn complete_environment_reference(
  api: &ApiClient,
  reference: &str,
) -> Result<String> {
  if reference.starts_with(ENVIRONMENT_ID_PREFIX) || reference.contains('/') {
    return Ok(reference.to_string());
  }
  let project = resolve_project_from_cwd(api).await?.with_context(|| {
    format!(
      "could not resolve project from the current directory for environment name '{reference}'"
    )
  })?;
  Ok(format!(
    "{}/{}/{}",
    project.workspace_name, project.name, reference
  ))
}

pub async fn resolve_project_from_cwd(api: &ApiClient) -> Result<Option<ResolvedProjectLocation>> {
  let cwd = env::current_dir().context("could not read the current working directory")?;
  let projects = api
    .request(Method::GET, api_paths::projects::COLLECTION, None)
    .await?;
  let workspaces = api
    .request(Method::GET, api_paths::workspaces::COLLECTION, None)
    .await?;
  let cwd = normalize_path(&cwd);
  let mut best: Option<(usize, ResolvedProjectLocation)> = None;
  for project in projects.as_array().into_iter().flatten() {
    let workspace_id = project.get("workspaceId").and_then(Value::as_str);
    let relative = project
      .get("relativePath")
      .and_then(Value::as_str)
      .unwrap_or(".");
    let name = project
      .get("name")
      .and_then(Value::as_str)
      .context("project response did not contain a name")?;
    let Some(workspace_id) = workspace_id else {
      continue;
    };
    let workspace = workspaces
      .as_array()
      .into_iter()
      .flatten()
      .find(|item| item.get("id").and_then(Value::as_str) == Some(workspace_id));
    let Some(workspace) = workspace else {
      continue;
    };
    let workspace_name = workspace
      .get("name")
      .and_then(Value::as_str)
      .context("workspace response did not contain a name")?;
    let root = workspace
      .get("rootPath")
      .and_then(Value::as_str)
      .context("workspace response did not contain rootPath")?;
    let absolute = join_workspace_path(root, relative);
    if !path_contains(&cwd, &absolute) {
      continue;
    }
    let score = absolute.len();
    if best.as_ref().is_none_or(|(current, _)| score > *current) {
      best = Some((
        score,
        ResolvedProjectLocation {
          name: name.into(),
          workspace_name: workspace_name.into(),
          workspace_root: normalize_path_string(root),
          relative_path: relative.replace('\\', "/"),
        },
      ));
    }
  }
  Ok(best.map(|(_, location)| location))
}

fn join_workspace_path(
  root: &str,
  relative: &str,
) -> String {
  let normalized_root = normalize_path_string(root);
  let root = normalized_root.trim_end_matches('/');
  if relative == "." {
    return root.to_string();
  }
  format!("{}/{}", root, relative.replace('\\', "/"))
}

fn normalize_path(path: &Path) -> String {
  path
    .canonicalize()
    .map(|value| normalize_path_string(value.to_string_lossy()))
    .unwrap_or_else(|_| normalize_path_string(path.to_string_lossy()))
}

fn normalize_path_string(value: impl AsRef<str>) -> String {
  let mut normalized = value.as_ref().replace('\\', "/");
  while normalized.len() > 1 && normalized.ends_with('/') {
    normalized.pop();
  }
  normalized
}

fn path_contains(
  cwd: &str,
  root: &str,
) -> bool {
  let cwd = normalize_path_string(cwd);
  let root = normalize_path_string(root);
  if cwd.eq_ignore_ascii_case(&root) {
    return true;
  }
  let prefix = format!("{}/", root);
  cwd.len() > root.len() && cwd[..prefix.len()].eq_ignore_ascii_case(&prefix)
}

pub fn ensure_docker_available() -> Result<()> {
  let status = std::process::Command::new("docker")
    .arg("version")
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::null())
    .status()
    .context("could not run `docker version`")?;
  if status.success() {
    Ok(())
  } else {
    bail!("Docker is not available. Install Docker and ensure `docker` is on PATH.");
  }
}
