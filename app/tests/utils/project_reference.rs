use app::utils::project_reference::{parse_create_target, to_api_reference};

#[test]
fn to_api_reference_expands_default_workspace_prefix() {
  assert_eq!(to_api_reference("/api").unwrap(), "default/api");
  assert_eq!(to_api_reference("blipsu/api").unwrap(), "blipsu/api");
  assert_eq!(
    to_api_reference("payment-service").unwrap(),
    "payment-service"
  );
}

#[test]
fn parse_create_target_assigns_workspace_location() {
  let target = parse_create_target("/api").unwrap();
  assert_eq!(target.name, "api");
  assert_eq!(target.workspace.as_deref(), Some("default"));
  assert_eq!(target.relative_path.as_deref(), Some("api"));
}
