use super::{controller, model::*};
use crate::http::{ErrorBody, HttpResponseFormat};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
  paths(
    controller::list,
    controller::create,
    controller::get,
    controller::delete,
    controller::tokens,
    controller::create_token,
    controller::revoke_token
  ),
  components(schemas(
    ServiceAccountResponse,
    CreateServiceAccountRequest,
    AgentTokenResponse,
    CreateAgentTokenRequest,
    CreatedAgentTokenResponse,
    ErrorBody,
    HttpResponseFormat<Vec<ServiceAccountResponse>>,
    HttpResponseFormat<ServiceAccountResponse>,
    HttpResponseFormat<Vec<AgentTokenResponse>>,
    HttpResponseFormat<CreatedAgentTokenResponse>,
    HttpResponseFormat<AgentTokenResponse>,
    HttpResponseFormat<serde_json::Value>
  )),
  tags((name = "service-accounts"))
)]
struct Api;

pub fn build() -> utoipa::openapi::OpenApi {
  Api::openapi()
}
