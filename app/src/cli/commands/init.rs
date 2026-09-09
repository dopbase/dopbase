use super::output;
use crate::cli::{client, dotenv, local_config::ResolvedServer};
use anyhow::Result;
use reqwest::Method;
use serde_json::json;
use std::path::Path;

pub(super) async fn execute(
  server: &ResolvedServer,
  project: String,
  environment: String,
  from: &Path,
  json_output: bool,
) -> Result<i32> {
  let api = client::human_client(server).await?;
  let entries = dotenv::parse_file(from)?;
  let data = api
    .request(
      Method::POST,
      "/api/v1/projects/init",
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
