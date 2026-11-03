use super::{controller, model::*};
use crate::http::{ErrorBody, HttpResponseFormat};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
  paths(controller::list, controller::create, controller::revoke),
  components(schemas(
    PersonalTokenResponse,
    CreatePersonalTokenRequest,
    CreatedPersonalTokenResponse,
    ErrorBody,
    HttpResponseFormat<Vec<PersonalTokenResponse>>,
    HttpResponseFormat<CreatedPersonalTokenResponse>,
    HttpResponseFormat<PersonalTokenResponse>
  )),
  tags((name = "personal-tokens"))
)]
struct PersonalTokensApi;

pub fn build() -> utoipa::openapi::OpenApi {
  PersonalTokensApi::openapi()
}
