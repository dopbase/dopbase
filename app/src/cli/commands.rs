use super::{
  args::*,
  client::{self, ApiClient, Credential, CredentialSource},
  dotenv,
  local_config::{self, ClientConfig},
  runtime_cache::{self, RuntimeSource},
};
use crate::{
  config::{ServerConfig, ServerOverrides, ensure_data_dir},
  constants::config::{DEFAULT_PUBLIC_URL, ENV_SERVER_URL},
  daemon::ManagedDaemonState,
  models::SecretInput,
};
use anyhow::{Context, Result, bail};
use reqwest::Method;
use serde_json::{Value, json};
use std::{
  env,
  io::{self, IsTerminal, Read, Write},
  path::{Path, PathBuf},
  time::Duration,
};

pub async fn execute(cli: Cli) -> Result<i32> {
  let server_argument = cli.server.clone();
  let data_dir = cli.data_dir.clone();
  let json_output = cli.json;
  match cli.command {
    Command::Serve(args) => {
      let docs = args.docs();
      let background = args.background;
      let supervised = args.supervised;
      if background {
        let config = ServerConfig::load(&ServerOverrides {
          data_dir: data_dir.clone(),
          docs,
          background,
          supervised,
          config_path: args.config.clone(),
          bind_address: args.bind_address.clone(),
          public_url: args.public_url.clone(),
          port: args.port,
          host: args.host.clone(),
          database_url: args.database_url.clone(),
          shutdown_grace_seconds: args.shutdown_grace_seconds,
          master_key_path: args.master_key_file.clone(),
        })?;
        let flags = serve_flags(&args, &config.data_dir);
        return crate::daemon::start(config, &flags, json_output).await;
      }
      let config = ServerConfig::load(&ServerOverrides {
        data_dir,
        docs,
        background,
        supervised,
        config_path: args.config,
        bind_address: args.bind_address,
        public_url: args.public_url,
        port: args.port,
        host: args.host,
        database_url: args.database_url,
        shutdown_grace_seconds: args.shutdown_grace_seconds,
        master_key_path: args.master_key_file,
      })?;
      let ready = supervised.then(crate::daemon::Ready::attached).flatten();
      let result = crate::server::serve_with_ready(config, ready.as_ref()).await;
      if let (Some(ready), Err(error)) = (&ready, &result) {
        ready.fail(&format!("{error:#}"));
      }
      result?;
      Ok(0)
    }
    Command::Client {
      command: ClientCommand::Connect { server_url },
    } => {
      if server_argument.is_some() {
        bail!(
          "--server cannot be used with `dopbase client connect`; pass the destination as the positional server URL"
        );
      }
      connect(&server_url, data_dir.as_deref(), json_output).await?;
      Ok(0)
    }
    Command::Login => {
      let server = local_config::resolve(server_argument.as_deref(), data_dir.as_deref())?;
      let _ = client::login(&server, true).await?;
      print_value(
        json_output,
        &json!({"server_url":server.url,"authentication":"encrypted_session"}),
      );
      Ok(0)
    }
    Command::Logout => {
      let server = local_config::resolve(server_argument.as_deref(), data_dir.as_deref())?;
      client::remove_credential(&server)?;
      let credential = client::credential(&server)?;
      print_value(
        json_output,
        &json!({"server_url":server.url,"authentication":credential.source.as_str()}),
      );
      Ok(0)
    }
    Command::Status => {
      show_status(server_argument.as_deref(), data_dir.as_deref(), json_output).await?;
      Ok(0)
    }
    Command::Admin { command } => admin(command, data_dir).await,
    Command::Update => super::update::run(json_output).await,
    Command::Stop { timeout } => {
      crate::daemon::stop(
        data_dir.as_deref(),
        Duration::from_secs(timeout),
        json_output,
      )
      .await
    }
    command => {
      let server = local_config::resolve(server_argument.as_deref(), data_dir.as_deref())?;
      execute_client(command, &server, json_output).await
    }
  }
}

