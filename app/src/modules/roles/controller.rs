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

/// List roles
///
/// Return every role, built-in and custom, with its capabilities, level
/// ceiling, token policy, member count, and whether the caller may give it
/// to someone. Needs users.view or roles.manage.
#[utoipa::path(
  get,
  path = crate::constants::api::roles::COLLECTION,
  tag = "roles",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  responses(
    (status = 200, description = "Roles fetched", body = inline(HttpResponseFormat<Vec<RoleResponse>>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "users.view or roles.manage is required", body = crate::http::ErrorBody),
  ),
)]
pub async fn list(
  State(state): State<AppState>,
  identity: AuthIdentity,
) -> Result<HttpResponse<Vec<RoleResponse>>, crate::http::HttpError> {
  Ok(HttpResponse::ok(
    service::list(&state, &identity).await?,
    "ROLES_FETCHED",
  ))
}

/// Create a role
///
/// Create a custom role. Root only, from the Admin UI with recent
/// authentication and the CSRF header.
#[utoipa::path(
  post,
  path = crate::constants::api::roles::COLLECTION,
  tag = "roles",
  security(("cookieAuth" = [])),
  request_body = CreateRoleRequest,
  responses(
    (status = 201, description = "Role created", body = inline(HttpResponseFormat<RoleResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Root, CSRF, and recent authentication are required", body = crate::http::ErrorBody),
    (status = 409, description = "A role with this name already exists", body = crate::http::ErrorBody),
    (status = 422, description = "The role's settings are invalid", body = crate::http::ErrorBody),
  ),
)]
pub async fn create(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  axum::Json(request): axum::Json<CreateRoleRequest>,
) -> Result<HttpResponse<RoleResponse>, crate::http::HttpError> {
  Ok(HttpResponse::created(
    service::create(&state, &identity, &headers, request).await?,
    "ROLE_CREATED",
  ))
}

/// Update a role
///
/// Change a custom role. Built-in roles can't change, and a role's sign-in
/// never does. Its holders get the new settings on their next request.
/// Root only.
#[utoipa::path(
  patch,
  path = crate::constants::api::roles::ITEM,
  tag = "roles",
  security(("cookieAuth" = [])),
  params(("id" = String, Path, description = "Role ID with rol_ prefix")),
  request_body = UpdateRoleRequest,
  responses(
    (status = 200, description = "Role updated", body = inline(HttpResponseFormat<RoleResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Root is required, or the role is built in", body = crate::http::ErrorBody),
    (status = 404, description = "The role was not found", body = crate::http::ErrorBody),
    (status = 409, description = "A role with this name already exists", body = crate::http::ErrorBody),
    (status = 422, description = "The role's settings are invalid", body = crate::http::ErrorBody),
  ),
)]
pub async fn update(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path(id): Path<String>,
  axum::Json(request): axum::Json<UpdateRoleRequest>,
) -> Result<HttpResponse<RoleResponse>, crate::http::HttpError> {
  Ok(HttpResponse::ok(
    service::update(&state, &identity, &headers, &id, request).await?,
    "ROLE_UPDATED",
  ))
}

/// Delete a role
///
/// Delete a custom role nobody holds. Root only.
#[utoipa::path(
  delete,
  path = crate::constants::api::roles::ITEM,
  tag = "roles",
  security(("cookieAuth" = [])),
  params(("id" = String, Path, description = "Role ID with rol_ prefix")),
  responses(
    (status = 200, description = "Role deleted", body = inline(HttpResponseFormat<serde_json::Value>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Root is required, or the role is built in", body = crate::http::ErrorBody),
    (status = 404, description = "The role was not found", body = crate::http::ErrorBody),
    (status = 409, description = "Accounts still hold the role", body = crate::http::ErrorBody),
  ),
)]
pub async fn delete(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path(id): Path<String>,
) -> Result<HttpResponse<serde_json::Value>, crate::http::HttpError> {
  service::delete(&state, &identity, &headers, &id).await?;
  Ok(HttpResponse::done("ROLE_DELETED"))
}
