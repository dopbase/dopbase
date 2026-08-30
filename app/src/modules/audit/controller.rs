use super::{error::AuditError, model::*, service};
use crate::{
  http::{ErrorBody, HttpResponse, HttpResponseFormat},
  models::AuthIdentity,
  state::AppState,
};
use axum::extract::{Query, State};
#[utoipa::path(get,path="/api/v1/audit-events",tag="audit",security(("bearerAuth"=[]),("cookieAuth"=[])),params(AuditQuery),responses((status=200,body=inline(HttpResponseFormat<AuditPage>)),(status=401,body=ErrorBody)))]
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
