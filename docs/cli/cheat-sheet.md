---
title: "CLI cheat sheet"
description: "Every Dopbase CLI command and option, with short examples."
---

# CLI cheat sheet

Use `payment-service/production` anywhere a command asks for an environment.
An immutable ID such as `env_482731` works too.

## Global options

Global options work with commands throughout the CLI. A command may reject an
option when it does not apply, such as `--server` with a local server command.

| Option             | What it does                                                                        | Example                                                     |
| ------------------ | ----------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| `--server <URL>`   | Use this server for one client command instead of the saved server or `DOPBASE_URL` | `dopbase --server https://dopbase.example.com project list` |
| `--data-dir <DIR>` | Use another directory for Dopbase state and configuration                           | `dopbase --data-dir /srv/dopbase server status`             |
| `--json`           | Print machine-readable JSON where the command supports it                           | `dopbase --json project list`                               |
| `-h`, `--help`     | Show help for the current command                                                   | `dopbase secret set --help`                                 |
| `-V`, `--version`  | Show the installed Dopbase version                                                  | `dopbase --version`                                         |

The built-in help command accepts the same command path. For example,
`dopbase help secret set` shows the usage, argument descriptions, options, and
examples for `secret set`.

## Server and connection

| Task                               | Command                               | Command options                                                                                                                                                    | Example                                              |
| ---------------------------------- | ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------- |
| Run the server in this terminal    | `dopbase server start`                | `--config <FILE>`, `--host <HOST>`, `--port <PORT>`, `--public-url <URL>`, `--shutdown-grace-seconds <SECONDS>`, `--docs`, `--no-docs`, `--master-key-file <FILE>` | `dopbase server start --port 9000`                   |
| Start the server in the background | `dopbase server up`                   | Same options as `server start`                                                                                                                                     | `dopbase server up --docs`                           |
| Stop the background server         | `dopbase server down`                 | `--timeout <SECONDS>` (default: `10`)                                                                                                                              | `dopbase server down --timeout 30`                   |
| Check the local server process     | `dopbase server status`               | No command-specific options                                                                                                                                        | `dopbase server status`                              |
| Read background logs               | `dopbase server logs`                 | `--lines <COUNT>` (default: `100`), `-w`, `--watch`, `--clean`                                                                                                     | `dopbase server logs --clean --watch`                |
| Select another server              | `dopbase client connect <SERVER_URL>` | No command-specific options                                                                                                                                        | `dopbase client connect https://dopbase.example.com` |
| Return to the local server         | `dopbase client connect local`        | No command-specific options                                                                                                                                        | `dopbase client connect local`                       |
| Check connection and login         | `dopbase client status`               | No command-specific options                                                                                                                                        | `dopbase client status`                              |
| Check connection and login (alias) | `dopbase status`                      | No command-specific options                                                                                                                                        | `dopbase status`                                     |
| Sign in                            | `dopbase login`                       | `--token` saves a runner token from a masked prompt or piped standard input                                                                                         | `printf '%s' "$RUNNER_TOKEN" \| dopbase login --token` |
| Sign out                           | `dopbase logout`                      | No command-specific options                                                                                                                                        | `dopbase logout`                                     |

`--host` defaults to `127.0.0.1` and `--port` defaults to `8840`. When
`--host` exposes the server beyond loopback, also set `--public-url`.

## Projects and environments

| Task                                   | Command                                              | Command options             | Example                                                |
| -------------------------------------- | ---------------------------------------------------- | --------------------------- | ------------------------------------------------------ |
| Create a project and first environment | `dopbase init <PROJECT> <ENVIRONMENT> --from <FILE>` | `--from <FILE>` (required)  | `dopbase init payment-service development --from .env` |
| Create a project                       | `dopbase project create <NAME>`                      | No command-specific options | `dopbase project create payment-service`               |
| List projects                          | `dopbase project list`                               | No command-specific options | `dopbase project list`                                 |
| Show a project                         | `dopbase project show <PROJECT>`                     | No command-specific options | `dopbase project show payment-service`                 |
| Rename a project                       | `dopbase project rename <PROJECT> <NEW_NAME>`        | No command-specific options | `dopbase project rename payment-service payments`      |
| Delete a project                       | `dopbase project delete <PROJECT>`                   | `--yes` skips confirmation  | `dopbase project delete payment-service --yes`         |
| Create an environment                  | `dopbase env create <PROJECT> <NAME>`                | No command-specific options | `dopbase env create payment-service production`        |
| List environments                      | `dopbase env list [PROJECT]`                         | No command-specific options | `dopbase env list payment-service`                     |
| Show an environment                    | `dopbase env show <ENVIRONMENT>`                     | No command-specific options | `dopbase env show payment-service/production`          |
| Set the run default                    | `dopbase env default <ENVIRONMENT>`                  | No command-specific options | `dopbase env default payment-service/development`      |
| Clear the run default                  | `dopbase env default --clear`                        | `--clear`                   | `dopbase env default --clear`                          |
| Rename an environment                  | `dopbase env rename <ENVIRONMENT> <NEW_NAME>`        | No command-specific options | `dopbase env rename payment-service/production prod`   |
| Delete an environment                  | `dopbase env delete <ENVIRONMENT>`                   | `--yes` skips confirmation  | `dopbase env delete payment-service/staging --yes`     |