/// Build the argv for the detached background server: the same user flags
/// plus the resolved data directory and the internal `--supervised` marker
/// (`--background` itself must not reappear, or the child would daemonize again).
fn serve_flags(
  args: &ServeArgs,
  data_dir: &Path,
) -> Vec<String> {
  let mut flags = vec!["serve".to_string()];
  if let Some(value) = &args.config {
    flags.push("--config".into());
    flags.push(value.to_string_lossy().into_owned());
  }
  if let Some(value) = &args.bind_address {
    flags.push("--bind-address".into());
    flags.push(value.clone());
  }
  if let Some(value) = args.port {
    flags.push("--port".into());
    flags.push(value.to_string());
  }
  if let Some(value) = &args.host {
    flags.push("--host".into());
    flags.push(value.clone());
  }
  if let Some(value) = &args.public_url {
    flags.push("--public-url".into());
    flags.push(value.clone());
  }
  if let Some(value) = &args.database_url {
    flags.push("--database-url".into());
    flags.push(value.clone());
  }
  if let Some(value) = args.shutdown_grace_seconds {
    flags.push("--shutdown-grace-seconds".into());
    flags.push(value.to_string());
  }
  match args.docs() {
    Some(true) => flags.push("--docs".into()),
    Some(false) => flags.push("--no-docs".into()),
    None => {}
  }
  if let Some(value) = &args.master_key_file {
    flags.push("--master-key-file".into());
    flags.push(value.to_string_lossy().into_owned());
  }
  flags.push("--data-dir".into());
  flags.push(data_dir.to_string_lossy().into_owned());
  flags.push("--supervised".into());
  flags
}

