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
      crate::constants::api::environments::COLLECTION,
      get(controller::list),
    )
    .route(
      crate::constants::api::environments::RESOLVE,
      get(controller::resolve),
    )
    .route(
      crate::constants::api::projects::ENVIRONMENTS,
      post(controller::create),
    )
    .route(
      crate::constants::api::environments::ITEM,
      get(controller::show)
        .patch(controller::rename)
        .delete(controller::delete),
    )
}
