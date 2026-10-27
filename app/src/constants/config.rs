/// Default network port. Change here and `DEFAULT_BIND_ADDRESS` /
/// `DEFAULT_PUBLIC_URL` must stay in sync (guarded by a test below).
pub const DEFAULT_PORT: u16 = 8840;
pub const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1:8840";
pub const DEFAULT_PUBLIC_URL: &str = "http://localhost:8840";
pub const SERVER_HOST_PLACEHOLDER: &str = "SERVER_HOST";
pub const DATA_DIRECTORY_NAME: &str = ".dopbase";
pub const DATABASE_FILENAME: &str = "dopbase.db";
pub const CLIENT_CONFIG_FILENAME: &str = "config.toml";
pub const MASTER_KEY_FILENAME: &str = "master.key";
pub const SERVER_CONFIG_FILENAME: &str = "server.toml";
pub const DAEMON_PID_FILENAME: &str = "dopbase.pid";
pub const DAEMON_LOG_FILENAME: &str = "serve.log";

/// Subdirectory of the data directory where retention archives expired audit
/// records before deleting them.
pub const AUDIT_ARCHIVE_DIRECTORY_NAME: &str = "audit-archive";

/// How often a running server applies audit retention. Records can outlive
/// `audit_retention_days` by up to this long.
pub const AUDIT_RETENTION_INTERVAL: std::time::Duration =
  std::time::Duration::from_secs(24 * 60 * 60);

pub const ENV_AUDIT_RETENTION_DAYS: &str = "DOPBASE_AUDIT_RETENTION_DAYS";
pub const ENV_SECRET_HISTORY_VERSIONS: &str = "DOPBASE_SECRET_HISTORY_VERSIONS";

/// Versions per secret that keep their encrypted value. Older versions keep
/// their metadata only.
pub const DEFAULT_SECRET_HISTORY_VERSIONS: u32 = 20;
/// Two is the smallest count that still leaves a version to roll back to.
pub const MIN_SECRET_HISTORY_VERSIONS: u32 = 2;
pub const MAX_SECRET_HISTORY_VERSIONS: u32 = 1000;
pub const ENV_TRUSTED_PROXIES: &str = "DOPBASE_TRUSTED_PROXIES";
pub const ENV_DATA_DIR: &str = "DOPBASE_DATA_DIR";
pub const ENV_DOCS: &str = "DOPBASE_DOCS";
pub const ENV_WEB_UI: &str = "DOPBASE_WEB_UI";
pub const ENV_HOST: &str = "DOPBASE_HOST";
pub const ENV_MASTER_KEY_PATH: &str = "DOPBASE_MASTER_KEY_PATH";
pub const ENV_PORT: &str = "DOPBASE_PORT";
pub const ENV_PUBLIC_URL: &str = "DOPBASE_PUBLIC_URL";
pub const ENV_ROOT_EMAIL: &str = "DOPBASE_ROOT_EMAIL";
pub const ENV_RUN_ENVIRONMENT: &str = "DOPBASE_ENV";
pub const ENV_SERVER_URL: &str = "DOPBASE_URL";
pub const ENV_SHUTDOWN_GRACE_SECONDS: &str = "DOPBASE_SHUTDOWN_GRACE_SECONDS";
pub const ENV_TOKEN: &str = "DOPBASE_TOKEN";
pub const ENV_CONTAINER: &str = "DOPBASE_CONTAINER";
/// Not read yet; reserved so a future file-mount mode cannot be spoofed by a
/// stored secret.
pub const ENV_SECRETS_PATH: &str = "DOPBASE_SECRETS_PATH";

/// Control variables `dopbase run` never injects from stored secrets.
pub const RUN_RESERVED_NAMES: [&str; 5] = [
  ENV_TOKEN,
  ENV_SERVER_URL,
  ENV_RUN_ENVIRONMENT,
  ENV_DATA_DIR,
  ENV_SECRETS_PATH,
];
/// Shell variables `dopbase run` never injects from stored secrets: the
/// union of what Doppler and Infisical skip.
pub const RUN_SHELL_RESERVED_NAMES: [&str; 4] = ["PATH", "HOME", "PS1", "PS2"];
pub const RUN_SHELL_RESERVED_PREFIXES: [&str; 2] = ["XDG_", "LC_"];
/// Names that make a program load extra code or run extra commands.
/// `dopbase run` skips them unless `--only` names them, then warns.
pub const RUN_CODE_LOADING_NAMES: [&str; 11] = [
  "LD_PRELOAD",
  "LD_LIBRARY_PATH",
  "DYLD_INSERT_LIBRARIES",
  "DYLD_LIBRARY_PATH",
  "NODE_OPTIONS",
  "PERL5OPT",
  "PYTHONWARNINGS",
  "BASH_ENV",
  "PROMPT_COMMAND",
  "JAVA_TOOL_OPTIONS",
  "PHPRC",
];
/// Names that change program behavior without loading code. `dopbase run`
/// injects them and warns.
pub const RUN_SENSITIVE_NAMES: [&str; 5] = [
  "NODE_VERSION",
  "BROWSER",
  "HOSTNAME",
  "WINDIR",
  "USERPROFILE",
];

/// Every environment variable read by the Dopbase executable.
pub const fn executable_environment_names() -> [&'static str; 16] {
  [
    ENV_TOKEN,
    ENV_CONTAINER,
    ENV_SERVER_URL,
    ENV_RUN_ENVIRONMENT,
    ENV_ROOT_EMAIL,
    ENV_DATA_DIR,
    ENV_HOST,
    ENV_PORT,
    ENV_PUBLIC_URL,
    ENV_DOCS,
    ENV_WEB_UI,
    ENV_MASTER_KEY_PATH,
    ENV_SHUTDOWN_GRACE_SECONDS,
    ENV_AUDIT_RETENTION_DAYS,
    ENV_SECRET_HISTORY_VERSIONS,
    ENV_TRUSTED_PROXIES,
  ]
}

/// Every `DOPBASE_*` environment variable the server configuration reads.
/// The background server strips these from its child process so the daemon
/// resolves its configuration from the captured launch inputs.
pub const fn daemon_environment_names() -> [&'static str; 12] {
  [
    ENV_DATA_DIR,
    ENV_DOCS,
    ENV_WEB_UI,
    ENV_HOST,
    ENV_MASTER_KEY_PATH,
    ENV_PORT,
    ENV_PUBLIC_URL,
    ENV_SERVER_URL,
    ENV_SHUTDOWN_GRACE_SECONDS,
    ENV_AUDIT_RETENTION_DAYS,
    ENV_SECRET_HISTORY_VERSIONS,
    ENV_TRUSTED_PROXIES,
  ]
}
