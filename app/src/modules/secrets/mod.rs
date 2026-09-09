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
      crate::constants::api::secrets::COLLECTION,
      get(controller::list),
    )
    .route(
      crate::constants::api::secrets::IMPORT,
      post(controller::import),
    )
    .route(
      crate::constants::api::secrets::LAYOUT,
      get(controller::layout),
    )
    .route(
      crate::constants::api::secrets::EXPORT,
      post(controller::export),
    )
    .route(
      crate::constants::api::secrets::RUNTIME,
      get(controller::runtime),
    )
    .route(
      crate::constants::api::secrets::ITEM,
      get(controller::get)
        .put(controller::set)
        .delete(controller::delete),
    )
    .route(
      crate::constants::api::secrets::REVEAL,
      post(controller::reveal),
    )
}
