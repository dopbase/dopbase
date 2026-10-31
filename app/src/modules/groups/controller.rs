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

/// List groups
///
/// Return every group with its members, grants, and creator. Needs
/// users.view or groups.manage from the Admin UI.
#[utoipa::path(
  get,
  path = crate::constants::api::groups::COLLECTION,
  tag = "groups",
  security(("cookieAuth" = [])),
  responses(
    (status = 200, description = "Groups fetched", body = inline(HttpResponseFormat<Vec<GroupListItem>>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "users.view or groups.manage from the Admin UI is required", body = crate::http::ErrorBody),
  ),
)]
pub async fn list(
  State(state): State<AppState>,
  identity: AuthIdentity,
) -> Result<HttpResponse<Vec<GroupListItem>>, crate::http::HttpError> {
  Ok(HttpResponse::ok(
    service::list(&state, &identity).await?,
    "GROUPS_FETCHED",
  ))
}

/// Show a group
///
/// Return a group with its members and grants.
#[utoipa::path(
  get,
  path = crate::constants::api::groups::ITEM,
  tag = "groups",
  security(("cookieAuth" = [])),
  params(("id" = String, Path, description = "Group ID with grp_ prefix")),
  responses(
    (status = 200, description = "Group fetched", body = inline(HttpResponseFormat<GroupDetailResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "users.view or groups.manage from the Admin UI is required", body = crate::http::ErrorBody),
    (status = 404, description = "The group was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn get(
  State(state): State<AppState>,
  identity: AuthIdentity,
  Path(id): Path<String>,
) -> Result<HttpResponse<GroupDetailResponse>, crate::http::HttpError> {
  Ok(HttpResponse::ok(
    service::get(&state, &identity, &id).await?,
    "GROUP_FETCHED",
  ))
}

/// Create a group
///
/// Needs groups.manage, an Admin UI session with recent authentication, and
/// the CSRF header.
#[utoipa::path(
  post,
  path = crate::constants::api::groups::COLLECTION,
  tag = "groups",
  security(("cookieAuth" = [])),
  request_body = CreateGroupRequest,
  responses(
    (status = 201, description = "Group created", body = inline(HttpResponseFormat<GroupDetailResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "groups.manage, CSRF, or recent authentication is required", body = crate::http::ErrorBody),
    (status = 409, description = "A group with this name already exists", body = crate::http::ErrorBody),
    (status = 422, description = "The name or description is invalid", body = crate::http::ErrorBody),
  ),
)]
pub async fn create(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  axum::Json(request): axum::Json<CreateGroupRequest>,
) -> Result<HttpResponse<GroupDetailResponse>, crate::http::HttpError> {
  Ok(HttpResponse::created(
    service::create(&state, &identity, &headers, request).await?,
    "GROUP_CREATED",
  ))
}

/// Update a group
///
/// Rename a group or change its description. Besides groups.manage, the
/// caller must manage every project the group has grants on.
#[utoipa::path(
  patch,
  path = crate::constants::api::groups::ITEM,
  tag = "groups",
  security(("cookieAuth" = [])),
  params(("id" = String, Path, description = "Group ID with grp_ prefix")),
  request_body = UpdateGroupRequest,
  responses(
    (status = 200, description = "Group updated", body = inline(HttpResponseFormat<GroupDetailResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "groups.manage, CSRF, or recent authentication is required, or the group reaches projects you don't manage", body = crate::http::ErrorBody),
    (status = 404, description = "The group was not found", body = crate::http::ErrorBody),
    (status = 409, description = "A group with this name already exists", body = crate::http::ErrorBody),
    (status = 422, description = "The name or description is invalid", body = crate::http::ErrorBody),
  ),
)]
pub async fn update(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path(id): Path<String>,
  axum::Json(request): axum::Json<UpdateGroupRequest>,
) -> Result<HttpResponse<GroupDetailResponse>, crate::http::HttpError> {
  Ok(HttpResponse::ok(
    service::update(&state, &identity, &headers, &id, request).await?,
    "GROUP_UPDATED",
  ))
}

/// Delete a group
///
/// Its members lose the group's grants. Same rights as updating it.
#[utoipa::path(
  delete,
  path = crate::constants::api::groups::ITEM,
  tag = "groups",
  security(("cookieAuth" = [])),
  params(("id" = String, Path, description = "Group ID with grp_ prefix")),
  responses(
    (status = 200, description = "Group deleted", body = inline(HttpResponseFormat<serde_json::Value>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "groups.manage, CSRF, or recent authentication is required, or the group reaches projects you don't manage", body = crate::http::ErrorBody),
    (status = 404, description = "The group was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn delete(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path(id): Path<String>,
) -> Result<HttpResponse<serde_json::Value>, crate::http::HttpError> {
  service::delete(&state, &identity, &headers, &id).await?;
  Ok(HttpResponse::done("GROUP_DELETED"))
}

/// Add a member
///
/// Add a user to a group. Adding someone already in it changes nothing.
/// Same rights as updating the group.
#[utoipa::path(
  put,
  path = crate::constants::api::groups::MEMBER,
  tag = "groups",
  security(("cookieAuth" = [])),
  params(
    ("id" = String, Path, description = "Group ID with grp_ prefix"),
    ("user_id" = String, Path, description = "User ID with usr_ prefix"),
  ),
  responses(
    (status = 200, description = "Member added", body = inline(HttpResponseFormat<GroupDetailResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "The caller can't change this group, or the user is root", body = crate::http::ErrorBody),
    (status = 404, description = "The group or user was not found", body = crate::http::ErrorBody),
  ),
)]
pub async fn add_member(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path((id, user_id)): Path<(String, String)>,
) -> Result<HttpResponse<GroupDetailResponse>, crate::http::HttpError> {
  Ok(HttpResponse::ok(
    service::add_member(&state, &identity, &headers, &id, &user_id).await?,
    "GROUP_MEMBER_ADDED",
  ))
}

/// Remove a member
///
/// The user loses the group's grants on their next request. Same rights as
/// updating the group.
#[utoipa::path(
  delete,
  path = crate::constants::api::groups::MEMBER,
  tag = "groups",
  security(("cookieAuth" = [])),
  params(
    ("id" = String, Path, description = "Group ID with grp_ prefix"),
    ("user_id" = String, Path, description = "User ID with usr_ prefix"),
  ),
  responses(
    (status = 200, description = "Member removed", body = inline(HttpResponseFormat<GroupDetailResponse>)),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "The caller can't change this group", body = crate::http::ErrorBody),
    (status = 404, description = "The group was not found or the user is not in it", body = crate::http::ErrorBody),
  ),
)]
pub async fn remove_member(
  State(state): State<AppState>,
  headers: HeaderMap,
  identity: AuthIdentity,
  Path((id, user_id)): Path<(String, String)>,
) -> Result<HttpResponse<GroupDetailResponse>, crate::http::HttpError> {
  Ok(HttpResponse::ok(
    service::remove_member(&state, &identity, &headers, &id, &user_id).await?,
    "GROUP_MEMBER_REMOVED",
  ))
}
