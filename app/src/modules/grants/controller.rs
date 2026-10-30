use super::{model::*, service};
use crate::{
  http::{HttpResponse, HttpResponseFormat},
  models::AuthIdentity,
  state::AppState,
};
use axum::{
  extract::{Path, State},
  http::HeaderMap,
};

/// Give access
///
/// Give a user or group a level on all projects, one project, or one
/// environment. A grant on a scope that already has one replaces its level.
/// Granting on a project or its environments needs the project manager
/// level on it; all projects is for root. Requires an Admin UI session with
/// recent authentication and the CSRF header.
#[utoipa::path(
  put,
  path = crate::constants::api::grants::COLLECTION,
  tag = "grants",
  security(("cookieAuth" = [])),
  request_body = SetGrantRequest,
  responses(
    (status = 200, description = "Grant saved", body = inline(HttpResponseFormat<GrantResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Not a project manager of the scope, a grant to root or to yourself, or the session, CSRF, or recent authentication check failed", body = crate::http::ErrorBody),
    (status = 404, description = "The user, group, project, or environment was not found", body = crate::http::ErrorBody),
    (status = 422, description = "The scope or level is invalid, or the level is above the account's role", body = crate::http::ErrorBody),
  ),
)]
pub async fn set(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  axum::Json(request): axum::Json<SetGrantRequest>,
) -> Result<HttpResponse<GrantResponse>, crate::http::HttpError> {
  Ok(HttpResponse::ok(
    service::set(&state, &identity, &headers, request).await?,
    "GRANT_SAVED",
  ))
}

/// Remove access
///
/// Delete a grant. Needs the same rights as giving it. Requires an Admin UI
/// session with recent authentication and the CSRF header.
#[utoipa::path(
  delete,
  path = crate::constants::api::grants::ITEM,
  tag = "grants",
  security(("cookieAuth" = [])),
  params(("id" = String, Path, description = "Grant ID with grt_ prefix")),
  responses(
    (status = 200, description = "Grant removed", body = inline(HttpResponseFormat<serde_json::Value>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Not a project manager of the scope, your own grant, or the session, CSRF, or recent authentication check failed", body = crate::http::ErrorBody),
    (status = 404, description = "The grant was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn delete(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path(id): Path<String>,
) -> Result<HttpResponse<serde_json::Value>, crate::http::HttpError> {
  service::delete(&state, &identity, &headers, &id).await?;
  Ok(HttpResponse::done("GRANT_REMOVED"))
}

/// Project access
///
/// List the project's environments and every grant on the project and its
/// environments. Needs the project manager level on the project, or the
/// access.report capability.
#[utoipa::path(
  get,
  path = crate::constants::api::projects::ACCESS,
  tag = "grants",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("project_ref" = String, Path, description = "Project id or name")),
  responses(
    (status = 200, description = "Project access fetched", body = inline(HttpResponseFormat<ProjectAccessResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "The project manager level or access.report is required", body = crate::http::ErrorBody),
    (status = 404, description = "The project was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn project_access(
  State(state): State<AppState>,
  identity: AuthIdentity,
  Path(reference): Path<String>,
) -> Result<HttpResponse<ProjectAccessResponse>, crate::http::HttpError> {
  Ok(HttpResponse::ok(
    service::project_access(&state, &identity, &reference).await?,
    "PROJECT_ACCESS_FETCHED",
  ))
}

/// User access
///
/// List an account's groups and the grants that reach it, its own and its
/// groups'. Anyone can read their own; another account's needs users.view.
#[utoipa::path(
  get,
  path = crate::constants::api::users::ACCESS,
  tag = "grants",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(("id" = String, Path, description = "User ID with usr_ prefix")),
  responses(
    (status = 200, description = "User access fetched", body = inline(HttpResponseFormat<UserAccessResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "users.view is required for another account", body = crate::http::ErrorBody),
    (status = 404, description = "The user was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn user_access(
  State(state): State<AppState>,
  identity: AuthIdentity,
  Path(id): Path<String>,
) -> Result<HttpResponse<UserAccessResponse>, crate::http::HttpError> {
  Ok(HttpResponse::ok(
    service::user_access(&state, &identity, &id).await?,
    "USER_ACCESS_FETCHED",
  ))
}
