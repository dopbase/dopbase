//! Personal tokens: credentials of people and AI agent accounts.
use crate::state::AppState;
use axum::{
  Router,
  routing::{get, post},
};
pub mod controller;
pub mod doc;
pub mod model;
mod repository;
pub mod service;
pub fn routes() -> Router<AppState> {
  Router::new()
    .route(
      crate::constants::api::users::TOKENS,
      get(controller::list).post(controller::create),
    )
    .route(
      crate::constants::api::users::TOKEN_REVOKE,
      post(controller::revoke),
    )
}
