use app::{
  cli::{
    local_config::{ClientConfig, ResolvedServer, ServerSource},
    session,
  },
  config::{MasterKeyConfig, ServerConfig},
  modules::bootstrap::{model::BootstrapAdminRequest, service},
  server,
  state::AppState,
};
use axum::Router;
use serde_json::{Value, json};
use std::{
  path::{Path, PathBuf},
  process::{Command, Output, Stdio},
  time::Duration,
};
use tempfile::TempDir;
use tokio::io::AsyncWriteExt;

pub const EMAIL: &str = "admin@example.com";
pub const PASSWORD: &str = "fixture-password-123";

pub fn command(data: &Path) -> Command {
  let mut command = Command::new(env!("CARGO_BIN_EXE_dopbase"));
  for name in app::constants::config::executable_environment_names() {
    command.env_remove(name);
  }
  command.env_remove("DOPBASE_INTERNAL_DAEMON_LAUNCH");
  command.env("NO_COLOR", "1").arg("--data-dir").arg(data);
  command
}

pub async fn output(
  command: Command,
  input: Option<&[u8]>,
) -> Output {
  let mut command = tokio::process::Command::from(command);
  command
    .kill_on_drop(true)
    .stdin(if input.is_some() {
      Stdio::piped()
    } else {
      Stdio::null()
    })
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
  let mut child = command.spawn().unwrap();
  if let Some(input) = input {
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(input).await.unwrap();
  }
  tokio::time::timeout(Duration::from_secs(15), child.wait_with_output())
    .await
    .expect("CLI timed out")
    .unwrap()
}

pub fn success(output: &Output) -> Value {
  assert!(
    output.status.success(),
    "CLI failed: {}",
    String::from_utf8_lossy(&output.stderr)
  );
  serde_json::from_slice(&output.stdout).expect("CLI did not return JSON")
}

pub fn failure(
  output: &Output,
  expected: &str,
) {
  assert!(!output.status.success());
  assert!(
    String::from_utf8_lossy(&output.stderr).contains(expected),
    "missing error {expected}: {}",
    String::from_utf8_lossy(&output.stderr)
  );
}

pub struct Fixture {
  pub state: AppState,
  pub url: String,
  pub client_dir: PathBuf,
  pub data_dir: PathBuf,
  pub directory: TempDir,
  pub token: String,
  task: tokio::task::JoinHandle<()>,
}

impl Fixture {
  pub async fn new() -> Self {
    let mut fixture = Self::uninitialized().await;
    let setup_token = fixture.state.setup.read().await.token.clone().unwrap();
    service::create(
      &fixture.state,
      BootstrapAdminRequest {
        setup_token,
        email: EMAIL.into(),
        password: PASSWORD.into(),
      },
    )
    .await
    .unwrap();
    let response = reqwest::Client::new()
      .post(format!("{}/api/v1/auth/login", fixture.url))
      .json(&json!({"email":EMAIL,"password":PASSWORD,"sessionKind":"cli"}))
      .send()
      .await
      .unwrap()
      .error_for_status()
      .unwrap()
      .json::<Value>()
      .await
      .unwrap();
    fixture.token = response["data"]["token"].as_str().unwrap().into();
    session::save(&fixture.resolved(), &fixture.token, Some(EMAIL)).unwrap();
    fixture
  }

  pub async fn uninitialized() -> Self {
    let directory = TempDir::new().unwrap();
    let data_dir = directory.path().join("server");
    let client_dir = directory.path().join("client");
    let config = ServerConfig {
      data_dir: data_dir.clone(),
      database_url: app::config::sqlite_url(&data_dir.join("dopbase.db")),
      master_key: MasterKeyConfig {
        provider: "file".into(),
        path: data_dir.join("master.key"),
      },
      ..ServerConfig::default()
    };
    let state = server::build_setup_state(config).await.unwrap();
    let (url, task) = serve(server::router(state.clone())).await;
    Self {
      state,
      url,
      client_dir,
      data_dir,
      directory,
      token: String::new(),
      task,
    }
  }

  pub fn resolved(&self) -> ResolvedServer {
    ResolvedServer {
      url: self.url.clone(),
      source: ServerSource::Argument,
      config_path: self.client_dir.join("config.toml"),
      config: ClientConfig::default(),
    }
  }

  pub fn command(&self) -> Command {
    let mut cmd = command(&self.client_dir);
    cmd.arg("--server").arg(&self.url);
    cmd
  }

  pub async fn run(
    &self,
    arguments: &[&str],
  ) -> Output {
    let mut cmd = self.command();
    cmd.args(arguments);
    output(cmd, None).await
  }

  pub async fn json(
    &self,
    arguments: &[&str],
  ) -> Value {
    let mut cmd = self.command();
    cmd.arg("--json").args(arguments);
    success(&output(cmd, None).await)
  }

