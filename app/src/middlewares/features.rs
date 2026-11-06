//! Route layers that refuse a whole feature while the server has it off.

use axum::{
  extract::{Request, State},
  middleware::Next,
  response::{IntoResponse, Response},
};

use crate::{services::features::Feature, state::AppState};

/// Refuse every single sign-on route while `DOPBASE_SSO` is off.
pub async fn sso(
  State(state): State<AppState>,
  request: Request,
  next: Next,
) -> Response {
  match state.config.features.require(Feature::Sso) {
    Ok(()) => next.run(request).await,
    Err(error) => error.into_response(),
  }
}
