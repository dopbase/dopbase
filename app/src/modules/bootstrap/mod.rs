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
    .route("/api/v1/bootstrap/status", get(controller::status))
    .route("/api/v1/bootstrap/admin", post(controller::create_admin))
    .route("/api/v1/bootstrap/restore", post(controller::restore))
    .layer(DefaultBodyLimit::max(250 * 1024 * 1024))
}
