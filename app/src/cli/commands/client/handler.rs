use crate::cli::{output, prompt};
use crate::{
  cli::{
    client::{self, ApiClient, Credential, CredentialSource},
    local_config::{self, ClientConfig},
  },
  config::{ServerConfig, ServerOverrides},
  constants::{
    api as api_paths,
    config::{DEFAULT_PUBLIC_URL, ENV_SERVER_URL},
    tokens::{
      ADMIN_SESSION_PREFIX, AGENT_TOKEN_PREFIX, PERSONAL_TOKEN_PREFIX, RUNNER_TOKEN_PREFIX,
    },
  },
  daemon::ManagedDaemonState,
};
use anyhow::{Result, bail};
use reqwest::Method;
use serde_json::{Value, json};
use std::{env, path::Path, time::Duration};

pub(crate) async fn connect(
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
  // The instance this machine runs from the same data directory. Its URL is
  // known without asking the network, so connecting to it does not need it
  // to be up.
  let server_config = ServerConfig::load(&ServerOverrides {
    data_dir: data_dir.map(Path::to_path_buf),
    ..ServerOverrides::default()
  })?;
  let default_url = local_config::normalize(DEFAULT_PUBLIC_URL)?;
  let target = if value == "local" {
    // With nothing saved the CLI already uses the default URL. When that
    // reaches this instance, `local` is where the CLI already is, even if
    // the public URL is spelled differently, so keep it instead of switching.
    if matches!(current.source, local_config::ServerSource::Default)
      && crate::cli::commands::connection_is_local(
        &current.url,
        &server_config.public_url,
        &server_config.bind_address,
      )
    {
      current.url.clone()
    } else {
      local_config::normalize(&server_config.public_url).unwrap_or_else(|_| default_url.clone())
    }
  } else {
    local_config::normalize_connect_target(value)?
  };
  let own_instance = value == "local" || is_own_instance(&target, &server_config);
  if target.starts_with("http://") {
    output::print_warning(&insecure_transport_warning(&target));
  }
  let prospective = local_config::ResolvedServer {
    url: target.clone(),
    source: local_config::ServerSource::Argument,
    config_path: current.config_path.clone(),
    config: current.config.clone(),
  };
  // Nothing listening at our own instance just means it is not started yet.
  // Anything that does answer must still be a compatible Dopbase server.
  let reachable = match ApiClient::new(&prospective, None)?.health().await {
    Ok(health) => {
      if health.get("product").and_then(Value::as_str) != Some("dopbase")
        || health.get("apiVersion").and_then(Value::as_str) != Some("v1")
      {
        bail!("the endpoint is not a compatible Dopbase v1 server");
      }
      true
    }
    Err(error) if own_instance && client::is_unreachable_error(&error) => false,
    Err(error) => return Err(error),
  };
  // `local` on the default URL keeps the implicit default instead of saving it.
  let saved_url = (value != "local" || target != default_url).then(|| target.clone());

  if current.url == target {
    let config = ClientConfig {
      version: 1,
      server_url: saved_url,
      default_environment: current.config.default_environment.clone(),
    };
    local_config::write(&current.config_path, &config)?;
    print_connection_result(
      json_output,
      &ConnectionResult {
        previous_server_url: &current.url,
        server_url: &target,
        reachable,
        changed: false,
        background_server_stopped: false,
        session_removed: false,
        default_environment_cleared: false,
      },
    );
    return Ok(());
  }

  let daemon_state = crate::daemon::inspect(&server_config.data_dir)?;
  let switching_away_from_background_server = match &daemon_state {
    ManagedDaemonState::Running(pid_file) => pid_file
      .resolved_public_url()
      .and_then(|url| local_config::normalize(&url).ok())
      .is_some_and(|url| url == current.url),
    ManagedDaemonState::Absent | ManagedDaemonState::Stale => false,
  };
  // Only a foreground server we are leaving blocks the switch. Connecting to
  // it, or moving between two other servers, is fine.
  let switching_away_from_local_server = crate::cli::commands::connection_is_local(
    &current.url,
    &server_config.public_url,
    &server_config.bind_address,
  );
  let foreground_running = match &daemon_state {
    ManagedDaemonState::Running(_) => false,
    ManagedDaemonState::Absent | ManagedDaemonState::Stale => {
      switching_away_from_local_server
        && crate::server::InstanceLock::is_held(&server_config.database_url)?
    }
  };
  if foreground_running {
    bail!(
      "{} is a foreground Dopbase server running from {}. Stop it with Ctrl+C before you switch away from it, then run `dopbase client connect` again",
      current.url,
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
    server_url: saved_url,
    default_environment: None,
  };
  local_config::write(&current.config_path, &config)?;
  print_connection_result(
    json_output,
    &ConnectionResult {
      previous_server_url: &current.url,
      server_url: &target,
      reachable,
      changed: true,
      background_server_stopped,
      session_removed,
      default_environment_cleared,
    },
  );
  Ok(())
}

/// Whether `target` is this machine's own instance: the data directory holds
/// a database, and the URL is its public URL or a loopback address on its
/// port.
fn is_own_instance(
  target: &str,
  server_config: &ServerConfig,
) -> bool {
  let has_database =
    crate::config::database_path(&server_config.database_url).is_ok_and(|path| path.is_file());
  has_database
    && crate::cli::commands::connection_is_local(
      target,
      &server_config.public_url,
      &server_config.bind_address,
    )
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

struct ConnectionResult<'a> {
  previous_server_url: &'a str,
  server_url: &'a str,
  /// False when nothing answered at our own instance, which was accepted
  /// because it simply has not been started.
  reachable: bool,
  changed: bool,
  background_server_stopped: bool,
  session_removed: bool,
  default_environment_cleared: bool,
}

fn print_connection_result(
  json_output: bool,
  result: &ConnectionResult<'_>,
) {
  let server_url = result.server_url;
  let value = json!({
    "server_url": server_url,
    "previous_server_url": result.previous_server_url,
    "connected": true,
    "server_reachable": result.reachable,
    "changed": result.changed,
    "background_server_stopped": result.background_server_stopped,
    "session_removed": result.session_removed,
    "default_environment_cleared": result.default_environment_cleared,
  });
  if json_output {
    output::print_json(&value).expect("serializing a JSON value cannot fail");
    return;
  }
  if result.changed {
    output::print_success(&format!("Connected to {server_url}."));
    output::print_fields(&[
      ("Previous server:", result.previous_server_url.to_owned()),
      (
        "Background server stopped:",
        yes_no(result.background_server_stopped).into(),
      ),
      (
        "CLI session removed:",
        yes_no(result.session_removed).into(),
      ),
      (
        "Default environment cleared:",
        yes_no(result.default_environment_cleared).into(),
      ),
    ]);
  } else {
    output::print_text(&format!("Already connected to {server_url}."));
  }
  if !result.reachable {
    output::print_text(&format!(
      "No server is running at {server_url} yet. Start it with `dopbase server start`, then run `dopbase login`."
    ));
  } else if result.changed {
    output::print_text("Run `dopbase login` to authenticate with the new server.");
  }
}

fn yes_no(value: bool) -> &'static str {
  if value { "yes" } else { "no" }
}

