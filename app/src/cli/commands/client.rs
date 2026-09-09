use super::output;
use crate::{
  cli::{
    client::{self, ApiClient, Credential, CredentialSource},
    local_config::{self, ClientConfig},
  },
  config::{ServerConfig, ServerOverrides},
  constants::config::{DEFAULT_PUBLIC_URL, ENV_SERVER_URL},
  daemon::ManagedDaemonState,
};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::{
  env,
  io::{self, IsTerminal, Write},
  path::Path,
  time::Duration,
};

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
    local_config::normalize(value)?
  };
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

async fn confirm_server_switch(
  current: &str,
  target: &str,
  background_server_running: bool,
) -> Result<()> {
  if !io::stdin().is_terminal() {
    bail!("interactive confirmation is required to change the active Dopbase server");
  }
  eprintln!("Change active Dopbase server?\nCurrent: {current}\nNew:     {target}\n\nThis will:");
  if background_server_running {
    eprintln!("- stop the managed background server");
  }
  eprintln!("- delete the saved CLI session and session key");
  eprintln!("- clear the saved default environment");
  eprint!("Continue? [y/N] ");
  io::stderr().flush()?;

  let mut prompt = tokio::task::spawn_blocking(read_server_switch_confirmation);
  let confirmed = tokio::select! {
    result = &mut prompt => result.context("server switch confirmation task failed")??,
    signal = tokio::signal::ctrl_c() => {
      signal?;
      let _ = tokio::time::timeout(Duration::from_millis(250), &mut prompt).await;
      return Err(client::CliCancelled::ServerSwitch.into());
    }
  };
  if !confirmed {
    return Err(client::CliCancelled::ServerSwitch.into());
  }
  Ok(())
}

fn read_server_switch_confirmation() -> Result<bool> {
  let mut answer = String::new();
  match io::stdin().read_line(&mut answer) {
    Ok(_) => Ok(server_switch_confirmed(&answer)),
    Err(error) if error.kind() == io::ErrorKind::Interrupted => {
      Err(client::CliCancelled::ServerSwitch.into())
    }
    Err(error) => Err(error.into()),
  }
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
    output::print_value(true, &value);
  } else if changed {
    println!(
      "Connected to {server_url}.\nPrevious server: {previous_server_url}\nBackground server stopped: {}\nCLI session removed: {}\nDefault environment cleared: {}\nRun `dopbase login` to authenticate with the new server.",
      yes_no(background_server_stopped),
      yes_no(session_removed),
      yes_no(default_environment_cleared),
    );
  } else {
    println!("Already connected to {server_url}.");
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
    println!("{}", serde_json::to_string_pretty(&value)?);
  } else {
    let email = match (&credential.email, credential.source) {
      (Some(email), _) => email.as_str(),
      (None, CredentialSource::EncryptedSession) => "unknown (run dopbase login again to refresh)",
      _ => "none",
    };
    let environment = server.default_environment().map_or_else(
      || "none (set with `dopbase env default <project/environment>`)".to_owned(),
      |id| format!("{id} (default)"),
    );
    let server_status = if connected {
      "connected (live)"
    } else {
      "offline (cache)"
    };
    println!(
      "Config file:     {}\nServer:          {}\nServer status:   {}\nServer source:   {}\nAuthentication:  {}\nIdentity:        {}\nEmail:           {}\nEnvironment:     {}",
      server.config_path.display(),
      server.url,
      server_status,
      server.source.as_str(),
      credential.source.as_str(),
      credential_identity(&credential),
      email,
      environment,
    );
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
    CredentialSource::Environment => "runner",
    CredentialSource::EncryptedSession => "admin",
    CredentialSource::None => "none",
  }
}
