use super::{client as command_client, output};
use crate::cli::{client as api_client, local_config};
use anyhow::{Context, Result};
use reqwest::Method;
use serde_json::{Value, json};
use std::path::PathBuf;

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

pub(super) async fn execute(
  server: &local_config::ResolvedServer,
  name: Option<String>,
  output: Option<PathBuf>,
  json_output: bool,
) -> Result<i32> {
  command_client::ensure_server_is_connected(server, "backup").await?;

  let steps_total = if output.is_some() { 2 } else { 1 };
  if !json_output {
    eprintln!(
      "==> [1/{steps_total}] Creating encrypted backup snapshot on server ({})...",
      server.url
    );
  }
  let api = api_client::human_client(server).await?;
  let payload = match &name {
    Some(n) => json!({ "name": n }),
    None => json!({}),
  };
  let data = api
    .request(Method::POST, "/api/v1/backups", Some(payload))
    .await?;
  let key = data
    .get("key")
    .and_then(Value::as_str)
    .context("Backup response missing key")?;
  let size = data.get("size").and_then(Value::as_u64).unwrap_or(0);

  let local_path_saved = if let Some(out_path) = output {
    if !json_output {
      eprintln!(
        "==> [2/{steps_total}] Downloading backup archive to {}...",
        out_path.display()
      );
    }
    let download_url = format!("/api/v1/backups/{key}");
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
    output::print_value(true, &out_json);
  } else {
    println!();
    println!("Backup Completed Successfully");
    println!("  Filename: {}", key);
    println!("  Size:     {}", format_bytes(size));
    println!("  Server:   {} (~/.dopbase/backups/{})", server.url, key);
    if let Some(p) = &local_path_saved {
      println!("  Saved To: {}", p.display());
    }
    println!("  Status:   Ready");
    println!();
    println!(
      "  Notice: This backup is encrypted with this instance's master key (~/.dopbase/master.key)."
    );
    println!(
      "          Restoring on a different server requires both this .dop file and the master key."
    );
  }

  Ok(0)
}
