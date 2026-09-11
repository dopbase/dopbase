use super::output;
use crate::{
  cli::args::{ServerCommand, ServerLaunchArgs},
  config::{ServerConfig, ServerOverrides, resolve_data_dir, sqlite_url},
  constants::config::DATABASE_FILENAME,
  daemon::ManagedDaemonState,
};
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::{
  path::{Path, PathBuf},
  time::Duration,
};

pub(super) async fn execute(
  command: ServerCommand,
  data_dir: Option<PathBuf>,
  json_output: bool,
) -> Result<i32> {
  match command {
    ServerCommand::Start(args) => {
      if json_output {
        bail!("--json cannot be used with `dopbase server start`");
      }
      let supervised = args.supervised;
      let config = load_server_config(args.launch, data_dir, false, supervised)?;
      let ready = supervised.then(crate::daemon::Ready::attached).flatten();
      let result = crate::server::serve_with_ready(config, ready.as_ref()).await;
      if let (Some(ready), Err(error)) = (&ready, &result) {
        ready.fail(&format!("{error:#}"));
      }
      result?;
      Ok(0)
    }
    ServerCommand::Up(args) => {
      let config = load_server_config(args.clone(), data_dir, true, false)?;
      let flags = server_start_flags(&args, &config.data_dir);
      crate::daemon::start(config, &flags, json_output).await
    }
    ServerCommand::Down { timeout } => {
      let resolved_data_dir = resolve_data_dir(data_dir.as_deref())?;
      let check_foreground = matches!(
        crate::daemon::inspect(&resolved_data_dir),
        Ok(ManagedDaemonState::Absent | ManagedDaemonState::Stale)
      );
      if check_foreground
        && crate::server::InstanceLock::is_held(&sqlite_url(
          &resolved_data_dir.join(DATABASE_FILENAME),
        ))?
      {
        bail!("the server is running in the foreground. Stop it with Ctrl+C");
      }
      crate::daemon::stop(
        data_dir.as_deref(),
        Duration::from_secs(timeout),
        json_output,
      )
      .await
    }
    ServerCommand::Status => server_status(data_dir.as_deref(), json_output),
    ServerCommand::Logs {
      lines,
      clean,
      watch,
    } => {
      if json_output && watch {
        bail!("--json cannot be used with `dopbase server logs --watch`");
      }
      crate::daemon::logs(data_dir.as_deref(), lines, clean, watch, json_output).await
    }
  }
}

fn load_server_config(
  args: ServerLaunchArgs,
  data_dir: Option<PathBuf>,
  background: bool,
  supervised: bool,
) -> Result<ServerConfig> {
  ServerConfig::load(&ServerOverrides {
    data_dir,
    docs: args.docs(),
    background,
    supervised,
    config_path: args.config,
    public_url: args.public_url,
    port: args.port,
    host: args.host,
    shutdown_grace_seconds: args.shutdown_grace_seconds,
    master_key_path: args.master_key_file,
  })
}

fn server_status(
  data_dir: Option<&Path>,
  json_output: bool,
) -> Result<i32> {
  let config = ServerConfig::load(&ServerOverrides {
    data_dir: data_dir.map(Path::to_path_buf),
    ..Default::default()
  })?;
  let log_file = crate::daemon::log_file_path(&config.data_dir);
  match crate::daemon::inspect(&config.data_dir)? {
    ManagedDaemonState::Running(pid) => {
      if json_output {
        output::print_value(
          true,
          &json!({
            "status": "running",
            "mode": "background",
            "pid": pid.pid,
            "started_at": pid.started_at,
            "version": pid.version,
            "bind_address": pid.bind_address,
            "public_url": pid.resolved_public_url(),
            "data_dir": config.data_dir,
            "log_file": log_file,
          }),
        );
      } else {
        println!(
          "Server:     running (background)\nPID:        {}\nStarted:    {}\nURL:        {}\nData:       {}\nLog:        {}",
          pid.pid,
          pid.started_at,
          pid.resolved_public_url().as_deref().unwrap_or("unknown"),
          config.data_dir.display(),
          log_file.display(),
        );
      }
      Ok(0)
    }
    state @ (ManagedDaemonState::Absent | ManagedDaemonState::Stale) => {
      let foreground = crate::server::InstanceLock::is_held(&config.database_url)?;
      if foreground {
        if json_output {
          output::print_value(
            true,
            &json!({"status":"running","mode":"foreground","data_dir":config.data_dir}),
          );
        } else {
          println!(
            "Server:     running (foreground)\nData:       {}",
            config.data_dir.display()
          );
        }
        Ok(0)
      } else {
        let stale_pid_file = matches!(state, ManagedDaemonState::Stale);
        if json_output {
          output::print_value(
            true,
            &json!({
              "status":"stopped",
              "mode":Value::Null,
              "data_dir":config.data_dir,
              "stale_pid_file":stale_pid_file,
            }),
          );
        } else if stale_pid_file {
          println!(
            "Server:     stopped\nData:       {}\nNote:       stale PID file found",
            config.data_dir.display()
          );
        } else {
          println!(
            "Server:     stopped\nData:       {}",
            config.data_dir.display()
          );
        }
        Ok(1)
      }
    }
  }
}

/// Build the argv for the detached server from the public launch options.
fn server_start_flags(
  args: &ServerLaunchArgs,
  data_dir: &Path,
) -> Vec<String> {
  let mut flags = vec!["server".to_string(), "start".to_string()];
  if let Some(value) = &args.config {
    flags.push("--config".into());
    flags.push(value.to_string_lossy().into_owned());
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
