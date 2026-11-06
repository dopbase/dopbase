use crate::cli::{client, local_config::ResolvedServer, session};
use crate::cli::{output, prompt};
use crate::constants::{api, config::ENV_TOKEN, tokens::ADMIN_SESSION_PREFIX};
use anyhow::{Result, bail};
use reqwest::Method;
use serde_json::json;
use std::{
  env,
  io::{self, IsTerminal, Read},
  path::Path,
  time::Duration,
};

const LOGIN_WITH_ENV_TOKEN: &str = "DOPBASE_TOKEN is set, so Dopbase would ignore a saved login. \
Unset DOPBASE_TOKEN, then run dopbase login again.";
const LOGOUT_WITH_ENV_TOKEN: &str = "DOPBASE_TOKEN is set, so Dopbase signs in with that token, \
not a saved login. Unset DOPBASE_TOKEN to stop using it.";
const RUNNER_TOKEN_STILL_WORKS: &str = "The runner token still works. Revoke it with \
dopbase token revoke or in the Admin UI when you no longer need it.";
/// How long logout waits for the server before deleting the files anyway.
const LOGOUT_TIMEOUT: Duration = Duration::from_secs(10);

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

/// What logging out did to the saved login on the server.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ServerSession {
  Revoked,
  AlreadyEnded,
  /// The server didn't confirm, so the session may still be valid there.
  NotReached,
  /// The saved login is damaged, so its session couldn't be ended.
  Unreadable,
  /// Nothing to end: no saved login, or a runner token.
  NotApplicable,
}

impl ServerSession {
  fn as_str(self) -> &'static str {
    match self {
      Self::Revoked => "revoked",
      Self::AlreadyEnded => "already_ended",
      Self::NotReached => "not_reached",
      Self::Unreadable => "unreadable",
      Self::NotApplicable => "not_applicable",
    }
  }
}

/// The flags this logout was given, so the suggested command reaches the
/// same saved login.
pub(crate) struct LogoutFlags<'a> {
  pub server: Option<&'a str>,
  pub data_dir: Option<&'a Path>,
}

pub(crate) async fn logout(
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
  let mut runner_token = false;
  // The saved login can belong to another server when --server or
  // DOPBASE_URL points elsewhere. It is ended on the server that issued it,
  // and its token is never sent to the one asked for.
  let mut session_url = server.url.clone();
  let (server_session, problem) = match session::load_any(server) {
    Ok(Some(saved)) => {
      session_url = saved.server_url;
      if saved.session.token.starts_with(ADMIN_SESSION_PREFIX) {
        let issuer = ResolvedServer {
          url: session_url.clone(),
          source: server.source,
          config_path: server.config_path.clone(),
          config: Default::default(),
        };
        end_server_session(&issuer, saved.session.token).await
      } else {
        runner_token = true;
        (ServerSession::NotApplicable, None)
      }
    }
    Ok(None) => (ServerSession::NotApplicable, None),
    Err(_) => (
      ServerSession::Unreadable,
      Some("the saved login could not be read".to_owned()),
    ),
  };
  client::remove_credential(server)?;
  let credential = client::credential(server)?;
  let other_server = session_url != server.url;
  let mut data = json!({
    "server_url": server.url,
    "authentication": credential.source.as_str(),
    "server_session": server_session.as_str(),
  });
  if other_server {
    data["session_server_url"] = json!(session_url);
  }
  if json_output {
    output::print_json(&data)?;
    return Ok(0);
  }
  let message = match server_session {
    ServerSession::Revoked => {
      format!("Logged out from {session_url} and ended the session on the server.")
    }
    ServerSession::AlreadyEnded => {
      format!("Logged out from {session_url}. The session had already ended on the server.")
    }
    ServerSession::NotReached | ServerSession::Unreadable | ServerSession::NotApplicable => {
      format!("Logged out from {session_url}.")
    }
  };
  output::print_success(&message);
  if other_server {
    output::print_info(&format!(
      "The saved login was for {session_url}, not {}.",
      server.url
    ));
  }
  if let Some(problem) = problem {
    output::print_warning(&format!(
      "Couldn't end the session on the server: {problem}. It stays valid there until it expires."
    ));
  }
  if runner_token {
    output::print_info(RUNNER_TOKEN_STILL_WORKS);
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

/// Ask the server to end a saved CLI session. The local files go either way.
async fn end_server_session(
  server: &ResolvedServer,
  token: String,
) -> (ServerSession, Option<String>) {
  let api_client = match client::ApiClient::new(server, Some(token)) {
    Ok(api_client) => api_client,
    Err(error) => return (ServerSession::NotReached, Some(first_line(&error))),
  };
  let request = api_client.request(Method::POST, api::auth::LOGOUT, None);
  match tokio::time::timeout(LOGOUT_TIMEOUT, request).await {
    Ok(Ok(_)) => (ServerSession::Revoked, None),
    Ok(Err(error)) if client::is_authentication_error(&error) => {
      (ServerSession::AlreadyEnded, None)
    }
    Ok(Err(error)) => (ServerSession::NotReached, Some(first_line(&error))),
    Err(_) => (
      ServerSession::NotReached,
      Some(format!(
        "{} did not answer within {} seconds",
        server.url,
        LOGOUT_TIMEOUT.as_secs()
      )),
    ),
  }
}

/// Transport errors carry advice on later lines that doesn't fit a warning.
fn first_line(error: &anyhow::Error) -> String {
  let message = error.to_string();
  message
    .lines()
    .next()
    .unwrap_or_default()
    .trim_end_matches('.')
    .to_owned()
}
