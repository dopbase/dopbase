//! Groups: named sets of users that receive grants together.
use crate::state::AppState;
use axum::{Router, routing::get};
pub mod controller;
pub mod doc;
pub mod model;
mod repository;
pub mod service;
pub fn routes() -> Router<AppState> {
  Router::new()
    .route(
      crate::constants::api::groups::COLLECTION,
      get(controller::list).post(controller::create),
    )
    .route(
      crate::constants::api::groups::ITEM,
      get(controller::get)
        .patch(controller::update)
        .delete(controller::delete),
    )
    .route(
      crate::constants::api::groups::MEMBER,
      axum::routing::put(controller::add_member).delete(controller::remove_member),
    )
}
