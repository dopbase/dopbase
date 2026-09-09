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
    .route(
      crate::constants::api::instance::ROOT,
      get(controller::status),
    )
    .route(
      crate::constants::api::instance::PUBLIC_STATUS,
      get(controller::public_status),
    )
    .route(
      crate::constants::api::instance::FACTORY_RESET,
      get(controller::factory_reset_preview).post(controller::factory_reset),
    )
}
