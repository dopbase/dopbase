use super::{controller, model::*};
use crate::{
  http::{ErrorBody, HttpResponseFormat},
  models::SecretInput,
};
use utoipa::OpenApi;
#[derive(OpenApi)]
#[openapi(paths(controller::list,controller::get,controller::set,controller::delete,controller::reveal,controller::import,controller::export,controller::runtime,controller::layout,controller::versions,controller::reveal_version,controller::rollback,controller::compare,controller::copy),components(schemas(CompareEnvironment,CompareCell,CompareKey,CompareResponse,CopySecretRequest,HttpResponseFormat<CompareResponse>,SecretVersion,SecretVersionPage,RollbackSecretRequest,RollbackSecretResponse,HttpResponseFormat<SecretVersionPage>,HttpResponseFormat<RollbackSecretResponse>,SecretMetadata,SetSecretRequest,RevealedSecret,ImportMode,ImportSecretsRequest,ImportSecretsResponse,ExportSecretsResponse,RuntimeSecretsResponse,EnvLayoutResponse,SecretInput,ErrorBody,HttpResponseFormat<Vec<SecretMetadata>>,HttpResponseFormat<SecretMetadata>,HttpResponseFormat<RevealedSecret>,HttpResponseFormat<ImportSecretsResponse>,HttpResponseFormat<ExportSecretsResponse>,HttpResponseFormat<RuntimeSecretsResponse>,HttpResponseFormat<EnvLayoutResponse>,HttpResponseFormat<serde_json::Value>)),tags((name="secrets")))]
struct SecretsApi;
pub fn build() -> utoipa::openapi::OpenApi {
  SecretsApi::openapi()
}
