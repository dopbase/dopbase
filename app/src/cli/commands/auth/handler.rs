use crate::cli::{output, prompt};
use crate::cli::{client, local_config::ResolvedServer};
use anyhow::{Result, bail};
use serde_json::json;
use std::io::{self, IsTerminal, Read};

pub(crate) async fn login(
  server: &ResolvedServer,
  args: super::LoginArgs,
  json_output: bool,
) -> Result<i32> {
  let token = args.token;
  if token {
    let token = read_runner_token()?;
    client::validate_runner_token(&token)?;
    client::save_credential(server, &token, None)?;
  } else {
    let _ = client::login(server, true).await?;
  }
  let data = json!({"server_url":server.url,"authentication":"encrypted_session"});
  if json_output {
    output::print_json(&data)?;
  } else {
    let message = if token {
      format!("Saved a runner token for {}.", server.url)
    } else {
      format!("Logged in to {}.", server.url)
    };
    output::print_success(&message);
  }
  Ok(0)
}

fn read_runner_token() -> Result<String> {
  let token = if io::stdin().is_terminal() {
    prompt::password("Runner token:", false, client::CliCancelled::TokenInput)?
  } else {
    let mut token = String::new();
    io::stdin().read_to_string(&mut token)?;
    token.trim().to_owned()
  };
  if token.is_empty() {
    bail!("Runner token cannot be empty.");
  }
  Ok(token)
}

pub(crate) fn logout(
  server: &ResolvedServer,
  json_output: bool,
) -> Result<i32> {
  client::remove_credential(server)?;
  let credential = client::credential(server)?;
  let data = json!({"server_url":server.url,"authentication":credential.source.as_str()});
  if json_output {
    output::print_json(&data)?;
  } else {
    output::print_success(&format!("Logged out from {}.", server.url));
  }
  Ok(0)
}
