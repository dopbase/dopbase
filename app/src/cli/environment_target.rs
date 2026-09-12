use crate::{constants::tokens::PROJECT_ID_PREFIX, utils::slug};

const TARGET_EXAMPLE: &str = "storefront/development";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvironmentTarget {
  project: String,
  environment: String,
}

impl EnvironmentTarget {
  pub fn into_parts(self) -> (String, String) {
    (self.project, self.environment)
  }
}

pub fn parse_init(value: &str) -> Result<EnvironmentTarget, String> {
  let target = parse(value)?;
  if !slug::is_valid(&target.project) {
    return Err(
      "project name must be a lowercase slug of at most 63 characters; project IDs cannot be used with init"
        .into(),
    );
  }
  Ok(target)
}

pub fn parse_create(value: &str) -> Result<EnvironmentTarget, String> {
  let target = parse(value)?;
  if !target.project.starts_with(PROJECT_ID_PREFIX) && !slug::is_valid(&target.project) {
    return Err(
      "project reference must be a project ID or a lowercase slug of at most 63 characters".into(),
    );
  }
  Ok(target)
}

fn parse(value: &str) -> Result<EnvironmentTarget, String> {
  let mut parts = value.split('/');
  let project = parts.next().unwrap_or_default();
  let environment = parts.next().unwrap_or_default();
  if project.is_empty() || environment.is_empty() || parts.next().is_some() {
    return Err(format!(
      "environment target must use PROJECT/ENVIRONMENT, for example {TARGET_EXAMPLE}"
    ));
  }
  if !slug::is_valid(environment) {
    return Err("environment name must be a lowercase slug of at most 63 characters".into());
  }
  Ok(EnvironmentTarget {
    project: project.into(),
    environment: environment.into(),
  })
}
