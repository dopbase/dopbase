use super::{controller, model::*};
use crate::http::{ErrorBody, HttpResponseFormat};
use utoipa::OpenApi;
#[derive(OpenApi)]
#[openapi(
  paths(
    controller::list,
    controller::list_workspace,
    controller::create,
    controller::create_workspace,
    controller::revoke
  ),
  components(schemas(
    TokenMetadata,
    WorkspaceTokenMetadata,
    CreateTokenRequest,
    CreatedTokenResponse,
    CreatedWorkspaceTokenResponse,
    RevokedTokenMetadata,
    ErrorBody,
    HttpResponseFormat<Vec<TokenMetadata>>,
    HttpResponseFormat<Vec<WorkspaceTokenMetadata>>,
    HttpResponseFormat<CreatedTokenResponse>,
    HttpResponseFormat<CreatedWorkspaceTokenResponse>,
    HttpResponseFormat<RevokedTokenMetadata>
  )),
  tags((name = "tokens"))
)]
struct TokensApi;
pub fn build() -> utoipa::openapi::OpenApi {
  TokensApi::openapi()
}
