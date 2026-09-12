mod handler;

pub(super) use handler::{connect, ensure_server_is_connected, show_status};
pub use handler::{insecure_transport_warning, server_switch_confirmed, status_document};
