use crate::cli::{commands::client as command_client, output};
use crate::{
  cli::{client as api_client, local_config},
  constants::api,
};
use anyhow::{Context, Result};
use reqwest::Method;
use serde_json::{Value, json};
fn format_bytes(bytes: u64) -> String {
  const KIB: u64 = 1024;
  const MIB: u64 = KIB * 1024;
  const GIB: u64 = MIB * 1024;

  if bytes >= GIB {
    format!("{:.2} GiB", bytes as f64 / GIB as f64)
  } else if bytes >= MIB {
    format!("{:.2} MiB", bytes as f64 / MIB as f64)
  } else if bytes >= KIB {
    format!("{:.2} KiB", bytes as f64 / KIB as f64)
  } else {
    format!("{} B", bytes)
  }
}

pub(crate) async fn execute(
  server: &local_config::ResolvedServer,
  args: BackupArgs,
  json_output: bool,
) -> Result<i32> {
  let BackupArgs { name, output } = args;
  command_client::ensure_server_is_connected(server, "backup").await?;

  let steps_total = if output.is_some() { 2 } else { 1 };
  if !json_output {
    output::print_progress(
      1,
      steps_total,
      &format!("Creating an encrypted backup on {}...", server.url),
    );
  }
  let api = api_client::human_client(server).await?;
  let payload = match &name {
    Some(n) => json!({ "name": n }),
    None => json!({}),
  };
  let data = api
    .request(Method::POST, api::backups::COLLECTION, Some(payload))
    .await?;
  let key = data
    .get("key")
    .and_then(Value::as_str)
    .context("Backup response missing key")?;
  let size = data.get("size").and_then(Value::as_u64).unwrap_or(0);

  let local_path_saved = if let Some(out_path) = output {
    if !json_output {
      output::print_progress(
        2,
        steps_total,
        &format!("Downloading the backup to {}...", out_path.display()),
      );
    }
    let download_url = api::backups::item(key);
    let bytes = api.download_bytes(&download_url).await?;
    output::write_private(&out_path, &bytes, false)?;
    Some(out_path)
  } else {
    None
  };

  if json_output {
    let mut out_json = data.clone();
    if let Some(p) = &local_path_saved {
      out_json["localPath"] = json!(p);
    }
    output::print_json(&out_json)?;
  } else {
    output::print_success("Backup complete.");
    let mut fields = vec![
      ("Filename:", key.to_owned()),
      ("Size:", format_bytes(size)),
      (
        "Server:",
        format!("{} (~/.dopbase/backups/{key})", server.url),
      ),
    ];
    if let Some(path) = &local_path_saved {
      fields.push(("Saved to:", path.display().to_string()));
    }
    output::print_fields(&fields);
    output::print_warning(
      "This backup uses the instance master key. Restoring it on another server requires both files.",
    );
  }

  Ok(0)
}
use super::BackupArgs;
