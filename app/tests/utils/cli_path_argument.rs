use app::utils::{cli_path_argument, project_reference::to_api_reference};

#[test]
fn normalize_git_bash_converted_default_project_reference() {
  assert_eq!(
    cli_path_argument::normalize("C:/Program Files/Git/api"),
    "/api"
  );
  assert_eq!(
    to_api_reference("C:/Program Files/Git/api").unwrap(),
    "default/api"
  );
}

#[test]
fn normalize_double_slash_default_workspace_prefix() {
  assert_eq!(cli_path_argument::normalize("//api"), "/api");
  assert_eq!(to_api_reference("//api").unwrap(), "default/api");
}
