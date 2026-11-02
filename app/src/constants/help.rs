pub(crate) const AFTER_HELP: &str = "\
Environment variables:
  DOPBASE_TOKEN                   Bearer token for a machine runner or AI agent. Overrides the saved login
  DOPBASE_URL                     Server URL for client commands when --server is not set
  DOPBASE_ENV                     Environment for dopbase run when its argument is omitted
  DOPBASE_ROOT_EMAIL              Root email for first startup or setup; setup --email overrides it
  DOPBASE_DATA_DIR                State and configuration directory (default: ~/.dopbase)
  DOPBASE_HOST                    Server bind host
  DOPBASE_PORT                    Server port (default: 8840)
  DOPBASE_PUBLIC_URL              Public URL
  DOPBASE_DOCS                    Enable or disable Swagger UI (true or false)
  DOPBASE_WEB_UI                  Enable or disable the web UI (default: true)
  DOPBASE_UPDATE_CHECK            Check for a newer release on the Status page (default: false)
  DOPBASE_MASTER_KEY_PATH         Path to the server master key file `./path/to/your.key`
  DOPBASE_SHUTDOWN_GRACE_SECONDS  Seconds allowed for graceful shutdown
  DOPBASE_AUDIT_RETENTION_DAYS    Days to keep audit records (unset keeps the whole log)
  DOPBASE_SECRET_HISTORY_VERSIONS Versions per secret that can be rolled back to (default: 20)
  DOPBASE_TRUSTED_PROXIES         Proxies allowed to report the client IP, comma-separated

Quickstart:
  dopbase server setup                     # initialize the local instance once
  dopbase server start                     # start on http://localhost:8840; fresh instances enter setup
  dopbase server start --background        # run the server in the background (also -b)
  dopbase server stop                      # stop the background server
  dopbase server restart                   # restart with saved launch settings
  dopbase login                            # authenticate with the active server
  dopbase init                             # create a project + environment from ./.env
  dopbase init myapp/dev --from .env       # create a project + environment from a secrets file
  dopbase secret set myapp/dev API_KEY --stdin
  dopbase run myapp/dev -- node server.js  # run with secrets injected as env vars

Common server options:
  --host <HOST>    Bind host (default: 127.0.0.1)
  --port <PORT>    Listen port (default: 8840)
  --no-web-ui      Disable the web UI for server start

Run 'dopbase help <command>' for details on any command.
";

pub(crate) const ENVIRONMENT_ARG_HELP: &str = "Existing environment reference: an environment ID or \
PROJECT_REF/ENVIRONMENT_NAME. PROJECT_REF can be a project ID or name. For example: payment-service/production.";
pub(crate) const RUN_ENVIRONMENT_ARG_HELP: &str = "Existing environment reference: an environment ID or \
PROJECT_REF/ENVIRONMENT_NAME. PROJECT_REF can be a project ID or name. For example: payment-service/production. \
With a runner token, leave it out: the token's own environment is used, and a reference only checks it.";
