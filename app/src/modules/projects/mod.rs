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
      crate::constants::api::projects::COLLECTION,
      get(controller::list).post(controller::create),
    )
    .route(
      crate::constants::api::projects::INIT,
      post(controller::init),
    )
    .route(
      crate::constants::api::projects::ITEM,
      get(controller::show)
        .patch(controller::rename)
        .delete(controller::delete),
    )
}