## Secrets and application commands

| Task                      | Command                                        | Command options                                                                                                | Example                                                                                   |
| ------------------------- | ---------------------------------------------- | -------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| List secret keys          | `dopbase secret list <ENVIRONMENT>`            | No command-specific options                                                                                    | `dopbase secret list payment-service/production`                                          |
| Set a secret              | `dopbase secret set <ENVIRONMENT> <KEY>`       | `--stdin` reads the value from standard input                                                                  | `printf '%s' "$API_KEY" \| dopbase secret set payment-service/production API_KEY --stdin` |
| Show secret metadata      | `dopbase secret get <ENVIRONMENT> <KEY>`       | No command-specific options                                                                                    | `dopbase secret get payment-service/production API_KEY`                                   |
| Reveal a secret           | `dopbase secret get <ENVIRONMENT> <KEY>`       | `--reveal` prints the value after password confirmation                                                        | `dopbase secret get payment-service/production API_KEY --reveal`                          |
| Delete a secret           | `dopbase secret delete <ENVIRONMENT> <KEY>`    | `--yes` skips confirmation                                                                                     | `dopbase secret delete payment-service/production API_KEY --yes`                          |
| Import a dotenv file      | `dopbase import <ENVIRONMENT> <FILE>`          | `--dry-run` previews changes; `--replace` removes remote keys absent from the file; `--yes` skips confirmation | `dopbase import payment-service/staging .env.staging --dry-run`                           |
| Export to a file          | `dopbase export <ENVIRONMENT> --output <FILE>` | `--output <FILE>` or `--stdout` is required; `--force` overwrites an existing file                             | `dopbase export payment-service/staging --output .env.staging --force`                    |
| Export to standard output | `dopbase export <ENVIRONMENT> --stdout`        | `--stdout` conflicts with `--output`                                                                           | `dopbase export payment-service/staging --stdout`                                         |
| Run with injected secrets | `dopbase run [ENVIRONMENT] -- <COMMAND>`       | `--token <TOKEN>` overrides other credentials; everything after `--` is the child command                       | `dopbase run payment-service/development -- npm start`                                    |

## Tokens, backups, and maintenance

| Task                            | Command                                            | Command options                                                                                                                            | Example                                                                            |
| ------------------------------- | -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------- |
| Create a runner token           | `dopbase token create <ENVIRONMENT> --name <NAME>` | `--name <NAME>` is required; `--role <ROLE>` defaults to `runner`                                                                          | `dopbase token create payment-service/production --name deploy --role runner`      |
| List runner tokens              | `dopbase token list <ENVIRONMENT>`                 | No command-specific options                                                                                                                | `dopbase token list payment-service/production`                                    |
| Revoke a token                  | `dopbase token revoke <TOKEN_ID>`                  | No command-specific options                                                                                                                | `dopbase token revoke tok_01ABCDEF`                                                |
| Create a backup                 | `dopbase backup [NAME]`                            | `-o <FILE>`, `--output <FILE>` downloads a local copy                                                                                      | `dopbase backup pre-upgrade --output backup.dop`                                   |
| Restore a backup                | `dopbase restore <FILE>`                           | `-k <KEY>`, `--key <KEY>` supplies the source master key; `--setup-token <TOKEN>` is used for first-run restore `--yes` skips confirmation | `dopbase restore backup.dop --key ./master.key --yes`                              |
| Reset an admin password offline | `dopbase admin reset-password <EMAIL>`             | `--config <FILE>`, `--master-key-file <FILE>`                                                                                              | `dopbase admin reset-password admin@example.com --config /srv/dopbase/server.toml` |
| Reset the local instance offline | `dopbase admin factory-reset`                      | `--config <FILE>`                                                                                                                           | `dopbase --data-dir /srv/dopbase admin factory-reset`                             |
| Check for a new release         | `dopbase update`                                   | No command-specific options                                                                                                                | `dopbase update`                                                                   |

Run `dopbase help <command>` or `dopbase help <command> <subcommand>` for the
same option descriptions in the terminal.
