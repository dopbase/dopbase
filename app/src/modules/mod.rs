use axum::Router;
use utoipa::openapi::OpenApi;

use crate::state::AppState;

pub mod audit;
pub mod auth;
pub mod backups;
pub mod bootstrap;
pub(crate) mod common;
pub mod environments;
pub mod health;
pub mod instance;
pub mod projects;
pub mod secrets;
pub mod service_accounts;
pub mod tokens;
pub mod users;

pub fn routes() -> Router<AppState> {
  Router::new()
    .merge(health::routes())
    .merge(bootstrap::routes())
    .merge(auth::routes())
    .merge(projects::routes())
    .merge(environments::routes())
    .merge(secrets::routes())
    .merge(service_accounts::routes())
    .merge(tokens::routes())
    .merge(audit::routes())
    .merge(instance::routes())
    .merge(backups::routes())
    .merge(users::routes())
}

pub fn openapi() -> OpenApi {
  let mut doc = health::doc::build();
  doc.merge(bootstrap::doc::build());
  doc.merge(auth::doc::build());
  doc.merge(projects::doc::build());
  doc.merge(environments::doc::build());
  doc.merge(secrets::doc::build());
  doc.merge(service_accounts::doc::build());
  doc.merge(tokens::doc::build());
  doc.merge(audit::doc::build());
  doc.merge(instance::doc::build());
  doc.merge(backups::doc::build());
  doc.merge(users::doc::build());
  doc
}
