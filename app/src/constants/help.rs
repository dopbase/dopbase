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

pub(crate) const PROJECT_HELP: &str = "\
Examples:
  dopbase project create payment-service
  dopbase project list
  dopbase project show payment-service
  dopbase project rename payment-service payments
  dopbase project delete payment-service
";

pub(crate) const PROJECT_CREATE_HELP: &str =
  "Examples:\n  dopbase project create payment-service\n";
pub(crate) const PROJECT_LIST_HELP: &str = "Examples:\n  dopbase project list\n";
pub(crate) const PROJECT_SHOW_HELP: &str = "Examples:\n  dopbase project show payment-service\n";
pub(crate) const PROJECT_RENAME_HELP: &str =
  "Examples:\n  dopbase project rename payment-service payments\n";
pub(crate) const PROJECT_DELETE_HELP: &str = "\
Examples:
  dopbase project delete payment-service
  dopbase project delete payment-service --yes
";

pub(crate) const ENV_HELP: &str = "\
Examples:
  dopbase env create payment-service/production
  dopbase env list payment-service
  dopbase env show payment-service/production
  dopbase env default payment-service/development
  dopbase env rename payment-service/production prod
  dopbase env delete payment-service/staging
";

pub(crate) const ENV_DEFAULT_HELP: &str = "\
Examples:
  dopbase env default payment-service/development
  dopbase env default --clear
";
pub(crate) const ENV_CREATE_HELP: &str =
  "Examples:\n  dopbase env create payment-service/production\n";
pub(crate) const ENV_LIST_HELP: &str = "\
Examples:
  dopbase env list
  dopbase env list payment-service
";
pub(crate) const ENV_SHOW_HELP: &str = "\
Examples:
  dopbase env show payment-service/production
  dopbase env show env_482731
";
pub(crate) const ENV_RENAME_HELP: &str =
  "Examples:\n  dopbase env rename payment-service/production prod\n";
pub(crate) const ENV_DELETE_HELP: &str = "\
Examples:
  dopbase env delete payment-service/staging
  dopbase env delete payment-service/staging --yes
";

pub(crate) const SECRET_HELP: &str = "\
Examples:
  dopbase secret list payment-service/production
  dopbase secret set payment-service/production API_KEY
  dopbase secret get payment-service/production API_KEY
  dopbase secret get payment-service/production API_KEY --reveal
  dopbase secret delete payment-service/production API_KEY

Use PROJECT_REF/ENVIRONMENT_NAME for readable references. PROJECT_REF can be a
project ID or name. Immutable environment IDs such as env_482731 also work.
";

pub(crate) const SECRET_LIST_HELP: &str = "\
Examples:
  dopbase secret list payment-service/production
  dopbase secret list env_482731

Run `dopbase env list` to find an environment.
";

pub(crate) const SECRET_SET_HELP: &str = "\
Examples:
  dopbase secret set payment-service/production API_KEY
  dopbase secret set payment-service/production API_KEY --stdin
  printf '%s' \"$API_KEY\" | dopbase secret set payment-service/production API_KEY --stdin

Without --stdin, Dopbase uses a masked prompt and displays * for each character.

With --stdin, Dopbase reads the value until EOF. In a terminal, paste or type
the value, then press Ctrl+D. On Windows, press Ctrl+Z, then Enter. Piped input
is read exactly as supplied.
";

pub(crate) const SECRET_GET_HELP: &str = "\
Examples:
  dopbase secret get payment-service/production API_KEY
  dopbase secret get payment-service/production API_KEY --reveal
";

pub(crate) const SECRET_DELETE_HELP: &str = "\
Examples:
  dopbase secret delete payment-service/production API_KEY
  dopbase secret delete payment-service/production API_KEY --yes
";

pub(crate) const IMPORT_HELP: &str = "\
Examples:
  dopbase import payment-service/production .env.production
  dopbase import payment-service/production secrets.json --dry-run
  cat secrets.yml | dopbase import payment-service/production - --format yaml
";
pub(crate) const EXPORT_HELP: &str = "\
Examples:
  dopbase export payment-service/production --output .env.production
  dopbase export payment-service/production --output secrets.json
  dopbase export payment-service/production --stdout --format yaml
";
pub(crate) const RUN_HELP: &str = "\
Examples:
  dopbase run -- npm run dev
  dopbase run env_482731 -- npm run dev
  dopbase run payment-service/development -- npm run dev
  dopbase run payment-service/production -t dbs_xxx -- npm start
  dopbase run payment-service/production --token dbs_xxx -- npm start
";

pub(crate) const TOKEN_HELP: &str = "\
Examples:
  dopbase token create payment-service/production --name deploy
  dopbase token list payment-service/production
  dopbase token revoke tok_01ABCDEF
";
pub(crate) const TOKEN_CREATE_HELP: &str =
  "Examples:\n  dopbase token create payment-service/production --name deploy\n";
pub(crate) const TOKEN_LIST_HELP: &str =
  "Examples:\n  dopbase token list payment-service/production\n";
pub(crate) const TOKEN_REVOKE_HELP: &str = "Examples:\n  dopbase token revoke tok_01ABCDEF\n";

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
