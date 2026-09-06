use crate::state::AppState;
use axum::{Router, routing::get};
pub mod controller;
pub mod doc;
pub mod model;
pub mod service;

pub fn routes() -> Router<AppState> {
  Router::new()
    .route(
      "/api/v1/service-accounts",
      get(controller::list).post(controller::create),
    )
    .route(
      "/api/v1/service-accounts/{id}",
      get(controller::get).delete(controller::delete),
    )
    .route(
      "/api/v1/service-accounts/{id}/tokens",
      get(controller::tokens).post(controller::create_token),
    )
    .route(
      "/api/v1/service-accounts/{id}/tokens/{token_id}/revoke",
      axum::routing::post(controller::revoke_token),
    )
}
