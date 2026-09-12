mod handler;

pub use handler::{ReleaseInfo, UpdateStatus, is_newer, parse_release, parse_version, update_message};
pub(super) use handler::run;
