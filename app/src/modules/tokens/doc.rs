use super::{controller, model::*};
use crate::http::{ErrorBody, HttpResponseFormat};
use utoipa::OpenApi;
#[derive(OpenApi)]
#[openapi(paths(controller::list,controller::create,controller::revoke),components(schemas(TokenMetadata,CreateTokenRequest,CreatedTokenResponse,ErrorBody,HttpResponseFormat<Vec<TokenMetadata>>,HttpResponseFormat<CreatedTokenResponse>,HttpResponseFormat<TokenMetadata>)),tags((name="tokens")))]
struct TokensApi;
pub fn build() -> utoipa::openapi::OpenApi {
    TokensApi::openapi()
}
