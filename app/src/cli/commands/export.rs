use super::{environment, output};
use crate::{
  cli::{client, dotenv, local_config},
  models::SecretInput,
};
use anyhow::{Context, Result, bail};
use reqwest::Method;
use serde_json::{Value, json};
use std::path::PathBuf;

pub(super) async fn execute(
  server: &local_config::ResolvedServer,
  reference: &str,
  output: Option<PathBuf>,
  stdout: bool,
  force: bool,
  json_output: bool,
) -> Result<i32> {
  if output.is_none() && !stdout {
    bail!("export requires --output <FILE> or --stdout");
  }
  if stdout && json_output {
    bail!("--stdout and --json cannot be combined");
  }
  let api = client::recently_authenticated_client(server).await?;
  let env = environment::resolve_environment(&api, reference).await?;
  let data = api
    .request(
      Method::POST,
      &format!(
        "/api/v1/environments/{}/secrets/export",
        environment::env_id(&env)?
      ),
      None,
    )
    .await?;
  let entries = parse_entries(&data)?;
  let rendered = dotenv::render(&entries);
  if stdout {
    print!("{rendered}");
  } else if let Some(path) = output {
    output::write_private(&path, rendered.as_bytes(), force)?;
    if json_output {
      output::print_json(&json!({"output":path,"secret_count":entries.len()}))?;
    } else {
      output::print_success(&format!(
        "Exported {} secret(s) to {}.",
        entries.len(),
        path.display()
      ));
    }
  }
  Ok(0)
}

fn parse_entries(value: &Value) -> Result<Vec<SecretInput>> {
  let entries = value
    .get("entries")
    .and_then(Value::as_array)
    .context("response did not contain entries")?;
  entries
    .iter()
    .map(|entry| {
      Ok(SecretInput {
        key: entry
          .get("key")
          .and_then(Value::as_str)
          .context("entry has no key")?
          .into(),
        value: entry
          .get("value")
          .and_then(Value::as_str)
          .context("entry has no value")?
          .into(),
      })
    })
    .collect()
}
