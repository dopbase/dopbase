use super::{error::AuditError, model::*, service};
use crate::{
  http::{HttpResponse, HttpResponseFormat},
  models::AuthIdentity,
  state::AppState,
};
use axum::extract::{Query, State};

/// List audit events
///
/// Return a cursor-paginated page of audit events, newest first. Filter by
/// `action`, `projectId`, `environmentId`, or `actor`; `limit` is clamped
/// to 1-200. The `nextCursor` value is opaque; pass it back unchanged to
/// fetch the next page.
#[utoipa::path(
  get,
  path = "/api/v1/audit-events",
  tag = "audit",
  security(("bearerAuth" = []), ("cookieAuth" = [])),
  params(AuditQuery),
  responses(
    (status = 200, description = "Audit events fetched", body = inline(HttpResponseFormat<AuditPage>)),
    (status = 400, description = "The pagination cursor is invalid", body = crate::http::ErrorBody),
    (status = 401, description = "Authentication is required", body = crate::http::ErrorBody),
    (status = 403, description = "Only administrators may read audit events", body = crate::http::ErrorBody),
  ),
)]
pub async fn list(
  State(state): State<AppState>,
  identity: AuthIdentity,
  Query(query): Query<AuditQuery>,
) -> Result<HttpResponse<AuditPage>, AuditError> {
  Ok(HttpResponse::ok(
    service::list(&state, &identity, query).await?,
    "AUDIT_EVENTS_FETCHED",
  ))
}
