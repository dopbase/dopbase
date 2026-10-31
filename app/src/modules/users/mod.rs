use crate::state::AppState;
use axum::{
  Router,
  routing::{delete, get, post},
};
pub mod controller;
pub mod doc;
pub mod model;
mod repository;
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
    .route(
      crate::constants::api::users::SSO,
      delete(controller::unlink_sso),
    )
    .route(
      crate::constants::api::users::DISABLE,
      post(controller::disable),
    )
    .route(
      crate::constants::api::users::ENABLE,
      post(controller::enable),
    )
}
