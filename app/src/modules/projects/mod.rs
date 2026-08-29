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
            "/api/v1/projects",
            get(controller::list).post(controller::create),
        )
        .route("/api/v1/projects/init", post(controller::init))
        .route(
            "/api/v1/projects/{project_ref}",
            get(controller::show)
                .patch(controller::rename)
                .delete(controller::delete),
        )
}
