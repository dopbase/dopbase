pub(crate) const AFTER_HELP: &str = "\
Environment variables:
  DOPBASE_TOKEN                   Bearer token for a machine runner or AI agent. Overrides the saved login
  DOPBASE_URL                     Server URL for client commands when --server is not set
  DOPBASE_ENV                     Environment for dopbase run when its argument is omitted
  DOPBASE_DATA_DIR                State and configuration directory (default: ~/.dopbase)
  DOPBASE_HOST                    Server bind host
  DOPBASE_PORT                    Server port (default: 8840)
  DOPBASE_PUBLIC_URL              Public URL
  DOPBASE_DOCS                    Enable or disable Swagger UI (true or false)
  DOPBASE_MASTER_KEY_PATH         Path to the server master key file `./path/to/your.key`
  DOPBASE_SHUTDOWN_GRACE_SECONDS  Seconds allowed for graceful shutdown

Quickstart:
  dopbase server start                     # run a server on http://localhost:8840
  dopbase server up                        # run the server in the background
  dopbase login                            # authenticate with the active server
  dopbase init myapp/dev --from .env       # create a project + environment from a secrets file
  dopbase secret set myapp/dev API_KEY --stdin
  dopbase run myapp/dev -- node server.js  # run with secrets injected as env vars

Common server options:
  --host <HOST>    Bind host (default: 127.0.0.1)
  --port <PORT>    Listen port (default: 8840)

Run 'dopbase help <command>' for details on any command.
";

pub(crate) const RUN_HELP: &str = "\
Examples:
  dopbase run -- npm run dev
  dopbase run env_482731 -- npm run dev
  dopbase run payment-service/development -- npm run dev
  dopbase run payment-service/production -t dbs_xxx -- npm start
  dopbase run payment-service/production --token dbs_xxx -- npm start
";

pub(crate) const ADMIN_HELP: &str = "\
Examples:
  dopbase admin reset-password admin@example.com
  dopbase admin factory-reset
";
pub(crate) const ADMIN_RESET_PASSWORD_HELP: &str =
  "Examples:\n  dopbase admin reset-password admin@example.com\n";
pub(crate) const ADMIN_FACTORY_RESET_HELP: &str = "\
Examples:
  dopbase admin factory-reset
  dopbase admin factory-reset --no-backup
  dopbase --data-dir /srv/dopbase admin factory-reset
";

pub(crate) const UPDATE_HELP: &str = "Examples:\n  dopbase update\n";
pub(crate) const BACKUP_HELP: &str = "\
Examples:
  dopbase backup
  dopbase backup pre-migration --output ./pre-migration.dop
";
pub(crate) const RESTORE_HELP: &str = "\
Examples:
  dopbase restore ./backup.dop
  dopbase restore ./backup.dop --setup-token dbs_... --yes
";

pub(crate) const ENVIRONMENT_ARG_HELP: &str = "Existing environment reference: an environment ID or \
PROJECT_REF/ENVIRONMENT_NAME. PROJECT_REF can be a project ID or name. For example: payment-service/production.";
