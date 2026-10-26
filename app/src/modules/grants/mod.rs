//! Grants: which users and groups reach which projects and environments.
use crate::state::AppState;
use axum::{
  Router,
  routing::{delete, get, put},
};
pub mod controller;
pub mod doc;
pub mod model;
pub(crate) mod repository;
pub mod service;
pub fn routes() -> Router<AppState> {
  Router::new()
    .route(
      crate::constants::api::grants::COLLECTION,
      put(controller::set),
    )
    .route(
      crate::constants::api::grants::ITEM,
      delete(controller::delete),
    )
    .route(
      crate::constants::api::projects::ACCESS,
      get(controller::project_access),
    )
    .route(
      crate::constants::api::users::ACCESS,
      get(controller::user_access),
    )
}