  pub async fn request(
    &self,
    method: reqwest::Method,
    path: &str,
    body: Option<Value>,
  ) -> Value {
    let mut request = reqwest::Client::new()
      .request(method, format!("{}{path}", self.url))
      .bearer_auth(&self.token);
    if let Some(body) = body {
      request = request.json(&body);
    }
    let response = request.send().await.unwrap().error_for_status().unwrap();
    response.json::<Value>().await.unwrap()["data"].clone()
  }

  pub async fn environment(&self) -> String {
    self.json(&["project", "create", "fixture"]).await;
    self.json(&["env", "create", "fixture/local"]).await["id"]
      .as_str()
      .unwrap()
      .into()
  }

  pub async fn stop(&self) {
    self.task.abort();
    while !self.task.is_finished() {
      tokio::task::yield_now().await;
    }
    self.state.db.close().await;
  }
}

impl Drop for Fixture {
  fn drop(&mut self) {
    self.task.abort();
  }
}

pub async fn serve(router: Router) -> (String, tokio::task::JoinHandle<()>) {
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let url = format!("http://{}", listener.local_addr().unwrap());
  let task = tokio::spawn(async move {
    axum::serve(listener, router).await.unwrap();
  });
  (url, task)
}

#[cfg(unix)]
pub mod terminal {
  use nix::{
    fcntl::{FcntlArg, OFlag, fcntl},
    pty::{Winsize, openpty},
  };
  use std::{
    fs::File,
    io::{Read, Write},
    os::unix::process::CommandExt,
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
  };

  pub struct Terminal {
    child: Child,
    master: File,
    pub transcript: String,
    cursor: usize,
  }

  impl Terminal {
    pub fn new(mut command: Command) -> Self {
      let pty = openpty(
        Some(&Winsize {
          ws_row: 40,
          ws_col: 120,
          ws_xpixel: 0,
          ws_ypixel: 0,
        }),
        None,
      )
      .unwrap();
      let slave = File::from(pty.slave);
      let master = File::from(pty.master);
      fcntl(&master, FcntlArg::F_SETFL(OFlag::O_NONBLOCK)).unwrap();
      command
        .env("TERM", "xterm")
        .stdin(Stdio::from(slave.try_clone().unwrap()))
        .stdout(Stdio::from(slave.try_clone().unwrap()))
        .stderr(Stdio::from(slave));
      unsafe {
        command.pre_exec(|| {
          nix::unistd::setsid().map_err(std::io::Error::from)?;
          if nix::libc::ioctl(0, nix::libc::TIOCSCTTY as _, 0) == -1 {
            return Err(std::io::Error::last_os_error());
          }
          Ok(())
        });
      }
      Self {
        child: command.spawn().unwrap(),
        master,
        transcript: String::new(),
        cursor: 0,
      }
    }

    fn read(&mut self) {
      let mut buffer = [0; 8192];
      while let Ok(count) = self.master.read(&mut buffer) {
        if count == 0 {
          break;
        }
        let chunk = String::from_utf8_lossy(&buffer[..count]);
        if chunk.contains("\x1b[6n") {
          self.master.write_all(b"\x1b[1;1R").unwrap();
        }
        self.transcript.push_str(&chunk);
      }
    }

    pub fn reply(
      &mut self,
      expected: &str,
      input: &str,
    ) {
      let deadline = Instant::now() + Duration::from_secs(10);
      loop {
        self.read();
        if self.transcript[self.cursor..].contains(expected) {
          break;
        }
        assert!(
          self.child.try_wait().unwrap().is_none(),
          "CLI exited before {expected}"
        );
        assert!(Instant::now() < deadline, "prompt timed out: {expected}");
        std::thread::sleep(Duration::from_millis(10));
      }
      self.cursor = self.transcript.len();
      self
        .master
        .write_all(input.replace("\n", "\r").as_bytes())
        .unwrap();
    }

    pub fn finish(&mut self) -> ExitStatus {
      let deadline = Instant::now() + Duration::from_secs(10);
      loop {
        self.read();
        if let Some(status) = self.child.try_wait().unwrap() {
          self.read();
          return status;
        }
        assert!(Instant::now() < deadline, "terminal CLI timed out");
        std::thread::sleep(Duration::from_millis(10));
      }
    }
  }

  impl Drop for Terminal {
    fn drop(&mut self) {
      if self.child.try_wait().ok().flatten().is_none() {
        let _ = nix::sys::signal::killpg(
          nix::unistd::Pid::from_raw(self.child.id() as i32),
          nix::sys::signal::Signal::SIGKILL,
        );
        let _ = self.child.kill();
      }
      let _ = self.child.wait();
    }
  }
}
