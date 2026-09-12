use super::output;
use crate::{
  cli::{
    client, environment_target::EnvironmentTarget, local_config::ResolvedServer, secret_format,
  },
  constants::api,
};
use anyhow::Result;
use reqwest::Method;
use serde_json::json;
use std::path::Path;

use secret_format::SecretFormat;

pub(super) async fn execute(
  server: &ResolvedServer,
  target: EnvironmentTarget,
  from: &Path,
  format: Option<SecretFormat>,
  json_output: bool,
) -> Result<i32> {
  let (project, environment) = target.into_parts();
  let format = SecretFormat::for_input(from, format)?;
  let api = client::human_client(server).await?;
  let entries = secret_format::read(from, Some(format))?;
  let data = api
    .request(
      Method::POST,
      api::projects::INIT,
      Some(json!({"projectName":&project,"environmentName":&environment,"entries":entries})),
    )
    .await?;
  if json_output {
    output::print_json(&data)?;
  } else {
    let created_project = data.get("project").unwrap_or(&serde_json::Value::Null);
    output::print_success(&format!("Initialized {project}/{environment}."));
    output::print_fields(&[
      ("Project ID:", output::string(created_project, "id")),
      ("Environment ID:", output::string(&data, "environmentId")),
      (
        "Secrets:",
        data
          .get("secretCount")
          .and_then(serde_json::Value::as_u64)
          .unwrap_or_default()
          .to_string(),
      ),
    ]);
  }
  Ok(0)
}
