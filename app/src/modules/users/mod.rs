use crate::state::AppState;
use axum::{Router, routing::get};
pub mod controller;
pub mod doc;
pub mod model;
pub mod service;
pub fn routes() -> Router<AppState> {
  Router::new()
    .route(
      "/api/v1/users",
      get(controller::list).post(controller::create),
    )
    .route(
      "/api/v1/users/{id}",
      get(controller::get)
        .patch(controller::update)
        .delete(controller::delete),
    )
}
