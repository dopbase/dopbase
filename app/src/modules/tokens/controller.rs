use super::{error::TokenError, model::*, service};
use crate::{
    extractors::require_mutation,
    http::{ErrorBody, HttpResponse, HttpResponseFormat},
    models::AuthIdentity,
    state::AppState,
};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
};
#[utoipa::path(get,path="/api/v1/environments/{environment_id}/tokens",tag="tokens",security(("bearerAuth"=[]),("cookieAuth"=[])),params(("environment_id"=String,Path)),responses((status=200,body=inline(HttpResponseFormat<Vec<TokenMetadata>>)),(status=401,body=ErrorBody)))]
pub async fn list(
    State(state): State<AppState>,
    identity: AuthIdentity,
    Path(id): Path<String>,
) -> Result<HttpResponse<Vec<TokenMetadata>>, TokenError> {
    Ok(HttpResponse::ok(
        service::list(&state, &identity, &id).await?,
        "TOKENS_FETCHED",
    ))
}
#[utoipa::path(post,path="/api/v1/environments/{environment_id}/tokens",tag="tokens",security(("bearerAuth"=[]),("cookieAuth"=[])),params(("environment_id"=String,Path)),request_body=CreateTokenRequest,responses((status=201,body=inline(HttpResponseFormat<CreatedTokenResponse>)),(status=409,body=ErrorBody),(status=422,body=ErrorBody)))]
pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    identity: AuthIdentity,
    Path(id): Path<String>,
    axum::Json(request): axum::Json<CreateTokenRequest>,
) -> Result<HttpResponse<CreatedTokenResponse>, TokenError> {
    require_mutation(&identity, &headers)?;
    Ok(HttpResponse::created(
        service::create(&state, &identity, &id, request).await?,
        "TOKEN_CREATED",
    ))
}
#[utoipa::path(post,path="/api/v1/tokens/{token_id}/revoke",tag="tokens",security(("bearerAuth"=[]),("cookieAuth"=[])),params(("token_id"=String,Path)),responses((status=200,body=inline(HttpResponseFormat<TokenMetadata>)),(status=404,body=ErrorBody),(status=409,body=ErrorBody)))]
pub async fn revoke(
    State(state): State<AppState>,
    headers: HeaderMap,
    identity: AuthIdentity,
    Path(id): Path<String>,
) -> Result<HttpResponse<TokenMetadata>, TokenError> {
    require_mutation(&identity, &headers)?;
    Ok(HttpResponse::ok(
        service::revoke(&state, &identity, &id).await?,
        "TOKEN_REVOKED",
    ))
}
