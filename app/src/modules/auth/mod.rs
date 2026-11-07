use crate::state::AppState;
use axum::{
  Router,
  routing::{get, post},
};
pub mod controller;
mod device;
pub mod doc;
pub mod error;
mod launch;
pub mod model;
mod repository;
pub mod service;
pub fn routes() -> Router<AppState> {
  Router::new()
    .route(crate::constants::api::auth::LOGIN, post(controller::login))
    .route(
      crate::constants::api::auth::LAUNCH,
      post(controller::launch),
    )
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
      crate::constants::api::auth::METHODS,
      get(controller::methods),
    )
    .route(
      crate::constants::api::auth::CLI_DEVICE,
      post(controller::cli_device),
    )
    .route(
      crate::constants::api::auth::CLI_TOKEN,
      post(controller::cli_token),
    )
    .route(
      crate::constants::api::auth::CLI_CANCEL,
      post(controller::cli_cancel),
    )
    .route(
      crate::constants::api::auth::CLI_INSPECT,
      post(controller::cli_inspect),
    )
    .route(
      crate::constants::api::auth::CLI_APPROVE,
      post(controller::cli_approve),
    )
    .route(
      crate::constants::api::auth::CLI_DENY,
      post(controller::cli_deny),
    )
}
