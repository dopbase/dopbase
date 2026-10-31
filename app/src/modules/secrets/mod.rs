use crate::state::AppState;
use axum::{
  Router,
  routing::{get, post},
};
pub mod controller;
pub mod doc;
pub mod error;
pub mod model;
pub(crate) mod repository;
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
    .route(
      crate::constants::api::secrets::VERSIONS,
      get(controller::versions),
    )
    .route(
      crate::constants::api::secrets::VERSION_REVEAL,
      post(controller::reveal_version),
    )
    .route(
      crate::constants::api::secrets::ROLLBACK,
      post(controller::rollback),
    )
    .route(crate::constants::api::secrets::COPY, post(controller::copy))
    .route(
      crate::constants::api::secrets::COMPARE,
      get(controller::compare),
    )
}
