use clap::Subcommand;

pub(crate) const HELP: &str = "\
Examples:
  dopbase project create payment-service
  dopbase project list
  dopbase project show payment-service
  dopbase project rename payment-service payments
  dopbase project delete payment-service
";
const CREATE_HELP: &str = "Examples:\n  dopbase project create payment-service\n";
const LIST_HELP: &str = "Examples:\n  dopbase project list\n";
const SHOW_HELP: &str = "Examples:\n  dopbase project show payment-service\n";
const RENAME_HELP: &str = "Examples:\n  dopbase project rename payment-service payments\n";
const DELETE_HELP: &str = "\
Examples:
  dopbase project delete payment-service
  dopbase project delete payment-service --yes
";

#[derive(Subcommand, Debug)]
pub enum ProjectCommand {
  /// Create an empty project.
  #[command(after_help = CREATE_HELP)]
  Create {
    /// Project name, unique on the server.
    #[arg(value_name = "PROJECT_NAME")]
    name: String,
  },
  /// List accessible projects.
  #[command(after_help = LIST_HELP)]
  List,
  /// Show project metadata.
  #[command(after_help = SHOW_HELP)]
  Show {
    /// Project ID or name.
    #[arg(value_name = "PROJECT_REF")]
    project: String,
  },
  /// Rename a project.
  #[command(after_help = RENAME_HELP)]
  Rename {
    /// Project ID or name.
    #[arg(value_name = "PROJECT_REF")]
    project: String,
    /// New project name.
    #[arg(value_name = "NEW_PROJECT_NAME")]
    new_name: String,
  },
  /// Delete a project with all its environments, secrets, and tokens.
  ///
  /// Asks for confirmation unless --yes is passed.
  #[command(after_help = DELETE_HELP)]
  Delete {
    /// Project ID or name.
    #[arg(value_name = "PROJECT_REF")]
    project: String,
    /// Skip the confirmation prompt (for automation).
    #[arg(long)]
    yes: bool,
  },
}
