mod handler;

pub use handler::{
  complete_factory_reset, factory_reset_archive_path, factory_reset_confirmation_matches,
  factory_reset_quarantine_path, validate_factory_reset_target,
};
pub(super) use handler::execute;
