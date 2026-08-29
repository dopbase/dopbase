use super::{error::SecretError, model::*, service};
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
#[utoipa::path(get,path="/api/v1/environments/{environment_id}/secrets",tag="secrets",security(("bearerAuth"=[]),("cookieAuth"=[])),params(("environment_id"=String,Path)),responses((status=200,body=inline(HttpResponseFormat<Vec<SecretMetadata>>)),(status=401,body=ErrorBody)))]
pub async fn list(
    State(state): State<AppState>,
    identity: AuthIdentity,
    Path(id): Path<String>,
) -> Result<HttpResponse<Vec<SecretMetadata>>, SecretError> {
    Ok(HttpResponse::ok(
        service::list(&state, &identity, &id).await?,
        "SECRETS_FETCHED",
    ))
}
#[utoipa::path(get,path="/api/v1/environments/{environment_id}/secrets/{key}",tag="secrets",security(("bearerAuth"=[]),("cookieAuth"=[])),params(("environment_id"=String,Path),("key"=String,Path)),responses((status=200,body=inline(HttpResponseFormat<SecretMetadata>)),(status=404,body=ErrorBody)))]
pub async fn get(
    State(state): State<AppState>,
    identity: AuthIdentity,
    Path((id, key)): Path<(String, String)>,
) -> Result<HttpResponse<SecretMetadata>, SecretError> {
    Ok(HttpResponse::ok(
        service::get(&state, &identity, &id, &key).await?,
        "SECRET_FETCHED",
    ))
}
#[utoipa::path(put,path="/api/v1/environments/{environment_id}/secrets/{key}",tag="secrets",security(("bearerAuth"=[]),("cookieAuth"=[])),params(("environment_id"=String,Path),("key"=String,Path)),request_body=SetSecretRequest,responses((status=200,body=inline(HttpResponseFormat<SecretMetadata>)),(status=422,body=ErrorBody)))]
pub async fn set(
    State(state): State<AppState>,
    headers: HeaderMap,
    identity: AuthIdentity,
    Path((id, key)): Path<(String, String)>,
    axum::Json(request): axum::Json<SetSecretRequest>,
) -> Result<HttpResponse<SecretMetadata>, SecretError> {
    require_mutation(&identity, &headers)?;
    Ok(HttpResponse::ok(
        service::set(&state, &identity, &id, key, request).await?,
        "SECRET_SAVED",
    ))
}
#[utoipa::path(delete,path="/api/v1/environments/{environment_id}/secrets/{key}",tag="secrets",security(("bearerAuth"=[]),("cookieAuth"=[])),params(("environment_id"=String,Path),("key"=String,Path)),responses((status=200,body=inline(HttpResponseFormat<serde_json::Value>)),(status=404,body=ErrorBody)))]
pub async fn delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    identity: AuthIdentity,
    Path((id, key)): Path<(String, String)>,
) -> Result<HttpResponse<serde_json::Value>, SecretError> {
    require_mutation(&identity, &headers)?;
    service::delete(&state, &identity, &id, &key).await?;
    Ok(HttpResponse::done("SECRET_DELETED"))
}
#[utoipa::path(post,path="/api/v1/environments/{environment_id}/secrets/{key}/reveal",tag="secrets",security(("bearerAuth"=[]),("cookieAuth"=[])),params(("environment_id"=String,Path),("key"=String,Path)),responses((status=200,body=inline(HttpResponseFormat<RevealedSecret>)),(status=403,body=ErrorBody),(status=404,body=ErrorBody)))]
pub async fn reveal(
    State(state): State<AppState>,
    headers: HeaderMap,
    identity: AuthIdentity,
    Path((id, key)): Path<(String, String)>,
) -> Result<HttpResponse<RevealedSecret>, SecretError> {
    require_mutation(&identity, &headers)?;
    Ok(HttpResponse::ok(
        service::reveal(&state, &identity, &id, &key).await?,
        "SECRET_REVEALED",
    ))
}
#[utoipa::path(post,path="/api/v1/environments/{environment_id}/secrets/import",tag="secrets",security(("bearerAuth"=[]),("cookieAuth"=[])),params(("environment_id"=String,Path)),request_body=ImportSecretsRequest,responses((status=200,body=inline(HttpResponseFormat<ImportSecretsResponse>)),(status=422,body=ErrorBody)))]
pub async fn import(
    State(state): State<AppState>,
    headers: HeaderMap,
    identity: AuthIdentity,
    Path(id): Path<String>,
    axum::Json(request): axum::Json<ImportSecretsRequest>,
) -> Result<HttpResponse<ImportSecretsResponse>, SecretError> {
    require_mutation(&identity, &headers)?;
    Ok(HttpResponse::ok(
        service::import(&state, &identity, &id, request).await?,
        "SECRETS_IMPORTED",
    ))
}
#[utoipa::path(get,path="/api/v1/environments/{environment_id}/secrets/layout",tag="secrets",security(("bearerAuth"=[]),("cookieAuth"=[])),params(("environment_id"=String,Path)),responses((status=200,body=inline(HttpResponseFormat<EnvLayoutResponse>)),(status=401,body=ErrorBody)))]
pub async fn layout(
    State(state): State<AppState>,
    identity: AuthIdentity,
    Path(id): Path<String>,
) -> Result<HttpResponse<EnvLayoutResponse>, SecretError> {
    Ok(HttpResponse::ok(
        service::layout(&state, &identity, &id).await?,
        "SECRETS_LAYOUT_FETCHED",
    ))
}
#[utoipa::path(post,path="/api/v1/environments/{environment_id}/secrets/export",tag="secrets",security(("bearerAuth"=[]),("cookieAuth"=[])),params(("environment_id"=String,Path)),responses((status=200,body=inline(HttpResponseFormat<ExportSecretsResponse>)),(status=403,body=ErrorBody)))]
pub async fn export(
    State(state): State<AppState>,
    headers: HeaderMap,
    identity: AuthIdentity,
    Path(id): Path<String>,
) -> Result<HttpResponse<ExportSecretsResponse>, SecretError> {
    require_mutation(&identity, &headers)?;
    Ok(HttpResponse::ok(
        service::export(&state, &identity, &id).await?,
        "SECRETS_EXPORTED",
    ))
}
#[utoipa::path(get,path="/api/v1/environments/{environment_id}/secrets/runtime",tag="secrets",security(("bearerAuth"=[])),params(("environment_id"=String,Path)),responses((status=200,body=inline(HttpResponseFormat<RuntimeSecretsResponse>)),(status=403,body=ErrorBody)))]
pub async fn runtime(
    State(state): State<AppState>,
    identity: AuthIdentity,
    Path(id): Path<String>,
) -> Result<HttpResponse<RuntimeSecretsResponse>, SecretError> {
    Ok(HttpResponse::ok(
        service::runtime(&state, &identity, &id).await?,
        "RUNTIME_SECRETS_FETCHED",
    ))
}
