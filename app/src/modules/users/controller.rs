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

/// List users
///
/// Return every administrator and member account ordered by role and email.
/// Requires an authenticated administrator browser session.
#[utoipa::path(
  get,
  path = crate::constants::api::users::COLLECTION,
  tag = "users",
  security(("cookieAuth" = [])),
  responses(
    (status = 200, description = "Users fetched", body = inline(HttpResponseFormat<Vec<UserResponse>>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Administrator browser session is required", body = crate::http::ErrorBody),
  ),
)]
pub async fn list(
  State(state): State<AppState>,
  identity: AuthIdentity,
) -> Result<HttpResponse<Vec<UserResponse>>, crate::http::HttpError> {
  Ok(HttpResponse::ok(
    service::list(&state, &identity).await?,
    "USERS_FETCHED",
  ))
}

/// Show a user
///
/// Fetch a single user account by its identifier. Requires an authenticated
/// administrator browser session.
#[utoipa::path(
  get,
  path = crate::constants::api::users::ITEM,
  tag = "users",
  security(("cookieAuth" = [])),
  params(("id" = String, Path, description = "User ID with usr_ prefix")),
  responses(
    (status = 200, description = "User fetched", body = inline(HttpResponseFormat<UserResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Administrator browser session is required", body = crate::http::ErrorBody),
    (status = 404, description = "The user was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn get(
  State(state): State<AppState>,
  identity: AuthIdentity,
  Path(id): Path<String>,
) -> Result<HttpResponse<UserResponse>, crate::http::HttpError> {
  Ok(HttpResponse::ok(
    service::get(&state, &identity, &id).await?,
    "USER_FETCHED",
  ))
}

/// Create a user
///
/// Provision a new administrator or member user account. Requires an
/// authenticated administrator browser session with recent re-authentication
/// and the CSRF header.
#[utoipa::path(
  post,
  path = crate::constants::api::users::COLLECTION,
  tag = "users",
  security(("cookieAuth" = [])),
  request_body = CreateUserRequest,
  responses(
    (status = 201, description = "User created", body = inline(HttpResponseFormat<UserResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Administrator browser session with recent authentication and CSRF token is required, or root role requested", body = crate::http::ErrorBody),
    (status = 409, description = "An account with this email already exists", body = crate::http::ErrorBody),
    (status = 422, description = "Validation failed for email or password", body = crate::http::ErrorBody),
  ),
)]
pub async fn create(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  axum::Json(request): axum::Json<CreateUserRequest>,
) -> Result<HttpResponse<UserResponse>, crate::http::HttpError> {
  Ok(HttpResponse::created(
    service::create(&state, &identity, &headers, request).await?,
    "USER_CREATED",
  ))
}

/// Update a user
///
/// Modify email, password, or role for an existing user account. Updating
/// credentials or roles revokes all active sessions for that account.
/// Requires an authenticated administrator browser session with recent
/// re-authentication and the CSRF header.
#[utoipa::path(
  patch,
  path = crate::constants::api::users::ITEM,
  tag = "users",
  security(("cookieAuth" = [])),
  params(("id" = String, Path, description = "User ID with usr_ prefix")),
  request_body = UpdateUserRequest,
  responses(
    (status = 200, description = "User updated", body = inline(HttpResponseFormat<UserResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Administrator browser session with recent authentication and CSRF token is required, root account modification blocked, or self role change attempted", body = crate::http::ErrorBody),
    (status = 404, description = "The user was not found", body = crate::http::ErrorBody),
    (status = 409, description = "An account with this email already exists", body = crate::http::ErrorBody),
    (status = 422, description = "Validation failed for email or password", body = crate::http::ErrorBody),
  ),
)]
pub async fn update(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path(id): Path<String>,
  axum::Json(request): axum::Json<UpdateUserRequest>,
) -> Result<HttpResponse<UserResponse>, crate::http::HttpError> {
  Ok(HttpResponse::ok(
    service::update(&state, &identity, &headers, &id, request).await?,
    "USER_UPDATED",
  ))
}

/// Delete a user
///
/// Delete an administrator or member account. The root administrator account
/// and the currently authenticated account cannot be deleted. Requires an
/// authenticated administrator browser session with recent re-authentication
/// and the CSRF header.
#[utoipa::path(
  delete,
  path = crate::constants::api::users::ITEM,
  tag = "users",
  security(("cookieAuth" = [])),
  params(("id" = String, Path, description = "User ID with usr_ prefix")),
  responses(
    (status = 200, description = "User deleted", body = inline(HttpResponseFormat<serde_json::Value>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Administrator browser session with recent authentication and CSRF token is required, or attempted to delete root or self", body = crate::http::ErrorBody),
    (status = 404, description = "The user was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn delete(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path(id): Path<String>,
) -> Result<HttpResponse<serde_json::Value>, crate::http::HttpError> {
  service::delete(&state, &identity, &headers, &id).await?;
  Ok(HttpResponse::done("USER_DELETED"))
}

/// Unlink a user's single sign-on
///
/// Remove the user's linked identity. A verified email links it again on
/// their next single sign-on. Requires an administrator browser session with
/// recent authentication and the CSRF header.
#[utoipa::path(
  delete,
  path = crate::constants::api::users::SSO,
  tag = "users",
  security(("cookieAuth" = [])),
  params(("id" = String, Path, description = "User ID with usr_ prefix")),
  responses(
    (status = 200, description = "Link removed", body = inline(HttpResponseFormat<UserResponse>)),
    (status = 403, description = "Administrator browser session, CSRF, or recent authentication required", body = crate::http::ErrorBody),
    (status = 404, description = "The user or the link was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn unlink_sso(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path(id): Path<String>,
) -> Result<HttpResponse<UserResponse>, crate::http::HttpError> {
  Ok(HttpResponse::ok(
    service::unlink_sso(&state, &identity, &headers, &id).await?,
    "USER_SSO_UNLINKED",
  ))
}

/// Disable a user
///
/// The account can't sign in, its sessions end, and its personal tokens and
/// those of AI agents it owns are refused until it is enabled again.
/// Disabling a disabled account changes nothing. Root and your own account
/// can't be disabled. Requires users.manage from the Admin UI with recent
/// authentication and the CSRF header.
#[utoipa::path(
  post,
  path = crate::constants::api::users::DISABLE,
  tag = "users",
  security(("cookieAuth" = [])),
  params(("id" = String, Path, description = "User ID with usr_ prefix")),
  responses(
    (status = 200, description = "User disabled", body = inline(HttpResponseFormat<UserResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "users.manage, CSRF, or recent authentication is required, the account is root or your own, or it has access you don't", body = crate::http::ErrorBody),
    (status = 404, description = "The user was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn disable(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path(id): Path<String>,
) -> Result<HttpResponse<UserResponse>, crate::http::HttpError> {
  Ok(HttpResponse::ok(
    service::set_status(&state, &identity, &headers, &id, UserStatus::Disabled).await?,
    "USER_DISABLED",
  ))
}

/// Enable a user
///
/// Let a disabled account sign in again. Its tokens that are still valid
/// work again. Enabling an active account changes nothing. Same rights as
/// disabling it.
#[utoipa::path(
  post,
  path = crate::constants::api::users::ENABLE,
  tag = "users",
  security(("cookieAuth" = [])),
  params(("id" = String, Path, description = "User ID with usr_ prefix")),
  responses(
    (status = 200, description = "User enabled", body = inline(HttpResponseFormat<UserResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "users.manage, CSRF, or recent authentication is required, the account is root or your own, or it has access you don't", body = crate::http::ErrorBody),
    (status = 404, description = "The user was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn enable(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path(id): Path<String>,
) -> Result<HttpResponse<UserResponse>, crate::http::HttpError> {
  Ok(HttpResponse::ok(
    service::set_status(&state, &identity, &headers, &id, UserStatus::Active).await?,
    "USER_ENABLED",
  ))
}
