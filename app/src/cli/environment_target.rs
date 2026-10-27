use crate::{
  constants::tokens::PROJECT_ID_PREFIX,
  utils::{environment_reference::parse, slug},
};

const TARGET_EXAMPLE: &str = "payment-service/local";
const QUALIFIED_EXAMPLE: &str = "workspace1/api/staging";
const DEFAULT_WORKSPACE_EXAMPLE: &str = "/storefront/staging";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvironmentTarget {
  project: String,
  environment: String,
}

impl EnvironmentTarget {
  pub fn into_parts(self) -> (String, String) {
    (self.project, self.environment)
  }

  pub fn environment_reference(&self) -> String {
    format!("{}/{}", self.project, self.environment)
  }
}

pub fn parse_init(value: &str) -> Result<EnvironmentTarget, String> {
  let target = parse_target(value)?;
  if target.project.contains('/') {
    return Err(
      "init uses PROJECT/ENVIRONMENT only; workspace-qualified targets are not supported".into(),
    );
  }
  if !slug::is_valid(&target.project) {
    return Err(
      "project name must be a lowercase slug of at most 63 characters, project IDs cannot be used with init"
        .into(),
    );
  }
  Ok(target)
}

pub fn parse_create(value: &str) -> Result<EnvironmentTarget, String> {
  let target = parse_target(value)?;
  if !target.project.starts_with(PROJECT_ID_PREFIX)
    && !target.project.contains('/')
    && !slug::is_valid(&target.project)
  {
    return Err(
      "project reference must be a project ID, WORKSPACE/PROJECT, or a lowercase slug of at most 63 characters"
        .into(),
    );
  }
  Ok(target)
}

pub fn parse_name(value: &str) -> Result<String, String> {
  if !slug::is_valid(value) {
    return Err("environment name must be a lowercase slug of at most 63 characters".into());
  }
  Ok(value.into())
}

fn parse_target(value: &str) -> Result<EnvironmentTarget, String> {
  let parsed = parse(value).ok_or_else(|| {
    format!(
      "environment target must use PROJECT/ENVIRONMENT, WORKSPACE/PROJECT/ENVIRONMENT, or /PROJECT/ENVIRONMENT for the default workspace, for example {TARGET_EXAMPLE}, {QUALIFIED_EXAMPLE}, or {DEFAULT_WORKSPACE_EXAMPLE}"
    )
  })?;
  let target = match parsed {
    crate::utils::environment_reference::EnvironmentReference::Id(_) => {
      return Err(
        "environment targets cannot be environment IDs; use PROJECT/ENVIRONMENT instead".into(),
      );
    }
    crate::utils::environment_reference::EnvironmentReference::Legacy {
      project,
      environment,
    } => EnvironmentTarget {
      project,
      environment,
    },
    crate::utils::environment_reference::EnvironmentReference::Qualified {
      workspace,
      project_path,
      environment,
    } => EnvironmentTarget {
      project: format!("{}/{}", workspace, project_path),
      environment,
    },
  };
  if !slug::is_valid(&target.environment) {
    return Err("environment name must be a lowercase slug of at most 63 characters".into());
  }
  Ok(target)
}
