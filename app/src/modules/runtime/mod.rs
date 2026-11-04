//! The runner-only gate. A runner token reaches one environment, so the
//! token alone decides what these routes return.
use crate::state::AppState;
use axum::{Router, routing::get};
pub mod controller;
pub mod doc;
pub mod error;
pub mod model;
pub mod service;
pub fn routes() -> Router<AppState> {
  Router::new().route(
    crate::constants::api::runtime::SECRETS,
    get(controller::secrets),
  )
}
