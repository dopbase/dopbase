use crate::cli::{client, local_config::ResolvedServer, session};
use crate::cli::{output, prompt};
use crate::constants::config::ENV_TOKEN;
use anyhow::{Result, bail};
use serde_json::json;
use std::{
  env,
  io::{self, IsTerminal, Read},
  path::Path,
};

const LOGIN_WITH_ENV_TOKEN: &str = "DOPBASE_TOKEN is set, so Dopbase would ignore a saved login. \
Unset DOPBASE_TOKEN, then run dopbase login again.";
const LOGOUT_WITH_ENV_TOKEN: &str = "DOPBASE_TOKEN is set, so Dopbase signs in with that token, \
not a saved login. Unset DOPBASE_TOKEN to stop using it.";

pub(crate) async fn login(
  server: &ResolvedServer,
  args: super::LoginArgs,
  json_output: bool,
) -> Result<i32> {
  // Checked before any prompt or stdin read: the token would hide whatever
  // this saves.
  if client::environment_token_set(env::var(ENV_TOKEN))? {
    bail!(LOGIN_WITH_ENV_TOKEN);
  }
  let token = args.token;
  if token {
    let token = read_runner_token()?;
    client::validate_runner_token(&token)?;
    client::save_credential(server, &token, None)?;
  } else if args.password {
    let _ = client::login(server, true).await?;
  } else if args.browser || args.browser_open {
    let _ = client::browser_login(server, args.browser_open, true).await?;
  } else {
    let _ = client::interactive_login(server, true).await?;
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

/// The flags this logout was given, so the suggested command reaches the
/// same saved login.
pub(crate) struct LogoutFlags<'a> {
  pub server: Option<&'a str>,
  pub data_dir: Option<&'a Path>,
}

pub(crate) fn logout(
  server: &ResolvedServer,
  flags: LogoutFlags<'_>,
  json_output: bool,
) -> Result<i32> {
  // Refuse before touching anything: the token keeps signing every command
  // in, whatever happens to the saved login.
  if client::environment_token_set(env::var(ENV_TOKEN))? {
    let mut message = LOGOUT_WITH_ENV_TOKEN.to_owned();
    if session::is_saved(server)? {
      message.push_str("\nA saved login also exists. To remove it, run: ");
      message.push_str(&logout_without_token(&flags));
    }
    bail!(message);
  }
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

/// `dopbase logout` with DOPBASE_TOKEN hidden for that one command, keeping
/// the --data-dir and --server this logout was given.
fn logout_without_token(flags: &LogoutFlags<'_>) -> String {
  let mut command = "env -u DOPBASE_TOKEN dopbase".to_owned();
  if let Some(data_dir) = flags.data_dir {
    command.push_str(" --data-dir ");
    command.push_str(&output::shell_quote(&data_dir.display().to_string()));
  }
  if let Some(server) = flags.server {
    command.push_str(" --server ");
    command.push_str(&output::shell_quote(server));
  }
  command.push_str(" logout");
  command
}
