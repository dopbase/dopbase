use clap::Subcommand;

pub(crate) const HELP: &str = "\
Examples:
  dopbase workspace create blipsu --root-path D:/Programming/Allior
  dopbase workspace list
  dopbase workspace show blipsu
  dopbase workspace update blipsu --root-path D:/Programming/Allior
  dopbase workspace delete blipsu --yes
";
const CREATE_HELP: &str = "\
Examples:
  dopbase workspace create blipsu
  dopbase workspace create blipsu --root-path D:/Programming/Allior
";
const LIST_HELP: &str = "\
Examples:
  dopbase workspace list
";
const SHOW_HELP: &str = "\
Examples:
  dopbase workspace show default
  dopbase workspace show wsp_01JEXAMPLE
";
const UPDATE_HELP: &str = "\
Examples:
  dopbase workspace update blipsu --name blipsu --root-path D:/Programming/Allior
";
const DELETE_HELP: &str = "\
Examples:
  dopbase workspace delete blipsu --yes
";

#[derive(Subcommand, Debug)]
pub enum WorkspaceCommand {
  /// Create a workspace.
  #[command(after_help = CREATE_HELP)]
  Create {
    /// Workspace name (unique slug).
    #[arg(value_name = "WORKSPACE_NAME")]
    name: String,
    /// Absolute root directory on this machine (optional; server default applies when omitted).
    #[arg(long, value_name = "PATH")]
    root_path: Option<String>,
  },
  /// List workspaces.
  #[command(after_help = LIST_HELP)]
  List,
  /// Show workspace metadata.
  #[command(after_help = SHOW_HELP)]
  Show {
    /// Workspace name or ID.
    #[arg(value_name = "WORKSPACE_REF")]
    workspace: String,
  },
  /// Update a workspace name and root directory.
  #[command(after_help = UPDATE_HELP)]
  Update {
    /// Workspace name or ID.
    #[arg(value_name = "WORKSPACE_REF")]
    workspace: String,
    /// New workspace name.
    #[arg(long, value_name = "NAME")]
    name: String,
    /// Absolute root directory on this machine.
    #[arg(long, value_name = "PATH")]
    root_path: String,
  },
  /// Delete an empty workspace.
  ///
  /// Asks for confirmation unless --yes is passed.
  #[command(after_help = DELETE_HELP)]
  Delete {
    /// Workspace name or ID.
    #[arg(value_name = "WORKSPACE_REF")]
    workspace: String,
    /// Skip the confirmation prompt (for automation).
    #[arg(long)]
    yes: bool,
  },
}
