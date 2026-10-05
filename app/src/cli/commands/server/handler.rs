use super::{ServerCommand, ServerLaunchArgs};
use crate::cli::output;
use crate::{
  config::{EnvironmentOverrides, ServerConfig, ServerOverrides, resolve_data_dir, sqlite_url},
  constants::config::DATABASE_FILENAME,
  daemon::ManagedDaemonState,
};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::{
  path::{Path, PathBuf},
  time::Duration,
};

pub(crate) async fn execute(
  command: ServerCommand,
  data_dir: Option<PathBuf>,
  json_output: bool,
) -> Result<i32> {
  match command {
    ServerCommand::Setup(args) => super::setup::execute(args, data_dir, json_output).await,
    ServerCommand::Up(_) => migration_notice(
      super::args::UP_MIGRATION,
      "dopbase server start --background",
      json_output,
    ),
    ServerCommand::Down { .. } => migration_notice(
      super::args::DOWN_MIGRATION,
      "dopbase server stop",
      json_output,
    ),
    ServerCommand::Start(args) => {
      let mut overrides = launch_overrides(args.launch, data_dir, args.background, args.supervised);
      overrides.web_ui = args.no_web_ui.then_some(false);
      if args.background {
        let environment = EnvironmentOverrides::read();
        let mut config = ServerConfig::load_with_environment(&overrides, environment.clone())?;
        overrides.data_dir = Some(config.data_dir.clone());
        overrides.config_path = Some(config.config_path.clone());
        config.daemon_launch = Some(crate::daemon::LaunchDescriptor {
          version: 1,
          overrides,
          environment,
          working_directory: std::env::current_dir()
            .context("failed to resolve current directory")?,
        });
        return crate::daemon::start(config, json_output).await;
      }
      if json_output {
        bail!(
          "--json cannot be used with foreground `dopbase server start`. Use `dopbase server start --background --json`"
        );
      }
      let ready = args
        .supervised
        .then(crate::daemon::Ready::attached)
        .flatten();
      let result = async {
        let config = if args.supervised {
          match std::env::var(crate::daemon::ENV_DAEMON_LAUNCH) {
            Ok(value) => serde_json::from_str::<crate::daemon::LaunchDescriptor>(&value)
              .context("invalid internal server launch settings")?
              .config()?,
            Err(std::env::VarError::NotPresent) => ServerConfig::load(&overrides)?,
            Err(error) => return Err(error.into()),
          }
        } else {
          ServerConfig::load(&overrides)?
        };
        crate::server::serve_with_ready(config, ready.as_ref()).await
      }
      .await;
      if let (Some(ready), Err(error)) = (&ready, &result) {
        ready.fail(&format!("{error:#}"));
      }
      result?;
      Ok(0)
    }
    ServerCommand::Stop { timeout } => {
      let resolved_data_dir = resolve_data_dir(data_dir.as_deref())?;
      reject_foreground(&resolved_data_dir)?;
      crate::daemon::stop(
        Some(&resolved_data_dir),
        Duration::from_secs(timeout),
        json_output,
      )
      .await
    }
    ServerCommand::Restart { timeout } => {
      let resolved_data_dir = resolve_data_dir(data_dir.as_deref())?;
      reject_foreground(&resolved_data_dir)?;
      let pid = match crate::daemon::inspect(&resolved_data_dir)? {
        ManagedDaemonState::Running(pid) => pid,
        ManagedDaemonState::Absent | ManagedDaemonState::Stale => bail!(
          "the background server is stopped. Start it with `dopbase server start --background`"
        ),
      };
      let launch = pid.launch.as_deref().context(
        "this server has no saved launch settings. Run `dopbase server stop`, then start it again with `dopbase server start --background` before using restart"
      )?;
      let config = launch.config()?;
      if config.data_dir != resolved_data_dir {
        bail!("saved launch settings do not match the selected data directory");
      }
      crate::daemon::restart(config, &pid, Duration::from_secs(timeout), json_output).await
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

fn migration_notice(
  message: &str,
  replacement: &str,
  json_output: bool,
) -> Result<i32> {
  if json_output {
    output::print_json(&json!({
      "success": false,
      "info": {
        "code": "COMMAND_REPLACED",
        "message": message,
        "replacement": replacement,
      },
    }))?;
  } else {
    output::print_info(message);
  }
  Ok(1)
}

fn reject_foreground(data_dir: &Path) -> Result<()> {
  if matches!(
    crate::daemon::inspect(data_dir),
    Ok(ManagedDaemonState::Absent | ManagedDaemonState::Stale)
  ) && crate::server::InstanceLock::is_held(&sqlite_url(&data_dir.join(DATABASE_FILENAME)))?
  {
    bail!("the server is running in the foreground. Stop it with Ctrl+C");
  }
  Ok(())
}

fn launch_overrides(
  args: ServerLaunchArgs,
  data_dir: Option<PathBuf>,
  background: bool,
  supervised: bool,
) -> ServerOverrides {
  ServerOverrides {
    data_dir,
    docs: args.docs(),
    web_ui: None,
    background,
    supervised,
    config_path: args.config,
    public_url: args.public_url,
    port: args.port,
    host: args.host,
    shutdown_grace_seconds: args.shutdown_grace_seconds,
    master_key_path: args.master_key_file,
  }
}

pub(super) fn load_server_config(
  args: ServerLaunchArgs,
  data_dir: Option<PathBuf>,
  background: bool,
  supervised: bool,
) -> Result<ServerConfig> {
  ServerConfig::load(&launch_overrides(args, data_dir, background, supervised))
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
