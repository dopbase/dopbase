use super::{controller, model::*};
use crate::http::{ErrorBody, HttpResponseFormat};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
  paths(
    controller::status,
    controller::public_status,
    controller::run_checks,
    controller::factory_reset_preview,
    controller::factory_reset
  ),
  components(schemas(
    InstanceStatus,
    StatusResponse,
    Platform,
    LatestRelease,
    CheckStatus,
    Check,
    StatusSummary,
    HealthReport,
    DatabaseHealth,
    MasterKeyHealth,
    DiskHealth,
    AuditLogHealth,
    Activity,
    SecretReads,
    FailedSignIns,
    RateLimited,
    ActiveSessions,
    FactoryResetPreview,
    FactoryResetRequest,
    ErrorBody,
    HttpResponseFormat<InstanceStatus>,
    HttpResponseFormat<StatusResponse>,
    HttpResponseFormat<FactoryResetPreview>,
    HttpResponseFormat<serde_json::Value>
  )),
  tags((name = "instance"))
)]
struct InstanceApi;

pub fn build() -> utoipa::openapi::OpenApi {
  InstanceApi::openapi()
}
