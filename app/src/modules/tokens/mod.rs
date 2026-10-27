use crate::state::AppState;
use axum::{
  Router,
  routing::{get, post},
};
pub mod access;
pub mod controller;
pub mod doc;
pub mod error;
pub mod model;
mod repository;
pub mod service;
pub fn routes() -> Router<AppState> {
  Router::new()
    .route(
      crate::constants::api::tokens::COLLECTION,
      get(controller::list).post(controller::create),
    )
    .route(
      crate::constants::api::tokens::WORKSPACE_COLLECTION,
      get(controller::list_workspace).post(controller::create_workspace),
    )
    .route(
      crate::constants::api::tokens::REVOKE,
      post(controller::revoke),
    )
}
