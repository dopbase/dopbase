use app::utils::environment_reference::{EnvironmentReference, parse};

#[test]
fn parse_legacy_environment_reference() {
  let parsed = parse("payment-service/production").unwrap();
  assert_eq!(
    parsed,
    EnvironmentReference::Legacy {
      project: "payment-service".into(),
      environment: "production".into(),
    }
  );
}

#[test]
fn parse_workspace_qualified_environment_reference() {
  let parsed = parse("workspace1/api/staging").unwrap();
  assert_eq!(
    parsed,
    EnvironmentReference::Qualified {
      workspace: "workspace1".into(),
      project_path: "api".into(),
      environment: "staging".into(),
    }
  );
}

#[test]
fn parse_default_workspace_shorthand_environment_reference() {
  let parsed = parse("/api/production").unwrap();
  assert_eq!(
    parsed,
    EnvironmentReference::Qualified {
      workspace: "default".into(),
      project_path: "api".into(),
      environment: "production".into(),
    }
  );
}

#[test]
fn parse_nested_project_path_environment_reference() {
  let parsed = parse("workspace1/services/api/staging").unwrap();
  assert_eq!(
    parsed,
    EnvironmentReference::Qualified {
      workspace: "workspace1".into(),
      project_path: "services/api".into(),
      environment: "staging".into(),
    }
  );
}
