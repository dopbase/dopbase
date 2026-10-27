use crate::constants::{tokens::ENVIRONMENT_ID_PREFIX, workspaces::DEFAULT_WORKSPACE_NAME};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnvironmentReference {
  Id(String),
  Legacy {
    project: String,
    environment: String,
  },
  Qualified {
    workspace: String,
    project_path: String,
    environment: String,
  },
}

pub fn parse(reference: &str) -> Option<EnvironmentReference> {
  let reference = crate::utils::cli_path_argument::normalize(reference);
  if reference.is_empty() {
    return None;
  }
  if reference.starts_with(ENVIRONMENT_ID_PREFIX) {
    return Some(EnvironmentReference::Id(reference));
  }
  let reference = if reference.starts_with('/') {
    let path = reference.trim_start_matches('/');
    if path.is_empty() {
      return None;
    }
    let segments = path
      .split('/')
      .map(str::trim)
      .filter(|part| !part.is_empty())
      .count();
    if segments < 2 {
      return None;
    }
    format!("{}/{}", DEFAULT_WORKSPACE_NAME, path)
  } else {
    reference
  };
  let parts = reference
    .split('/')
    .map(str::trim)
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>();
  match parts.len() {
    0 | 1 => None,
    2 => Some(EnvironmentReference::Legacy {
      project: parts[0].to_owned(),
      environment: parts[1].to_owned(),
    }),
    _ => Some(EnvironmentReference::Qualified {
      workspace: parts[0].to_owned(),
      project_path: parts[1..parts.len() - 1].join("/"),
      environment: parts[parts.len() - 1].to_owned(),
    }),
  }
}

pub fn normalize_path(value: &str) -> String {
  value.replace('\\', "/")
}
