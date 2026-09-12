use crate::cli::{
  client::{self as api_client, ApiClient},
  local_config,
};
use crate::cli::{commands::client as command_client, output, prompt};
use crate::constants::api;
use anyhow::{Context, Result, bail};
use reqwest::Method;
use serde_json::Value;
use std::path::Path;

pub(crate) async fn execute(
  server: &local_config::ResolvedServer,
  args: RestoreArgs,
  json_output: bool,
) -> Result<i32> {
  let RestoreArgs {
    path,
    key: key_input,
    setup_token,
    yes,
  } = args;
  if !path.exists() {
    bail!("Backup file not found: {}", path.display());
  }
  let file_name = path
    .file_name()
    .and_then(|s| s.to_str())
    .unwrap_or("backup.dop");

  if !file_name.ends_with(".dop") {
    bail!("Invalid backup file: must have a .dop extension");
  }

  command_client::ensure_server_is_connected(server, "restore").await?;

  let master_key_bytes = if let Some(k) = key_input {
    let p = Path::new(&k);
    let raw = if p.exists() {
      std::fs::read(p)
        .with_context(|| format!("Failed to read master key file: {}", p.display()))?
    } else {
      k.into_bytes()
    };
    Some(crate::services::crypto::parse_master_key(&raw)?)
  } else {
    None
  };

  let bytes = std::fs::read(&path)
    .with_context(|| format!("Failed to read backup file: {}", path.display()))?;

  let anon_client = ApiClient::new(server, None)?;
  let status_res = anon_client
    .request(Method::GET, api::bootstrap::STATUS, None)
    .await?;

  let state = status_res
    .get("state")
    .and_then(Value::as_str)
    .unwrap_or("ready");

  if !yes {
    output::print_warning("Restoring will overwrite existing projects, environments, and secrets.");
    prompt::confirm(
      &format!("Proceed with restoring from \"{}\"?", path.display()),
      yes,
    )?;
  }

  if state == "setupRequired" {
    if !json_output {
      output::print_progress(
        1,
        2,
        &format!(
          "Connecting to the uninitialized server at {}...",
          server.url
        ),
      );
      output::print_progress(
        2,
        2,
        "Restoring the snapshot and initializing the instance...",
      );
    }
    let setup_token = setup_token
      .as_deref()
      .context("--setup-token is required when restoring an uninitialized server")?;
    let res = anon_client
      .upload_bootstrap_multipart(
        api::bootstrap::RESTORE,
        file_name,
        bytes,
        master_key_bytes,
        setup_token,
      )
      .await?;

    if json_output {
      output::print_json(&res)?;
    } else {
      output::print_success("Restore complete.");
      output::print_fields(&[
        ("Source:", path.display().to_string()),
        ("Server:", format!("{} (initialized)", server.url)),
        ("Sign in:", format!("{}/login", server.url)),
        ("Status:", "ready".into()),
      ]);
    }
  } else {
    if !json_output {
      output::print_progress(1, 3, "Authenticating the administrator session...");
    }
    let auth_client = api_client::recently_authenticated_client(server).await?;

    if !json_output {
      output::print_progress(2, 3, "Uploading the backup snapshot...");
    }
    let upload_res = auth_client
      .upload_backup_multipart(
        api::backups::UPLOAD,
        file_name,
        bytes,
        master_key_bytes.clone(),
      )
      .await?;
    let key = upload_res
      .get("key")
      .and_then(Value::as_str)
      .context("Uploaded backup missing key")?;

    if !json_output {
      output::print_progress(3, 3, "Restoring database tables and running migrations...");
    }
    let restore_url = api::backups::restore(key);
    let restore_body = master_key_bytes
      .as_ref()
      .map(|k| serde_json::json!({ "master_key": hex::encode(k) }));
    let restore_res = auth_client
      .request(Method::POST, &restore_url, restore_body)
      .await?;

    if json_output {
      output::print_json(&restore_res)?;
    } else {
      output::print_success("Restore complete.");
      output::print_fields(&[
        ("Source:", path.display().to_string()),
        ("Key:", key.to_owned()),
        ("Server:", server.url.clone()),
        ("Status:", "ready".into()),
      ]);
    }
  }

  Ok(0)
}
use super::RestoreArgs;
