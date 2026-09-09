use super::{environment, output, prompt};
use crate::cli::{client, dotenv, local_config};
use anyhow::Result;
use reqwest::Method;
use serde_json::{Value, json};
use std::path::Path;

pub(super) async fn execute(
  server: &local_config::ResolvedServer,
  reference: &str,
  path: &Path,
  dry_run: bool,
  replace: bool,
  yes: bool,
  json_output: bool,
) -> Result<i32> {
  let api = client::human_client(server).await?;
  let env = environment::resolve_environment(&api, reference).await?;
  let id = environment::env_id(&env)?;
  let entries = dotenv::parse_file(path)?;
  let mode = if replace { "replace" } else { "merge" };
  let endpoint = format!("/api/v1/environments/{id}/secrets/import");
  let mut expected_revision = None;
  if replace && !dry_run {
    let preview = api
      .request(
        Method::POST,
        &endpoint,
        Some(json!({"mode":mode,"dryRun":true,"entries":entries})),
      )
      .await?;
    expected_revision = preview
      .get("revision")
      .and_then(Value::as_str)
      .map(str::to_owned);
    let deleted = preview
      .get("deletedKeys")
      .and_then(Value::as_array)
      .cloned()
      .unwrap_or_default();
    if !deleted.is_empty() {
      output::print_warning(&format!(
        "Replace will delete: {}",
        deleted
          .iter()
          .filter_map(Value::as_str)
          .collect::<Vec<_>>()
          .join(", ")
      ));
      prompt::confirm("Apply this replacement?", yes)?;
    }
  }
  let data = api
    .request(
      Method::POST,
      &endpoint,
      Some(json!({"mode":mode,"dryRun":dry_run,"entries":entries,"expectedRevision":expected_revision})),
    )
    .await?;
  if json_output {
    output::print_json(&data)?;
  } else {
    let count = |field: &str| {
      data
        .get(field)
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
        .to_string()
    };
    if dry_run {
      output::print_text("Dry run complete. No changes were applied.");
    } else {
      output::print_success(&format!("Imported secrets into {reference}."));
    }
    output::print_fields(&[
      ("Mode:", mode.into()),
      ("Added:", count("addedKeys")),
      ("Updated:", count("updatedKeys")),
      ("Unchanged:", count("unchangedKeys")),
      ("Deleted:", count("deletedKeys")),
    ]);
  }
  Ok(0)
}
