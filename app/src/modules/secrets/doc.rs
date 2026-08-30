use super::{controller, model::*};
use crate::{
  http::{ErrorBody, HttpResponseFormat},
  models::SecretInput,
};
use utoipa::OpenApi;
#[derive(OpenApi)]
#[openapi(paths(controller::list,controller::get,controller::set,controller::delete,controller::reveal,controller::import,controller::export,controller::runtime,controller::layout),components(schemas(SecretMetadata,SetSecretRequest,RevealedSecret,ImportMode,ImportSecretsRequest,ImportSecretsResponse,ExportSecretsResponse,RuntimeSecretsResponse,EnvLayoutResponse,SecretInput,ErrorBody,HttpResponseFormat<Vec<SecretMetadata>>,HttpResponseFormat<SecretMetadata>,HttpResponseFormat<RevealedSecret>,HttpResponseFormat<ImportSecretsResponse>,HttpResponseFormat<ExportSecretsResponse>,HttpResponseFormat<RuntimeSecretsResponse>,HttpResponseFormat<EnvLayoutResponse>,HttpResponseFormat<serde_json::Value>)),tags((name="secrets")))]
struct SecretsApi;
pub fn build() -> utoipa::openapi::OpenApi {
  SecretsApi::openapi()
}
