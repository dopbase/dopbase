use axum::{
  Router,
  extract::DefaultBodyLimit,
  routing::{get, post},
};

use crate::state::AppState;

pub mod controller;
pub mod doc;
pub mod model;
pub mod service;

pub fn routes() -> Router<AppState> {
  Router::new()
    .route(
      crate::constants::api::backups::COLLECTION,
      get(controller::list).post(controller::create),
    )
    .route(
      crate::constants::api::backups::MASTER_KEY,
      get(controller::download_master_key),
    )
    .route(
      crate::constants::api::backups::UPLOAD,
      post(controller::upload),
    )
    .route(
      crate::constants::api::backups::ITEM,
      get(controller::download).delete(controller::delete),
    )
    .route(
      crate::constants::api::backups::RESTORE,
      post(controller::restore),
    )
    .layer(DefaultBodyLimit::max(250 * 1024 * 1024))
}
