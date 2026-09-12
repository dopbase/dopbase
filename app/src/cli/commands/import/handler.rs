use crate::cli::{commands::environment, output, prompt};
use crate::{
  cli::{client, local_config, secret_format},
  constants::api,
};
use anyhow::Result;
use reqwest::Method;
use secret_format::SecretFormat;
use serde_json::{Value, json};

pub(crate) async fn execute(
  server: &local_config::ResolvedServer,
  args: ImportArgs,
  json_output: bool,
) -> Result<i32> {
  let ImportArgs {
    environment,
    path,
    format,
    dry_run,
    replace,
    yes,
  } = args;
  let format = SecretFormat::for_input(&path, format)?;
  let api = client::human_client(server).await?;
  let env = environment::resolve_environment(&api, &environment).await?;
  let id = environment::env_id(&env)?;
  let entries = secret_format::read(&path, Some(format))?;
  let mode = if replace { "replace" } else { "merge" };
  let endpoint = api::secrets::import(id);
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
      output::print_success(&format!("Imported secrets into {environment}."));
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
use super::ImportArgs;
