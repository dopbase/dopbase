//! Roles: what their holders may do on the instance and how deep into
//! project data their grants can reach. Root creates custom roles.
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
      crate::constants::api::roles::COLLECTION,
      get(controller::list).post(controller::create),
    )
    .route(
      crate::constants::api::roles::ITEM,
      axum::routing::patch(controller::update).delete(controller::delete),
    )
}
