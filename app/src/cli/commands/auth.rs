use super::output;
use crate::cli::{client, local_config::ResolvedServer};
use anyhow::Result;
use serde_json::json;

pub(super) async fn login(
  server: &ResolvedServer,
  json_output: bool,
) -> Result<i32> {
  let _ = client::login(server, true).await?;
  output::print_value(
    json_output,
    &json!({"server_url":server.url,"authentication":"encrypted_session"}),
  );
  Ok(0)
}

pub(super) fn logout(
  server: &ResolvedServer,
  json_output: bool,
) -> Result<i32> {
  client::remove_credential(server)?;
  let credential = client::credential(server)?;
  output::print_value(
    json_output,
    &json!({"server_url":server.url,"authentication":credential.source.as_str()}),
  );
  Ok(0)
}
