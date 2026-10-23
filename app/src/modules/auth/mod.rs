use crate::state::AppState;
use axum::{
  Router,
  routing::{get, post},
};
pub mod controller;
pub mod doc;
pub mod error;
pub mod model;
pub mod oidc;
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
    .route(
      crate::constants::api::auth::OIDC_CONFIG,
      get(controller::oidc_config),
    )
    .route(
      crate::constants::api::auth::OIDC_START,
      get(controller::oidc_start),
    )
    .route(
      crate::constants::api::auth::OIDC_CALLBACK,
      get(controller::oidc_callback),
    )
}
