use super::{output, prompt};
use crate::{
  cli::{
    client::{self, ApiClient, Credential, CredentialSource},
    local_config::{self, ClientConfig},
  },
  config::{ServerConfig, ServerOverrides},
  constants::{
    config::{DEFAULT_PUBLIC_URL, ENV_SERVER_URL},
    tokens::{ADMIN_SESSION_PREFIX, AGENT_TOKEN_PREFIX, RUNNER_TOKEN_PREFIX},
  },
  daemon::ManagedDaemonState,
};
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::{env, path::Path, time::Duration};

pub(super) async fn connect(
  value: &str,
  data_dir: Option<&Path>,
  json_output: bool,
) -> Result<()> {
  if env::var_os(ENV_SERVER_URL).is_some() {
    bail!(
      "DOPBASE_URL is set and would override the saved server. Unset DOPBASE_URL before changing the active server"
    );
  }
  let current = local_config::resolve(None, data_dir)?;
  let target = if value == "local" {
    DEFAULT_PUBLIC_URL.to_owned()
  } else {
    local_config::normalize_connect_target(value)?
  };
  if target.starts_with("http://") {
    output::print_warning(&insecure_transport_warning(&target));
  }
  let prospective = local_config::ResolvedServer {
    url: target.clone(),
    source: local_config::ServerSource::Argument,
    config_path: current.config_path.clone(),
    config: current.config.clone(),
  };
  let health = ApiClient::new(&prospective, None)?.health().await?;
  if health.get("product").and_then(Value::as_str) != Some("dopbase")
    || health.get("apiVersion").and_then(Value::as_str) != Some("v1")
  {
    bail!("the endpoint is not a compatible Dopbase v1 server");
  }

  if current.url == target {
    let config = ClientConfig {
      version: 1,
      server_url: (value != "local").then_some(target.clone()),
      default_environment: current.config.default_environment.clone(),
    };
    local_config::write(&current.config_path, &config)?;
    print_connection_result(
      json_output,
      &current.url,
      &target,
      false,
      false,
      false,
      false,
    );
    return Ok(());
  }

  let server_config = ServerConfig::load(&ServerOverrides {
    data_dir: data_dir.map(Path::to_path_buf),
    ..ServerOverrides::default()
  })?;
  let daemon_state = crate::daemon::inspect(&server_config.data_dir)?;
  let switching_away_from_background_server = match &daemon_state {
    ManagedDaemonState::Running(pid_file) => pid_file
      .resolved_public_url()
      .and_then(|url| local_config::normalize(&url).ok())
      .is_some_and(|url| url == current.url),
    ManagedDaemonState::Absent | ManagedDaemonState::Stale => false,
  };
  let foreground_running = match &daemon_state {
    ManagedDaemonState::Running(_) => false,
    ManagedDaemonState::Absent | ManagedDaemonState::Stale => {
      crate::server::InstanceLock::is_held(&server_config.database_url)?
    }
  };
  if foreground_running {
    bail!(
      "A foreground Dopbase server is running for {}. Stop it with Ctrl+C, then run `dopbase client connect` again",
      server_config.data_dir.display()
    );
  }

  confirm_server_switch(&current.url, &target, switching_away_from_background_server).await?;

  let background_server_stopped = if switching_away_from_background_server {
    crate::daemon::stop_managed(Some(&server_config.data_dir), Duration::from_secs(10)).await?;
    true
  } else {
    if matches!(&daemon_state, ManagedDaemonState::Stale) {
      crate::daemon::remove_pid_file(&crate::daemon::pid_file_path(&server_config.data_dir))?;
    }
    false
  };

  let session_removed = client::remove_credential(&current)?;
  let default_environment_cleared = current.config.default_environment.is_some();
  let config = ClientConfig {
    version: 1,
    server_url: (value != "local").then_some(target.clone()),
    default_environment: None,
  };
  local_config::write(&current.config_path, &config)?;
  print_connection_result(
    json_output,
    &current.url,
    &target,
    true,
    background_server_stopped,
    session_removed,
    default_environment_cleared,
  );
  Ok(())
}

pub fn insecure_transport_warning(server_url: &str) -> String {
  format!(
    "Plain HTTP does not encrypt traffic to {server_url}. Credentials and secrets could be exposed. Use HTTPS whenever possible."
  )
}

async fn confirm_server_switch(
  current: &str,
  target: &str,
  background_server_running: bool,
) -> Result<()> {
  eprintln!("Change active Dopbase server?\nCurrent: {current}\nNew:     {target}\n\nThis will:");
  if background_server_running {
    eprintln!("- stop the managed background server");
  }
  eprintln!("- delete the saved CLI session and session key");
  eprintln!("- clear the saved default environment");
  prompt::confirm_with_cancel("Continue?", false, client::CliCancelled::ServerSwitch)
}

