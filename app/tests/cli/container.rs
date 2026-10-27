use app::cli::args::Cli;
use app::cli::container::execute_in_container;
use clap::Parser;

#[test]
fn container_rejects_server_lifecycle_commands() {
  for arguments in [
    ["dopbase", "--container", "server", "up"].as_slice(),
    ["dopbase", "--container", "server", "down"].as_slice(),
    ["dopbase", "--container", "server", "stop"].as_slice(),
    ["dopbase", "--container", "server", "restart"].as_slice(),
    ["dopbase", "--container", "server", "start", "--background"].as_slice(),
    ["dopbase", "--container", "server", "start"].as_slice(),
    ["dopbase", "--container", "server", "setup"].as_slice(),
  ] {
    let cli = Cli::try_parse_from(arguments).unwrap();
    let error = execute_in_container(cli).unwrap_err();
    assert!(
      error
        .to_string()
        .contains("Docker controls the Dopbase server process"),
      "{arguments:?}: {error}"
    );
  }
}