async fn connect(
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
    print_value(true, &value);
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

async fn show_status(
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

async fn execute_client(
  command: Command,
  server: &local_config::ResolvedServer,
  json_output: bool,
) -> Result<i32> {
  match command {
    Command::Init {
      project,
      environment,
      from,
    } => {
      let api = client::human_client(server).await?;
      let entries = dotenv::parse_file(&from)?;
      let data = api
        .request(
          Method::POST,
          "/api/v1/projects/init",
          Some(json!({"projectName":project,"environmentName":environment,"entries":entries})),
        )
        .await?;
      print_value(json_output, &data);
      Ok(0)
    }
    Command::Project { command } => project(command, server, json_output).await,
    Command::Env { command } => environment(command, server, json_output).await,
    Command::Secret { command } => secret(command, server, json_output).await,
    Command::Import {
      environment,
      path,
      dry_run,
      replace,
      yes,
    } => {
      import(
        server,
        &environment,
        &path,
        dry_run,
        replace,
        yes,
        json_output,
      )
      .await
    }
    Command::Export {
      environment,
      output,
      stdout,
      force,
    } => export(server, &environment, output, stdout, force, json_output).await,
    Command::Token { command } => token(command, server, json_output).await,
    Command::Run {
      environment,
      command,
    } => run(server, environment, command).await,
    _ => bail!("unsupported command"),
  }
}

async fn project(
  command: ProjectCommand,
  server: &local_config::ResolvedServer,
  json_output: bool,
) -> Result<i32> {
  let api = client::human_client(server).await?;
  let data = match command {
    ProjectCommand::Create { name } => {
      api
        .request(Method::POST, "/api/v1/projects", Some(json!({"name":name})))
        .await?
    }
    ProjectCommand::List => api.request(Method::GET, "/api/v1/projects", None).await?,
    ProjectCommand::Show { project } => {
      api
        .request(Method::GET, &format!("/api/v1/projects/{project}"), None)
        .await?
    }
    ProjectCommand::Rename { project, new_name } => {
      api
        .request(
          Method::PATCH,
          &format!("/api/v1/projects/{project}"),
          Some(json!({"name":new_name})),
        )
        .await?
    }
    ProjectCommand::Delete { project, yes } => {
      let detail = api
        .request(Method::GET, &format!("/api/v1/projects/{project}"), None)
        .await?;
      let name = detail
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(&project);
      let environments = api
        .request(
          Method::GET,
          &format!(
            "/api/v1/environments?project={}",
            client::encode_query(&project)
          ),
          None,
        )
        .await?;
      let count = environments.as_array().map_or(0, Vec::len);
      confirm(
        &format!("Delete project {name} and its {count} environment(s)?"),
        yes,
      )?;
      api
        .request(Method::DELETE, &format!("/api/v1/projects/{project}"), None)
        .await?
    }
  };
  print_value(json_output, &data);
  Ok(0)
}

async fn resolve_environment(
  api: &ApiClient,
  reference: &str,
) -> Result<Value> {
  api
    .request(
      Method::GET,
      &format!(
        "/api/v1/environments/resolve?reference={}",
        client::encode_query(reference)
      ),
      None,
    )
    .await
}
async fn environment(
  command: EnvCommand,
  server: &local_config::ResolvedServer,
  json_output: bool,
) -> Result<i32> {
  let command = match command {
    EnvCommand::Default { environment, clear } => {
      if clear {
        let cleared = local_config::clear_default_environment(server)?;
        print_value(
          json_output,
          &json!({"server_url":server.url,"environment":Value::Null,"cleared":cleared}),
        );
        return Ok(0);
      }
      let reference = environment.context("default environment is required")?;
      let api = client::human_client(server).await?;
      let environment = resolve_environment(&api, &reference).await?;
      let id = env_id(&environment)?;
      local_config::save_default_environment(server, id)?;
      print_value(
        json_output,
        &json!({"server_url":server.url,"environment":environment,"default":true}),
      );
      return Ok(0);
    }
    command => command,
  };
  let api = client::human_client(server).await?;
  let data = match command {
    EnvCommand::Default { .. } => unreachable!(),
    EnvCommand::Create { project, name } => {
      api
        .request(
          Method::POST,
          &format!("/api/v1/projects/{project}/environments"),
          Some(json!({"name":name})),
        )
        .await?
    }
    EnvCommand::List { project } => {
      let path = project.map_or_else(
        || "/api/v1/environments".into(),
        |value| {
          format!(
            "/api/v1/environments?project={}",
            client::encode_query(&value)
          )
        },
      );
      api.request(Method::GET, &path, None).await?
    }
    EnvCommand::Show { environment } => {
      let env = resolve_environment(&api, &environment).await?;
      let id = env_id(&env)?;
      api
        .request(Method::GET, &format!("/api/v1/environments/{id}"), None)
        .await?
    }
    EnvCommand::Rename {
      environment,
      new_name,
    } => {
      let env = resolve_environment(&api, &environment).await?;
      let id = env_id(&env)?;
      api
        .request(
          Method::PATCH,
          &format!("/api/v1/environments/{id}"),
          Some(json!({"name":new_name})),
        )
        .await?
    }
    EnvCommand::Delete { environment, yes } => {
      let env = resolve_environment(&api, &environment).await?;
      let id = env_id(&env)?;
      let secrets = api
        .request(
          Method::GET,
          &format!("/api/v1/environments/{id}/secrets"),
          None,
        )
        .await?;
      let tokens = api
        .request(
          Method::GET,
          &format!("/api/v1/environments/{id}/tokens"),
          None,
        )
        .await?;
      confirm(
        &format!(
          "Delete environment {environment}, {} secret(s), and {} token(s)?",
          array_len(&secrets),
          array_len(&tokens)
        ),
        yes,
      )?;
      api
        .request(Method::DELETE, &format!("/api/v1/environments/{id}"), None)
        .await?
    }
  };
  print_value(json_output, &data);
  Ok(0)
}

async fn secret(
  command: SecretCommand,
  server: &local_config::ResolvedServer,
  json_output: bool,
) -> Result<i32> {
  let api = if matches!(&command, SecretCommand::Get { reveal: true, .. }) {
    client::password_confirmed_human_client(server).await?
  } else {
    client::human_client(server).await?
  };
  let data = match command {
    SecretCommand::List { environment } => {
      let env = resolve_environment(&api, &environment).await?;
      api
        .request(
          Method::GET,
          &format!("/api/v1/environments/{}/secrets", env_id(&env)?),
          None,
        )
        .await?
    }
    SecretCommand::Set {
      environment,
      key,
      stdin,
    } => {
      let value = if stdin {
        let mut value = String::new();
        io::stdin().read_to_string(&mut value)?;
        value
      } else {
        if !io::stdin().is_terminal() {
          bail!("use --stdin when setting a secret non-interactively");
        }
        rpassword::prompt_password("Secret value: ")?
      };
      let env = resolve_environment(&api, &environment).await?;
      api
        .request(
          Method::PUT,
          &format!("/api/v1/environments/{}/secrets/{key}", env_id(&env)?),
          Some(json!({"value":value})),
        )
        .await?
    }
    SecretCommand::Get {
      environment,
      key,
      reveal,
    } => {
      let env = resolve_environment(&api, &environment).await?;
      let action = if reveal {
        format!(
          "/api/v1/environments/{}/secrets/{key}/reveal",
          env_id(&env)?
        )
      } else {
        format!("/api/v1/environments/{}/secrets/{key}", env_id(&env)?)
      };
      api
        .request(
          if reveal { Method::POST } else { Method::GET },
          &action,
          None,
        )
        .await?
    }
    SecretCommand::Delete {
      environment,
      key,
      yes,
    } => {
      confirm(&format!("Delete secret {key} from {environment}?"), yes)?;
      let env = resolve_environment(&api, &environment).await?;
      api
        .request(
          Method::DELETE,
          &format!("/api/v1/environments/{}/secrets/{key}", env_id(&env)?),
          None,
        )
        .await?
    }
  };
  print_value(json_output, &data);
  Ok(0)
}

async fn import(
  server: &local_config::ResolvedServer,
  reference: &str,
  path: &Path,
  dry_run: bool,
  replace: bool,
  yes: bool,
  json_output: bool,
) -> Result<i32> {
  let api = client::human_client(server).await?;
  let env = resolve_environment(&api, reference).await?;
  let id = env_id(&env)?;
  let entries = dotenv::parse_file(path)?;
  let mode = if replace { "replace" } else { "merge" };
  let endpoint = format!("/api/v1/environments/{id}/secrets/import");
  let mut expected_revision = None;
  if replace && !dry_run {
    let preview = api
      .request(
        Method::POST,
        &endpoint,
        Some(json!({"mode":mode,"dryRun":true,"entries":entries})),
      )
      .await?;
    expected_revision = preview
      .get("revision")
      .and_then(Value::as_str)
      .map(str::to_owned);
    let deleted = preview
      .get("deletedKeys")
      .and_then(Value::as_array)
      .cloned()
      .unwrap_or_default();
    if !deleted.is_empty() {
      eprintln!(
        "Replace will delete: {}",
        deleted
          .iter()
          .filter_map(Value::as_str)
          .collect::<Vec<_>>()
          .join(", ")
      );
      confirm("Apply this replacement?", yes)?;
    }
  }
  let data = api
    .request(
      Method::POST,
      &endpoint,
      Some(json!({"mode":mode,"dryRun":dry_run,"entries":entries,"expectedRevision":expected_revision})),
    )
    .await?;
  print_value(json_output, &data);
  Ok(0)
}
async fn export(
  server: &local_config::ResolvedServer,
  reference: &str,
  output: Option<PathBuf>,
  stdout: bool,
  force: bool,
  json_output: bool,
) -> Result<i32> {
  if output.is_none() && !stdout {
    bail!("export requires --output <FILE> or --stdout");
  }
  if stdout && json_output {
    bail!("--stdout and --json cannot be combined");
  }
  let api = client::password_confirmed_human_client(server).await?;
  let env = resolve_environment(&api, reference).await?;
  let data = api
    .request(
      Method::POST,
      &format!("/api/v1/environments/{}/secrets/export", env_id(&env)?),
      None,
    )
    .await?;
  let entries = parse_entries(&data)?;
  let rendered = dotenv::render(&entries);
  if stdout {
    print!("{rendered}");
  } else if let Some(path) = output {
    write_private(&path, rendered.as_bytes(), force)?;
    if json_output {
      print_value(true, &json!({"output":path,"secret_count":entries.len()}));
    } else {
      println!("Exported {} secret(s) to {}", entries.len(), path.display());
    }
  }
  Ok(0)
}

async fn token(
  command: TokenCommand,
  server: &local_config::ResolvedServer,
  json_output: bool,
) -> Result<i32> {
  let api = client::human_client(server).await?;
  let data = match command {
    TokenCommand::Create {
      environment,
      name,
      role,
    } => {
      let env = resolve_environment(&api, &environment).await?;
      api
        .request(
          Method::POST,
          &format!("/api/v1/environments/{}/tokens", env_id(&env)?),
          Some(json!({"name":name,"role":role})),
        )
        .await?
    }
    TokenCommand::List { environment } => {
      let env = resolve_environment(&api, &environment).await?;
      api
        .request(
          Method::GET,
          &format!("/api/v1/environments/{}/tokens", env_id(&env)?),
          None,
        )
        .await?
    }
    TokenCommand::Revoke { token_id } => {
      api
        .request(
          Method::POST,
          &format!("/api/v1/tokens/{token_id}/revoke"),
          None,
        )
        .await?
    }
  };
  print_value(json_output, &data);
  Ok(0)
}

async fn run(
  server: &local_config::ResolvedServer,
  environment: Option<String>,
  command: Vec<String>,
) -> Result<i32> {
  let selection = run_environment(
    environment,
    env::var("DOPBASE_ENV"),
    server.default_environment(),
  )?;
  let api = client::any_authenticated_client(server).await?;
  let loaded = runtime_cache::load(server, &api, &selection.reference).await;
  let loaded = match loaded {
    Err(error)
      if selection.source == RunEnvironmentSource::Default
        && error.to_string().contains("ENVIRONMENT_NOT_FOUND") =>
    {
      bail!(
        "Saved default environment {} is unavailable. Set a new one with `dopbase env default <project/environment>` or clear it with `dopbase env default --clear`.",
        selection.reference
      )
    }
    result => result?,
  };
  match &loaded.source {
    RuntimeSource::Live { cache_warning } => {
      if let Some(warning) = cache_warning {
        eprintln!("Dopbase warning: {warning}");
      }
    }
    RuntimeSource::Cache {
      fetched_at,
      age,
      reason,
    } => {
      eprintln!(
        "Dopbase warning: {reason}; using encrypted cache fetched at {fetched_at} ({age} old)."
      );
    }
  }
  eprintln!(
    "Dopbase: {}/{} ({}), {} key(s), source={}",
    loaded.project,
    loaded.environment,
    loaded.environment_id,
    loaded.entries.len(),
    loaded.source.as_str(),
  );
  let program = command.first().context("run requires a child command")?;
  let mut child_command = tokio::process::Command::new(program);
  child_command
    .args(&command[1..])
    .env_remove("DOPBASE_TOKEN");
  for entry in loaded.entries {
    child_command.env(entry.key, entry.value);
  }
  #[cfg(unix)]
  {
    use std::os::unix::process::CommandExt;
    child_command.as_std_mut().process_group(0);
  }
  let mut child = child_command
    .spawn()
    .with_context(|| format!("failed to start {program}"))?;
  #[cfg(unix)]
  {
    use nix::{
      sys::signal::{Signal, killpg},
      unistd::Pid,
    };
    let pid = child.id().context("child process has no process ID")? as i32;
    let status = tokio::select! {status=child.wait()=>status?,_=tokio::signal::ctrl_c()=>{let _=killpg(Pid::from_raw(pid),Signal::SIGINT);child.wait().await?},_=terminate_signal()=>{let _=killpg(Pid::from_raw(pid),Signal::SIGTERM);child.wait().await?}};
    use std::os::unix::process::ExitStatusExt;
    Ok(
      status
        .code()
        .unwrap_or_else(|| 128 + status.signal().unwrap_or(1)),
    )
  }
  #[cfg(not(unix))]
  {
    Ok(child.wait().await?.code().unwrap_or(1))
  }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RunEnvironmentSource {
  Argument,
  Environment,
  Default,
}

pub struct RunEnvironment {
  pub reference: String,
  source: RunEnvironmentSource,
}

pub fn run_environment(
  argument: Option<String>,
  environment: Result<String, env::VarError>,
  default: Option<&str>,
) -> Result<RunEnvironment> {
  if let Some(reference) = argument {
    return Ok(RunEnvironment {
      reference,
      source: RunEnvironmentSource::Argument,
    });
  }
  match environment {
    Ok(reference) if reference.is_empty() => bail!("DOPBASE_ENV is set but empty"),
    Ok(reference) => {
      return Ok(RunEnvironment {
        reference,
        source: RunEnvironmentSource::Environment,
      });
    }
    Err(env::VarError::NotUnicode(_)) => bail!("DOPBASE_ENV contains invalid Unicode"),
    Err(env::VarError::NotPresent) => {}
  }
  if let Some(reference) = default {
    return Ok(RunEnvironment {
      reference: reference.into(),
      source: RunEnvironmentSource::Default,
    });
  }
  bail!("No default environment is set. Set one with: dopbase env default <project/environment>")
}
#[cfg(unix)]
async fn terminate_signal() {
  if let Ok(mut signal) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
  {
    signal.recv().await;
  } else {
    std::future::pending::<()>().await;
  }
}

async fn admin(
  command: AdminCommand,
  data_dir: Option<PathBuf>,
) -> Result<i32> {
  match command {
    AdminCommand::ResetPassword {
      email,
      config,
      database_url,
      master_key_file,
    } => reset_password(email, data_dir, config, database_url, master_key_file).await?,
  }
  Ok(0)
}
async fn reset_password(
  email: String,
  data_dir: Option<PathBuf>,
  config: Option<PathBuf>,
  database_url: Option<String>,
  master_key_file: Option<PathBuf>,
) -> Result<()> {
  if !io::stdin().is_terminal() {
    bail!("password reset requires an interactive terminal");
  }
  let config = ServerConfig::load(&ServerOverrides {
    data_dir,
    config_path: config,
    database_url,
    master_key_path: master_key_file,
    ..Default::default()
  })?;
  ensure_data_dir(&config.data_dir)?;
  let _lock = crate::server::InstanceLock::acquire(&config.database_url)?
    .context("offline recovery requires a file-backed SQLite database")?;
  let db = crate::services::db::DbClient::connect(&config.database_url).await?;
  db.migrate().await?;
  let _crypto =
    crate::services::crypto::CryptoService::initialize(db.pool(), &config.master_key.path).await?;
  let admin: Option<(String, String)> =
    sqlx::query_as("SELECT id,email FROM admins WHERE email=? COLLATE NOCASE")
      .bind(email.trim())
      .fetch_optional(db.pool())
      .await?;
  let (admin_id, normalized) = admin.context("no administrator exists with that email")?;
  let password = rpassword::prompt_password("New password: ")?;
  let confirm = rpassword::prompt_password("Confirm new password: ")?;
  if password != confirm {
    bail!("passwords do not match");
  }
  crate::modules::common::validate_password(&password)
    .map_err(|error| anyhow::anyhow!("{:?}", error.errors))?;
  let hash = crate::modules::common::hash_password(&password)
    .map_err(|_| anyhow::anyhow!("password hashing failed"))?;
  let now = chrono::Utc::now().to_rfc3339();
  let mut tx = db.pool().begin().await?;
  sqlx::query("UPDATE admins SET password_hash=?,updated_at=? WHERE id=?")
    .bind(hash)
    .bind(&now)
    .bind(&admin_id)
    .execute(&mut *tx)
    .await?;
  sqlx::query("UPDATE sessions SET revoked_at=? WHERE revoked_at IS NULL")
    .bind(&now)
    .execute(&mut *tx)
    .await?;
  crate::modules::common::audit(
    &mut *tx,
    "system",
    None,
    Some("offline-recovery"),
    "admin.password_reset",
    None,
    None,
    Some("admin"),
    Some(&admin_id),
    json!({"email":normalized}),
  )
  .await?;
  tx.commit().await?;
  db.checkpoint().await?;
  db.close().await;
  println!("Password reset complete.\nAll human sessions were revoked.");
  Ok(())
}

fn env_id(value: &Value) -> Result<&str> {
  value
    .get("id")
    .and_then(Value::as_str)
    .context("environment response did not contain an ID")
}
fn array_len(value: &Value) -> usize {
  value.as_array().map_or(0, Vec::len)
}
fn parse_entries(value: &Value) -> Result<Vec<SecretInput>> {
  let entries = value
    .get("entries")
    .and_then(Value::as_array)
    .context("response did not contain entries")?;
  entries
    .iter()
    .map(|entry| {
      Ok(SecretInput {
        key: entry
          .get("key")
          .and_then(Value::as_str)
          .context("entry has no key")?
          .into(),
        value: entry
          .get("value")
          .and_then(Value::as_str)
          .context("entry has no value")?
          .into(),
      })
    })
    .collect()
}
fn confirm(
  prompt: &str,
  yes: bool,
) -> Result<()> {
  if yes {
    return Ok(());
  }
  if !io::stdin().is_terminal() {
    bail!("confirmation is required; pass --yes for non-interactive use");
  }
  print!("{prompt} [y/N] ");
  io::stdout().flush()?;
  let mut answer = String::new();
  io::stdin().read_line(&mut answer)?;
  if !matches!(answer.trim().to_lowercase().as_str(), "y" | "yes") {
    bail!("operation cancelled");
  }
  Ok(())
}
fn print_value(
  json_output: bool,
  value: &Value,
) {
  if json_output {
    println!(
      "{}",
      serde_json::to_string_pretty(value).unwrap_or_else(|_| "null".into())
    );
  } else if value.is_null() {
    println!("Done.");
  } else if let Some(value) = value.as_str() {
    println!("{value}");
  } else {
    println!(
      "{}",
      serde_json::to_string_pretty(value).unwrap_or_else(|_| "Done.".into())
    );
  }
}
fn write_private(
  path: &Path,
  contents: &[u8],
  force: bool,
) -> Result<()> {
  crate::utils::private_file::write(path, contents, force)
}
