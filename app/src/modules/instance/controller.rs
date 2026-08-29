use super::{error::InstanceError, model::InstanceStatus, service};
use crate::{
    http::{ErrorBody, HttpResponse, HttpResponseFormat},
    models::AuthIdentity,
    state::AppState,
};
use axum::extract::State;
#[utoipa::path(get,path="/api/v1/instance",tag="instance",security(("bearerAuth"=[]),("cookieAuth"=[])),responses((status=200,body=inline(HttpResponseFormat<InstanceStatus>)),(status=401,body=ErrorBody)))]
pub async fn status(
    State(state): State<AppState>,
    identity: AuthIdentity,
) -> Result<HttpResponse<InstanceStatus>, InstanceError> {
    Ok(HttpResponse::ok(
        service::status(&state, &identity).await?,
        "INSTANCE_STATUS_FETCHED",
    ))
}
