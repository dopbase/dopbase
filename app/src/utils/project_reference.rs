use crate::constants::{tokens::PROJECT_ID_PREFIX, workspaces::DEFAULT_WORKSPACE_NAME};
use crate::utils::{cli_path_argument, slug};

/// Turn a CLI project reference into the API `project_ref` (workspace-qualified when needed).
pub fn to_api_reference(reference: &str) -> Result<String, String> {
  let reference = cli_path_argument::normalize(reference);
  if reference.is_empty() {
    return Err("project reference cannot be empty".into());
  }
  if reference.starts_with(PROJECT_ID_PREFIX) {
    return Ok(reference);
  }
  if reference.starts_with('/') {
    let path = reference.trim_start_matches('/');
    if path.is_empty() {
      return Err("project reference cannot be '/' alone".into());
    }
    validate_path_segments(path)?;
    return Ok(format!("{}/{}", DEFAULT_WORKSPACE_NAME, path));
  }
  if reference.contains('/') {
    validate_path_segments(&reference)?;
    return Ok(reference);
  }
  if !slug::is_valid(&reference) {
    return Err(
      "project reference must be a project ID, WORKSPACE/PROJECT, /PROJECT for the default workspace, or a lowercase slug of at most 63 characters"
        .into(),
    );
  }
  Ok(reference)
}

/// Parsed `dopbase project create` target (optional workspace directory binding).
pub struct ProjectCreateTarget {
  pub name: String,
  pub workspace: Option<String>,
  pub relative_path: Option<String>,
}

pub fn parse_create_target(value: &str) -> Result<ProjectCreateTarget, String> {
  let value = value.trim();
  if value.is_empty() {
    return Err("project name cannot be empty".into());
  }
  let api_ref = to_api_reference(value)?;
  if api_ref.starts_with(PROJECT_ID_PREFIX) {
    return Err("project create expects a name, not a project ID".into());
  }
  if let Some((workspace, project_path)) = api_ref.split_once('/') {
    validate_path_segments(&api_ref)?;
    let name = project_path
      .rsplit('/')
      .next()
      .ok_or_else(|| "project path cannot be empty".to_string())?;
    if !slug::is_valid(name) {
      return Err("project name must be a lowercase slug of at most 63 characters".into());
    }
    return Ok(ProjectCreateTarget {
      name: name.to_owned(),
      workspace: Some(workspace.to_owned()),
      relative_path: Some(project_path.to_owned()),
    });
  }
  if !slug::is_valid(&api_ref) {
    return Err("project name must be a lowercase slug of at most 63 characters".into());
  }
  Ok(ProjectCreateTarget {
    name: api_ref,
    workspace: None,
    relative_path: None,
  })
}

fn validate_path_segments(path: &str) -> Result<(), String> {
  for segment in path.split('/') {
    if segment.is_empty() {
      return Err("project reference path cannot contain empty segments".into());
    }
    if !slug::is_valid(segment) {
      return Err(
        "each workspace and project path segment must be a lowercase slug of at most 63 characters"
          .into(),
      );
    }
  }
  Ok(())
}
