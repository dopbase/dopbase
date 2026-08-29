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
        .route(
            "/api/v1/environments/{environment_id}/tokens",
            get(controller::list).post(controller::create),
        )
        .route("/api/v1/tokens/{token_id}/revoke", post(controller::revoke))
}
