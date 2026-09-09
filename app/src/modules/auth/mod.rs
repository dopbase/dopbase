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
    .route(crate::constants::api::auth::LOGIN, post(controller::login))
    .route(
      crate::constants::api::auth::LOGOUT,
      post(controller::logout),
    )
    .route(
      crate::constants::api::auth::SESSION,
      get(controller::session),
    )
    .route(
      crate::constants::api::auth::REAUTHENTICATE,
      post(controller::reauthenticate),
    )
    .route(
      crate::constants::api::auth::CHANGE_PASSWORD,
      post(controller::change_password),
    )
}
