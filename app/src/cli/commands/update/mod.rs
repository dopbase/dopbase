mod args;
mod handler;

pub(crate) use args::HELP;
pub(super) use handler::run;
pub use handler::{
  ReleaseInfo, UpdateStatus, check_release, is_newer, parse_release, parse_version, update_message,
};
