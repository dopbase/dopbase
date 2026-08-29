use axum::Router;
use utoipa::openapi::OpenApi;

use crate::state::AppState;

pub mod audit;
pub mod auth;
pub mod bootstrap;
pub(crate) mod common;
pub mod environments;
pub mod health;
pub mod instance;
pub mod projects;
pub mod secrets;
pub mod tokens;

pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(health::routes())
        .merge(bootstrap::routes())
        .merge(auth::routes())
        .merge(projects::routes())
        .merge(environments::routes())
        .merge(secrets::routes())
        .merge(tokens::routes())
        .merge(audit::routes())
        .merge(instance::routes())
}

pub fn openapi() -> OpenApi {
    let mut doc = health::doc::build();
    doc.merge(bootstrap::doc::build());
    doc.merge(auth::doc::build());
    doc.merge(projects::doc::build());
    doc.merge(environments::doc::build());
    doc.merge(secrets::doc::build());
    doc.merge(tokens::doc::build());
    doc.merge(audit::doc::build());
    doc.merge(instance::doc::build());
    doc
}
