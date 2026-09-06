use crate::state::AppState;
use axum::{Router, routing::get};
pub mod controller;
pub mod doc;
pub mod error;
pub mod model;
mod repository;
pub mod service;
pub fn routes() -> Router<AppState> {
  Router::new()
    .route("/api/v1/instance", get(controller::status))
    .route("/api/v1/status", get(controller::public_status))
    .route(
      "/api/v1/instance/factory-reset",
      get(controller::factory_reset_preview).post(controller::factory_reset),
    )
}
