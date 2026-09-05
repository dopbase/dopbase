use axum::{
  Router,
  extract::DefaultBodyLimit,
  routing::{get, post},
};

use crate::state::AppState;

pub mod controller;
pub mod doc;
pub mod model;
pub mod service;

pub fn routes() -> Router<AppState> {
  Router::new()
    .route(
      "/api/v1/backups",
      get(controller::list).post(controller::create),
    )
    .route(
      "/api/v1/backups/master-key",
      get(controller::download_master_key),
    )
    .route("/api/v1/backups/upload", post(controller::upload))
    .route(
      "/api/v1/backups/{key}",
      get(controller::download).delete(controller::delete),
    )
    .route("/api/v1/backups/{key}/restore", post(controller::restore))
    .layer(DefaultBodyLimit::max(250 * 1024 * 1024))
}
