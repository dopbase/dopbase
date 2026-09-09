use crate::state::AppState;
use axum::{
  Router,
  extract::DefaultBodyLimit,
  routing::{get, post},
};
pub mod controller;
pub mod doc;
pub mod error;
pub mod model;
mod repository;
pub mod service;
pub fn routes() -> Router<AppState> {
  Router::new()
    .route(
      crate::constants::api::bootstrap::STATUS,
      get(controller::status),
    )
    .route(
      crate::constants::api::bootstrap::ADMIN,
      post(controller::create_admin),
    )
    .route(
      crate::constants::api::bootstrap::RESTORE,
      post(controller::restore),
    )
    .layer(DefaultBodyLimit::max(250 * 1024 * 1024))
}