pub(crate) async fn show_status(
  argument: Option<&str>,
  data_dir: Option<&Path>,
  json_output: bool,
) -> Result<()> {
  let server = local_config::resolve(argument, data_dir)?;
  let credential = client::credential(&server)?;
  let connected = server_is_connected(&server).await;
  let mut identity = credential_identity(&credential);
  let role = if connected && has_account(identity) {
    Some(fetch_role(&server, &credential).await)
  } else {
    None
  };
  let role_name = match &role {
    Some(RoleLookup::Found { name, .. }) => Some(name.as_str()),
    _ => None,
  };
  // AI agent tokens now share the personal token prefix, so the server's
  // answer is what tells an agent apart: only an agent has an owner.
  let owner = match &role {
    Some(RoleLookup::Found { owner, .. }) => owner.clone(),
    _ => None,
  };
  if owner.is_some() && identity == "personal_token" {
    identity = "ai_agent";
  }
  let mut value = status_document(&server, &credential, connected, role_name);
  value["identity"] = json!(identity);
  if let Some(owner) = &owner {
    value["owner"] = json!(owner);
  }
  if json_output {
    output::print_json(&value)?;
  } else {
    let account = match &role {
      Some(RoleLookup::Found { account, .. }) => account.as_deref(),
      _ => None,
    };
    let email = match (&credential.email, credential.source, identity) {
      (Some(email), _, _) => email.as_str(),
      // An agent has a name, not an email; its name shows under Agent.
      (None, _, "ai_agent") => "none",
      (None, _, "personal_token") => account.unwrap_or("none"),
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
    let mut fields = vec![
      ("Config file:", server.config_path.display().to_string()),
      ("Server:", server.url.clone()),
      ("Server status:", server_status.into()),
      ("Server source:", server.source.as_str().into()),
      (
        "Authentication:",
        human_authentication(credential.source, identity).into(),
      ),
      ("Role:", human_role(identity, connected, role.as_ref())),
      ("Email:", email.into()),
    ];
    if let Some(owner) = &owner {
      fields.push(("Agent:", account.unwrap_or("unknown").to_owned()));
      fields.push(("Owner:", owner.clone()));
    }
    fields.push(("Environment:", environment));
    output::print_fields(&fields);
  }
  Ok(())
}

enum RoleLookup {
  /// The role name, the account's email (an AI agent's name), and an AI
  /// agent's owner.
  Found {
    name: String,
    account: Option<String>,
    owner: Option<String>,
  },
  SessionExpired,
  Unavailable,
}

/// Runner tokens and missing or unrecognized credentials have no account
/// behind them to ask about.
fn has_account(identity: &str) -> bool {
  matches!(identity, "admin" | "human" | "personal_token" | "ai_agent")
}

/// Asks the server for the role of the account behind the credential, such
/// as `Member` or `Root`. The token prefix alone can't tell.
async fn fetch_role(
  server: &local_config::ResolvedServer,
  credential: &Credential,
) -> RoleLookup {
  let Ok(api) = client::ApiClient::new(server, credential.token.clone()) else {
    return RoleLookup::Unavailable;
  };
  let path = api_paths::users::access("me");
  let request = api.request(Method::GET, &path, None);
  match tokio::time::timeout(Duration::from_secs(3), request).await {
    Ok(Ok(data)) => match data.get("roleName").and_then(Value::as_str) {
      Some(name) if !name.is_empty() => {
        let text = |key: &str| data.get(key).and_then(Value::as_str).map(str::to_owned);
        RoleLookup::Found {
          name: name.to_owned(),
          account: text("email"),
          owner: text("ownerEmail"),
        }
      }
      _ => RoleLookup::Unavailable,
    },
    Ok(Err(error)) if client::is_authentication_error(&error) => RoleLookup::SessionExpired,
    _ => RoleLookup::Unavailable,
  }
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

pub(crate) async fn ensure_server_is_connected(
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
  role: Option<&str>,
) -> Value {
  json!({
    "config_file": server.config_path,
    "server_url": server.url,
    "server_source": server.source.as_str(),
    "authentication": credential.source.as_str(),
    "identity": credential_identity(credential),
    "role": role,
    "email": credential.email,
    "environment": server.default_environment(),
    "server_status": if connected { "connected" } else { "offline" },
    "status_source": if connected { "live" } else { "cache" },
  })
}

fn credential_identity(credential: &Credential) -> &'static str {
  match credential.source {
    CredentialSource::Environment => match credential.token.as_deref() {
      Some(token) if token.starts_with(ADMIN_SESSION_PREFIX) => "human",
      Some(token) if token.starts_with(RUNNER_TOKEN_PREFIX) => "runner",
      Some(token) if token.starts_with(PERSONAL_TOKEN_PREFIX) => "personal_token",
      Some(token) if token.starts_with(AGENT_TOKEN_PREFIX) => "ai_agent",
      _ => "unknown",
    },
    CredentialSource::EncryptedSession => match credential.token.as_deref() {
      Some(token) if token.starts_with(RUNNER_TOKEN_PREFIX) => "runner",
      Some(token) if token.starts_with(PERSONAL_TOKEN_PREFIX) => "personal_token",
      Some(token) if token.starts_with(AGENT_TOKEN_PREFIX) => "ai_agent",
      _ => "admin",
    },
    CredentialSource::None => "none",
  }
}

/// How the CLI signed in, and where the credential came from.
fn human_authentication(
  source: CredentialSource,
  identity: &str,
) -> &'static str {
  match (source, identity) {
    (CredentialSource::EncryptedSession, "admin") => "dopbase login (saved session)",
    (CredentialSource::EncryptedSession, "runner") => "runner token (saved)",
    (CredentialSource::EncryptedSession, "personal_token") => "personal token (saved)",
    (CredentialSource::EncryptedSession, "ai_agent") => "AI agent token (saved)",
    (CredentialSource::Environment, "human") => "session token (DOPBASE_TOKEN)",
    (CredentialSource::Environment, "runner") => "runner token (DOPBASE_TOKEN)",
    (CredentialSource::Environment, "personal_token") => "personal token (DOPBASE_TOKEN)",
    (CredentialSource::Environment, "ai_agent") => "AI agent token (DOPBASE_TOKEN)",
    (CredentialSource::Environment, _) => "unrecognized token (DOPBASE_TOKEN)",
    _ => "none (run dopbase login)",
  }
}

fn human_role(
  identity: &str,
  connected: bool,
  role: Option<&RoleLookup>,
) -> String {
  match (identity, role) {
    ("none", _) => "none".into(),
    ("runner", _) => "none (runner tokens act for one environment, not a person)".into(),
    ("personal_token" | "ai_agent", Some(RoleLookup::Found { name, .. })) => {
      format!("{name} (the token may reach less; run dopbase access)")
    }
    (_, Some(RoleLookup::Found { name, .. })) => name.clone(),
    (_, Some(RoleLookup::SessionExpired)) => "unknown (run dopbase login again)".into(),
    _ if !connected => "unknown (server offline)".into(),
    _ => "unknown".into(),
  }
}
