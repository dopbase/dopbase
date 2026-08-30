use crate::state::AppState;
use axum::{
  Router,
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
    .route("/api/v1/environments", get(controller::list))
    .route("/api/v1/environments/resolve", get(controller::resolve))
    .route(
      "/api/v1/projects/{project_ref}/environments",
      post(controller::create),
    )
    .route(
      "/api/v1/environments/{environment_id}",
      get(controller::show)
        .patch(controller::rename)
        .delete(controller::delete),
    )
}
