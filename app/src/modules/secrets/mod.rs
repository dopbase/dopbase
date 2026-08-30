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
    .route(
      "/api/v1/environments/{environment_id}/secrets",
      get(controller::list),
    )
    .route(
      "/api/v1/environments/{environment_id}/secrets/import",
      post(controller::import),
    )
    .route(
      "/api/v1/environments/{environment_id}/secrets/layout",
      get(controller::layout),
    )
    .route(
      "/api/v1/environments/{environment_id}/secrets/export",
      post(controller::export),
    )
    .route(
      "/api/v1/environments/{environment_id}/secrets/runtime",
      get(controller::runtime),
    )
    .route(
      "/api/v1/environments/{environment_id}/secrets/{key}",
      get(controller::get)
        .put(controller::set)
        .delete(controller::delete),
    )
    .route(
      "/api/v1/environments/{environment_id}/secrets/{key}/reveal",
      post(controller::reveal),
    )
}
