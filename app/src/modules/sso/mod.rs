use crate::state::AppState;
use axum::{
  Router,
  routing::{get, post, put},
};
pub mod controller;
pub mod doc;
pub mod error;
pub mod model;
pub mod presets;
mod repository;
pub mod service;
/// Every route here answers `FEATURE_DISABLED` while `DOPBASE_SSO` is off.
pub fn routes(state: &AppState) -> Router<AppState> {
  Router::new()
    .route(
      crate::constants::api::sso::PROVIDER,
      get(controller::get_provider)
        .put(controller::put_provider)
        .delete(controller::delete_provider),
    )
    .route(
      crate::constants::api::sso::PROVIDER_ENABLED,
      put(controller::set_enabled),
    )
    .route(
      crate::constants::api::sso::PROVIDER_REQUIRED,
      put(controller::set_required),
    )
    .route(
      crate::constants::api::sso::PROVIDER_PASSWORD_FORM,
      put(controller::set_password_form),
    )
    .route(
      crate::constants::api::auth::SSO_START,
      get(controller::start),
    )
    .route(
      crate::constants::api::auth::SSO_CALLBACK,
      get(controller::callback),
    )
    .route(
      crate::constants::api::auth::SSO_EXCHANGE,
      post(controller::exchange),
    )
    .route(
      crate::constants::api::auth::SSO_LINK,
      post(controller::link),
    )
    .route(
      crate::constants::api::auth::SSO_IDENTITY,
      get(controller::identity).delete(controller::unlink),
    )
    .route(
      crate::constants::api::auth::SSO_REAUTH,
      post(controller::reauth),
    )
    .route_layer(axum::middleware::from_fn_with_state(
      state.clone(),
      crate::middlewares::features::sso,
    ))
}
