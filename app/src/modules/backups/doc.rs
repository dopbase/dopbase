use super::{
  controller,
  model::{BackupItem, CreateBackupRequest},
};
use crate::http::{ErrorBody, HttpResponseFormat};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
  paths(
    controller::list,
    controller::create,
    controller::download,
    controller::download_master_key,
    controller::upload,
    controller::restore,
    controller::delete,
  ),
  components(schemas(
    BackupItem,
    CreateBackupRequest,
    super::model::RestoreBackupRequest,
    ErrorBody,
    HttpResponseFormat<BackupItem>,
    HttpResponseFormat<Vec<BackupItem>>,
    HttpResponseFormat<serde_json::Value>,
  )),
  tags((name = "backups", description = "Backup and restore operations"))
)]
struct BackupsApi;

pub fn build() -> utoipa::openapi::OpenApi {
  BackupsApi::openapi()
}
