use crate::state::AppState;
use axum::{Router, routing::get};
pub mod controller;
pub mod doc;
pub mod model;
pub mod service;

pub fn routes() -> Router<AppState> {
  Router::new()
    .route(
      crate::constants::api::service_accounts::COLLECTION,
      get(controller::list).post(controller::create),
    )
    .route(
      crate::constants::api::service_accounts::ITEM,
      get(controller::get).delete(controller::delete),
    )
    .route(
      crate::constants::api::service_accounts::TOKENS,
      get(controller::tokens).post(controller::create_token),
    )
    .route(
      crate::constants::api::service_accounts::REVOKE_TOKEN,
      axum::routing::post(controller::revoke_token),
    )
}