pub fn server_switch_confirmed(answer: &str) -> bool {
  matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

fn print_connection_result(
  json_output: bool,
  previous_server_url: &str,
  server_url: &str,
  changed: bool,
  background_server_stopped: bool,
  session_removed: bool,
  default_environment_cleared: bool,
) {
  let value = json!({
    "server_url": server_url,
    "previous_server_url": previous_server_url,
    "connected": true,
    "changed": changed,
    "background_server_stopped": background_server_stopped,
    "session_removed": session_removed,
    "default_environment_cleared": default_environment_cleared,
  });
  if json_output {
    output::print_json(&value).expect("serializing a JSON value cannot fail");
  } else if changed {
    output::print_success(&format!("Connected to {server_url}."));
    output::print_fields(&[
      ("Previous server:", previous_server_url.to_owned()),
      (
        "Background server stopped:",
        yes_no(background_server_stopped).into(),
      ),
      ("CLI session removed:", yes_no(session_removed).into()),
      (
        "Default environment cleared:",
        yes_no(default_environment_cleared).into(),
      ),
    ]);
    output::print_text("Run `dopbase login` to authenticate with the new server.");
  } else {
    output::print_text(&format!("Already connected to {server_url}."));
  }
}

fn yes_no(value: bool) -> &'static str {
  if value { "yes" } else { "no" }
}

pub(super) async fn show_status(
  argument: Option<&str>,
  data_dir: Option<&Path>,
  json_output: bool,
) -> Result<()> {
  let server = local_config::resolve(argument, data_dir)?;
  let credential = client::credential(&server)?;
  let connected = server_is_connected(&server).await;
  let value = status_document(&server, &credential, connected);
  if json_output {
    output::print_json(&value)?;
  } else {
    let identity = credential_identity(&credential);
    let email = match (&credential.email, credential.source, identity) {
      (Some(email), _, _) => email.as_str(),
      (None, CredentialSource::EncryptedSession, "admin") => {
        "unknown (run dopbase login again to refresh)"
      }
      _ => "none",
    };
    let environment = server.default_environment().map_or_else(
      || "none (set with `dopbase env default <ENVIRONMENT_REF>`)".to_owned(),
      |id| format!("{id} (default)"),
    );
    let server_status = if connected {
      "connected (live)"
    } else {
      "offline (cache)"
    };
    output::print_fields(&[
      ("Config file:", server.config_path.display().to_string()),
      ("Server:", server.url.clone()),
      ("Server status:", server_status.into()),
      ("Server source:", server.source.as_str().into()),
      ("Authentication:", credential.source.as_str().into()),
      ("Identity:", human_identity(identity).into()),
      ("Email:", email.into()),
      ("Environment:", environment),
    ]);
  }
  Ok(())
}

async fn server_is_connected(server: &local_config::ResolvedServer) -> bool {
  let Ok(api) = client::ApiClient::new(server, None) else {
    return false;
  };
  matches!(
    tokio::time::timeout(Duration::from_secs(3), api.health()).await,
    Ok(Ok(_))
  )
}

pub(super) async fn ensure_server_is_connected(
  server: &local_config::ResolvedServer,
  operation: &str,
) -> Result<()> {
  if !server_is_connected(server).await {
    bail!(
      "Cannot perform {operation}: Dopbase server at {} is not connected or offline (live status required).\n\
       Check that the server is running with `dopbase server start` and verify the endpoint with `dopbase client status`.",
      server.url
    );
  }
  Ok(())
}

pub fn status_document(
  server: &local_config::ResolvedServer,
  credential: &Credential,
  connected: bool,
) -> Value {
  json!({
    "config_file": server.config_path,
    "server_url": server.url,
    "server_source": server.source.as_str(),
    "authentication": credential.source.as_str(),
    "identity": credential_identity(credential),
    "email": credential.email,
    "environment": server.default_environment(),
    "server_status": if connected { "connected" } else { "offline" },
    "status_source": if connected { "live" } else { "cache" },
  })
}

fn credential_identity(credential: &Credential) -> &'static str {
  match credential.source {
    CredentialSource::Argument | CredentialSource::Environment => match credential.token.as_deref()
    {
      Some(token) if token.starts_with(ADMIN_SESSION_PREFIX) => "human",
      Some(token) if token.starts_with(RUNNER_TOKEN_PREFIX) => "runner",
      Some(token) if token.starts_with(AGENT_TOKEN_PREFIX) => "ai_agent",
      _ => "unknown",
    },
    CredentialSource::EncryptedSession => match credential.token.as_deref() {
      Some(token) if token.starts_with(RUNNER_TOKEN_PREFIX) => "runner",
      Some(token) if token.starts_with(AGENT_TOKEN_PREFIX) => "ai_agent",
      _ => "admin",
    },
    CredentialSource::None => "none",
  }
}

fn human_identity(identity: &str) -> &str {
  if identity == "ai_agent" {
    "AI agent"
  } else {
    identity
  }
}
