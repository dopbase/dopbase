/// Default network port. Change here and `DEFAULT_BIND_ADDRESS` /
/// `DEFAULT_PUBLIC_URL` must stay in sync (guarded by a test below).
pub const DEFAULT_PORT: u16 = 8840;
pub const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1:8840";
pub const DEFAULT_PUBLIC_URL: &str = "http://localhost:8840";
pub const DATA_DIRECTORY_NAME: &str = ".dopbase";
pub const DATABASE_FILENAME: &str = "dopbase.db";
pub const CLIENT_CONFIG_FILENAME: &str = "config.toml";
pub const MASTER_KEY_FILENAME: &str = "master.key";
pub const SERVER_CONFIG_FILENAME: &str = "server.toml";
pub const DAEMON_PID_FILENAME: &str = "dopbase.pid";
pub const DAEMON_LOG_FILENAME: &str = "serve.log";

pub const ENV_BIND_ADDRESS: &str = "DOPBASE_BIND_ADDRESS";
pub const ENV_DATA_DIR: &str = "DOPBASE_DATA_DIR";
pub const ENV_DATABASE_URL: &str = "DOPBASE_DATABASE_URL";
pub const ENV_DOCS: &str = "DOPBASE_DOCS";
pub const ENV_HOST: &str = "DOPBASE_HOST";
pub const ENV_MASTER_KEY_PATH: &str = "DOPBASE_MASTER_KEY_PATH";
pub const ENV_PORT: &str = "DOPBASE_PORT";
pub const ENV_PUBLIC_URL: &str = "DOPBASE_PUBLIC_URL";
pub const ENV_SERVER_URL: &str = "DOPBASE_URL";
pub const ENV_SHUTDOWN_GRACE_SECONDS: &str = "DOPBASE_SHUTDOWN_GRACE_SECONDS";

/// Every `DOPBASE_*` environment variable the server configuration reads.
/// The background server strips these from its child process so the daemon
/// resolves its configuration from explicit command-line flags only.
pub const fn daemon_environment_names() -> [&'static str; 10] {
  [
    ENV_BIND_ADDRESS,
    ENV_DATA_DIR,
    ENV_DATABASE_URL,
    ENV_DOCS,
    ENV_HOST,
    ENV_MASTER_KEY_PATH,
    ENV_PORT,
    ENV_PUBLIC_URL,
    ENV_SERVER_URL,
    ENV_SHUTDOWN_GRACE_SECONDS,
  ]
}
