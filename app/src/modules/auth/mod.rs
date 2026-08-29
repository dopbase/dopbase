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
        .route("/api/v1/auth/login", post(controller::login))
        .route("/api/v1/auth/logout", post(controller::logout))
        .route("/api/v1/auth/session", get(controller::session))
        .route(
            "/api/v1/auth/reauthenticate",
            post(controller::reauthenticate),
        )
        .route(
            "/api/v1/auth/change-password",
            post(controller::change_password),
        )
}
