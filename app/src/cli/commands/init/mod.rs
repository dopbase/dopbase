mod args;
mod handler;

pub(crate) use args::HELP;
pub use args::InitArgs;
pub(super) use handler::execute;
#[doc(hidden)]
pub use handler::{
  EnvCleanup, classify_entries, remove_env_if_unchanged, render_detection_summary,
  validate_interactive_target,
};
