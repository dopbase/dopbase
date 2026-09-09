use crate::state::AppState;
use axum::{Router, routing::get};
pub mod controller;
pub mod doc;
pub mod model;
pub mod service;
pub fn routes() -> Router<AppState> {
  Router::new()
    .route(
      crate::constants::api::users::COLLECTION,
      get(controller::list).post(controller::create),
    )
    .route(
      crate::constants::api::users::ITEM,
      get(controller::get)
        .patch(controller::update)
        .delete(controller::delete),
    )
}
