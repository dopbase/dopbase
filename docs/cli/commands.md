---
title: "Command reference"
description: "Reference for every Dopbase CLI command: connections, authentication, projects, environments, and secret operations."
---

# Command reference

This page defines the implemented v{{version}} command surface. Dopbase does not infer a
project or environment from the current directory. Management commands receive
an environment reference directly. `run` may use a server-scoped default from
the user configuration.

For a compact table of every command, command-specific option, and example, see
the [CLI cheat sheet](./cheat-sheet).

## Connections and authentication

| Command                                | Purpose                             |
| -------------------------------------- | ----------------------------------- |
| `dopbase server start`                 | Run the server in the foreground    |
| `dopbase server up`                    | Start the server in the background  |
| `dopbase server down`                  | Stop the background server          |
| `dopbase server status`                | Check the local server process      |
| `dopbase server logs`                  | Read background server logs         |
| `dopbase client connect <server-url>`  | Validate and save another server    |
| `dopbase client connect local`         | Return to the implicit local server |
| `dopbase login`                        | Sign in or save a runner token      |
| `dopbase logout`                       | Remove the active saved credential  |
| `dopbase client status`                | Show safe effective client settings |
| `dopbase status`                       | Alias for `dopbase client status`   |
| `dopbase update`                       | Check GitHub for a newer release    |
| `dopbase admin reset-password <email>` | Reset a user password offline       |
| `dopbase admin factory-reset`          | Reset the local instance offline    |

When no server is configured, client commands use `http://localhost:8840`.
`client connect` validates a new endpoint, asks for interactive confirmation,
stops the current managed background server when present, deletes the previous
encrypted CLI session, clears the saved default environment, and then saves the
new endpoint. A foreground server must first be stopped with Ctrl+C. It does
not select a project or environment, stop remote or unrelated local servers,
or revoke browser sessions.

## Server lifecycle

`server start` stays attached to the terminal and stops with Ctrl+C. On macOS
and Linux, `server up` starts a managed background process. Use `server down`
to stop it, `server status` to inspect it, and `server logs` to read its output.
See [server lifecycle](./serve) for options and examples.

`server start` and `server up` accept the same launch options:

| Option                               | Purpose                                     |
| ------------------------------------ | ------------------------------------------- |
| `--config <FILE>`                    | Read a different server configuration file  |
| `--host <HOST>`                      | Bind to an IP address, default `127.0.0.1`  |
| `--port <PORT>`                      | Listen on a port, default `8840`            |
| `--public-url <URL>`                 | Set the URL clients use to reach the server |
| `--shutdown-grace-seconds <SECONDS>` | Set the request-drain timeout               |
| `--docs` / `--no-docs`               | Enable or disable API documentation         |
| `--master-key-file <FILE>`           | Read the master key from another file       |

`server down --timeout <SECONDS>` waits 10 seconds by default. `server logs`
accepts `--lines <COUNT>` (default 100), `--watch` or `-w`, and `--clean`.
`--clean --watch` clears the existing log before waiting for new output.

The old `serve`, `serve --background`, and `stop` commands are not supported.
Use `server start`, `server up`, and `server down` respectively.

## Check for updates

`dopbase update` compares the running version against the latest release tag on
GitHub. It is informational only and never modifies the binary:

```text
dopbase {{version}} is up to date (latest release {{version}}).
```

When a newer release exists, the command prints the current version, the latest
version, and the release URL. Stop every running Dopbase server, then install
the release with:

```sh
curl -fsSL https://dopbase.com/install.sh | sh
```

The command exits with status 0 whether or not an update is available, and with
status 1 when the release cannot be queried. Run it with `--json` for
automation. No other command contacts GitHub.

The server resolution order is:

1. A global `--server <url>` argument
2. `DOPBASE_URL`
3. The endpoint saved by `dopbase client connect`
4. `http://localhost:8840`

Machine runners can register a credential with `dopbase login --token`. CI
systems and deployment platforms can use `DOPBASE_TOKEN`. For `dopbase run`, an
explicit `-t <TOKEN>` or `--token <TOKEN>` takes precedence over
`DOPBASE_TOKEN`, followed by the encrypted credential saved by `login`. A saved
credential is used only when it matches the resolved server. Command-line
tokens may be visible in shell history and process inspection, so they are best
kept for one-off overrides.

`dopbase client status` displays the config path, resolved server and its source,
authentication source, locally identified credential type, login email, and
the saved default environment. It performs a short health check and reports
`connected (live)` or `offline (cache)` without failing when the server is
unavailable. It never displays token contents. See
[client configuration](./configuration) for the TOML schema and override
behavior.

## Environment references

Every environment belongs to one project, so an environment reference contains
all the context needed by secret, import, export, token, and run commands.

Commands accept these forms:

```text
env_482731                       # Immutable environment ID
payment-service/production      # Project name and environment name
prj_01JABCDEF1234/production    # Project ID and environment name
```

Use readable references interactively. Use immutable IDs in CI and deployment
configuration because an environment ID does not change when its project or
environment is renamed.

Project names are unique within one Dopbase server. Environment names are
unique within their project. IDs belong to the Dopbase server that created
them. An ID from one server cannot address a resource on another server.

## Bootstrap a project

`init` creates a project, its first environment, and its secrets in one
operation:

```bash
dopbase init
dopbase init payment-service/development --from .env
dopbase init worker/development --from secrets.json
cat secrets.yml | dopbase init storefront/development --from - --format yaml
```

Run `dopbase init` without arguments from a directory that contains `.env`.
The CLI checks the server and login, shows the number of variables it found,
and asks for a new `project/environment` target. A key-name check estimates how
many variables may be sensitive without using or printing their values. All
variables are imported.

After a successful interactive import, Dopbase asks whether to delete `.env`.
Yes is selected by default. The file is kept if it changed after Dopbase read
it, and the command never edits `.gitignore`.

Use the explicit form for scripts, stdin, JSON, YAML, TOML, or machine-readable
output. Both modes validate the complete file before changing server state.
The project, environment, and secrets are created atomically, so a failed
request cannot leave a partial project. Terminal output never includes secret
values.

## Project commands

| Command                                                   | Purpose                  |
| --------------------------------------------------------- | ------------------------ |
| `dopbase project create <PROJECT_NAME>`                   | Create an empty project  |
| `dopbase project list`                                    | List accessible projects |
| `dopbase project show <PROJECT_REF>`                      | Show project metadata    |
| `dopbase project rename <PROJECT_REF> <NEW_PROJECT_NAME>` | Rename a project         |
| `dopbase project delete <PROJECT_REF>`                    | Delete a project         |

`<PROJECT_REF>` accepts an immutable project ID or project name. Deleting a project
also deletes its environments, secrets, and scoped tokens. Dopbase shows the
affected resource counts and requires confirmation. Automation must pass
`--yes`.

## Environment commands

| Command                                                                          | Purpose                            |
| -------------------------------------------------------------------------------- | ---------------------------------- |
| `dopbase env create <PROJECT_REF/ENVIRONMENT_NAME>`                              | Create an environment              |
| `dopbase env clone <PROJECT_REF/SOURCE_ENVIRONMENT_NAME> <NEW_ENVIRONMENT_NAME>` | Clone secrets to a new environment |
| `dopbase env default <ENVIRONMENT_REF>`                                          | Set the run default                |
| `dopbase env default --clear`                                                    | Clear the run default              |
| `dopbase env list [PROJECT_REF]`                                                 | List environments                  |
| `dopbase env show <ENVIRONMENT_REF>`                                             | Show environment metadata          |
| `dopbase env rename <ENVIRONMENT_REF> <NEW_ENVIRONMENT_NAME>`                    | Rename an environment              |
| `dopbase env delete <ENVIRONMENT_REF>`                                           | Delete an environment              |

Deleting an environment also deletes its secrets and scoped tokens. The
operation requires confirmation or `--yes` and is recorded in the audit log.

Clone an environment when a local, preview, or production environment should
start with the same secrets as an existing environment:

```bash
dopbase env clone payment-service/local production
dopbase env clone storefront/staging preview-42 --yes
```

The source must use `PROJECT_REF/ENVIRONMENT_NAME`. The new environment is
created in that project, so cloning across projects or servers is not
supported. Dopbase resolves the source, checks that the new name is available,
shows the source, destination, and secret count, then asks for confirmation.
`--yes` skips this confirmation but still requires a recently authenticated
human account. Runner tokens and service accounts cannot clone environments.

Dopbase creates the destination before reading plaintext secrets. This closes
a name-conflict race without exposing source values. If export or import
fails, the CLI tries to remove the new environment and reports whether cleanup
succeeded. Secret values remain in memory and are not written to a temporary
file.

A successful clone prints every environment in the project. With `--json`, it
returns the same environment array as `dopbase env list <PROJECT_REF> --json`.
The clone copies current secret key/value pairs only. It does not copy editor
layout, tokens, audit history, IDs, or timestamps, and it does not keep the two
environments synchronized.

## Secret commands

| Command                                               | Purpose                     |
| ----------------------------------------------------- | --------------------------- |
| `dopbase secret list <ENVIRONMENT_REF>`               | List keys and safe metadata |
| `dopbase secret set <ENVIRONMENT_REF> <KEY>`          | Create or update a value    |
| `dopbase secret get <ENVIRONMENT_REF> <KEY>`          | Read safe metadata          |
| `dopbase secret get <ENVIRONMENT_REF> <KEY> --reveal` | Explicitly reveal a value   |
| `dopbase secret delete <ENVIRONMENT_REF> <KEY>`       | Delete one secret           |

`secret set` securely prompts for a value when attached to a terminal. Use
`--stdin` for automation:

```bash
printf '%s' "$NEW_DATABASE_URL" | \
  dopbase secret set payment-service/staging DATABASE_URL --stdin
```

Dopbase does not provide a plaintext `--value` argument. `--reveal` requires
interactive password confirmation on every invocation and cannot be bypassed
with `--json`, stdin, or a confirmation flag. Reveal and deletion operations
are explicit, permission-controlled, and audited. Normal human and JSON output
never contains a value.

## Import and export

Import into an existing environment with:

```bash
dopbase import payment-service/staging .env.staging
dopbase import payment-service/staging secrets.json
dopbase import payment-service/staging secrets.toml
cat secrets.yml | dopbase import payment-service/staging - --format yaml
```

Import merges by default: keys in the file are created or updated, while
remote keys absent from the file remain unchanged. `--dry-run` reports the
number of additions, updates, unchanged values, and deletions without changing
server state.

Use `--replace` to make the remote environment match the file exactly. Replace
shows the keys that would be deleted and requires confirmation. Non-interactive
use also requires `--yes`.

The CLI infers JSON from `.json`, YAML from `.yaml` or `.yml`, and TOML from
`.toml`. Every other filename defaults to dotenv, which keeps names such as
`.env.production` compatible. Pass `--format <dotenv|json|yaml|toml>` to
override inference. Stdin uses `-` and always requires `--format`.

JSON, YAML, and TOML input must be one flat mapping of string keys to string
values. Nested objects or tables, arrays, numbers, booleans, dates, null
values, duplicate keys, and empty keys fail the complete import. Dotenv does
not expand variables or run command substitutions. All formats allow empty
string values but must contain at least one secret. Errors may name a key, but
never print its value.

Export requires an explicit destination:

```bash
dopbase export payment-service/staging --output .env.staging
dopbase export payment-service/staging --output secrets.json
dopbase export payment-service/staging --output secrets.toml
dopbase export payment-service/staging --stdout --format yaml
```

To inject secrets into an existing container, use Docker format:

```bash
dopbase export payment-service/staging --stdout --format docker |
  docker exec \
    --env-file=/dev/stdin \
    --user node \
    -w /workspace \
    container-name \
    node script.js
```

`--output` and `--stdout` are mutually exclusive. File output infers its format
from the filename, while stdout defaults to dotenv. `--format` overrides either
default. Docker format is selected explicitly with `--format docker`; it is not
available for import or init. All output formats sort keys for stable exports.
Global `--json` controls command status output and cannot be combined with
plaintext `--stdout`.

Docker env files use raw `KEY=value` lines. Spaces and extra `=` characters are
part of the value, while quotes are also literal characters. Docker format
therefore does not quote or escape values. It rejects secrets containing line
breaks or NUL bytes because Docker env files cannot represent them safely. Use
a mounted secret file for multiline certificates and private keys.

Place Docker's `--env-file` option before the container name. The example does
not use `-i` because Docker consumes stdin as the env file. If the program also
needs stdin, export to a temporary private file, pass its path to `--env-file`,
and remove the file after the command exits.

File export refuses to overwrite an existing path unless `--force` is passed
and creates the file with restrictive permissions where the platform supports
them. Export and stdout reveal plaintext values and therefore require reveal
permission and create an audit event. The CLI also requires interactive
password confirmation for every export. Non-interactive export is rejected.

The Admin UI provides the same workflow for dotenv files, with a visual review
and dry-run summary before anything is stored. See
[import and export](/ui/import-export).

## Runner tokens

Create a different token for each deployed environment:

```bash
dopbase token create payment-service/production \
  --name production-server --role runner
```

| Command                                                              | Purpose               |
| -------------------------------------------------------------------- | --------------------- |
| `dopbase token create <ENVIRONMENT_REF> --name <NAME> --role runner` | Create a runner token |
| `dopbase token list <ENVIRONMENT_REF>`                               | List token metadata   |
| `dopbase token revoke <token-id>`                                    | Revoke a token        |

The plaintext token is displayed only once. Tokens can also be created and
revoked on an environment's Tokens tab in the [Admin UI](/ui/projects-environments).
A runner token can retrieve values
for its assigned environment so `dopbase run` can inject them, but it cannot
modify, export, or access another environment.

## Run a process

Pass the environment directly:

```bash
dopbase run payment-service/development -- npm run dev
```

On an application server, save its runner token once:

```bash
printf '%s' "$RUNNER_TOKEN" | dopbase login --token
dopbase run payment-service/production -- npm start
```

Use an explicit token only for a one-off command:

```bash
dopbase run payment-service/production -t "$RUNNER_TOKEN" -- npm start
```

Automation may set `DOPBASE_ENV` instead:

```bash
DOPBASE_ENV=env_482731 dopbase run -- ./payment-service
```

Interactive users may save a server-scoped default:

```bash
dopbase env default payment-service/development
dopbase run -- npm run dev
```

An explicit positional environment takes precedence over `DOPBASE_ENV`, which
takes precedence over the saved default. If none is available, `run` explains
how to set the default. Use `dopbase env default --clear` to remove it. An empty
`DOPBASE_ENV` is an error and does not fall back.

Before starting the child, Dopbase writes the resolved project, environment,
immutable ID, loaded key count, and whether values came from the live server or
encrypted cache to standard error. It does not print values.

Managed values override same-named variables inherited from the parent process.
Dopbase authentication variables are removed from the child environment so the
child application does not receive the credential used to contact Dopbase.

Every successful live retrieval refreshes the credential-bound encrypted cache.
Connection failures, timeouts, and 5xx responses fall back to the latest
matching cache after a five-second live-fetch deadline.

Cached entries do not expire, so the warning includes their fetch time and age. Authentication,
authorization, not-found, malformed-response, missing-cache, or cache-integrity
failures stop before the child starts. Once started, signals are forwarded and
the child's exit status is returned to the calling shell.

## Runtime cache maintenance

Inspect safe metadata for the active server's encrypted runtime cache:

```bash
dopbase cache list
dopbase cache list --json
```

The output includes the server URL, project and environment names, immutable
environment ID, fetch time, age, and aliases. It never includes secret names,
values, cache keys, nonces, or ciphertext.

Remove entries older than 14 days:

```bash
dopbase cache clean --dry-run
dopbase cache clean --yes
```

Set a different age with a positive whole number followed by `s`, `m`, `h`, or
`d`. An entry must be strictly older than the cutoff, so an entry exactly 14
days old is kept by the default cleanup.

```bash
dopbase cache clean --older-than 30d --yes
dopbase cache clean --all --yes
```

Only `--all` ignores age. Cleanup asks for confirmation unless `--yes` is
present. `--dry-run` never writes to the cache and does not ask for
confirmation. These commands work offline, but the active credential must be
able to unlock the cache.

## backup

Create an encrypted system-level snapshot containing all projects, environments,
secrets, runner tokens, and administrator credentials.

```bash
# Create a timestamped backup stored on the server
dopbase backup

# Create a named backup and save a copy locally
dopbase backup pre-upgrade --output ./pre-upgrade.dop

# Run with structured JSON output
dopbase backup pre-upgrade --json
```

| Argument / Flag       | Description                                                   |
| --------------------- | ------------------------------------------------------------- |
| `[name]`              | Optional name prefix (default: `dopbase_backup_<timestamp>`)  |
| `-o, --output <path>` | Download a copy of the encrypted archive to a local file path |
| `--json`              | Format results as machine-readable JSON                       |

### Operation flow

1. Dopbase contacts the server and verifies that the instance is running and reachable (`server_status: connected (live)`).
2. The server acquires an online SQLite snapshot lock, generates `dopbase.db` and `manifest.json`, and encrypts the ZIP container using **XChaCha20-Poly1305** with the server's master key (`~/.dopbase/master.key`).
3. The encrypted `.dop` archive is stored on the server under `~/.dopbase/backups/`.
4. If `--output` was specified, the CLI streams and downloads the archive to your local device.
5. The CLI prints a confirmation and a security notice reminding you that the backup is encrypted with this instance's master key and that the master key will be required if restoring onto a new server.

> [!IMPORTANT]
> `dopbase backup` requires an active, reachable server (`server_status: connected (live)`).
> If the server is offline or disconnected, the command halts immediately.

## restore

Restore all database tables and instance configuration from a `.dop` backup archive.

```bash
# Restore on the same server (uses server's existing master key)
dopbase restore ./pre-upgrade.dop

# Restore on a new or different server (Dual-input restore with master.key file)
dopbase restore ./pre-upgrade.dop --key /path/to/source/master.key

# Restore on a new server using a 64-character hex master key
dopbase restore ./pre-upgrade.dop --key 4a2f8b9c01234567...

# First-run restore requires the setup token printed by the target server
dopbase restore ./pre-upgrade.dop --setup-token dbs_... --yes
```

| Argument / Flag         | Description                                                                                         |
| ----------------------- | --------------------------------------------------------------------------------------------------- |
| `<path>`                | Path to the local `.dop` backup file to restore                                                     |
| `-k, --key <KEY>`       | Path to source server's `master.key` file or a 64-character hex string                              |
| `--yes`                 | Skip the destructive confirmation prompt (initialized restores still require password confirmation) |
| `--setup-token <TOKEN>` | One-time token required for first-run restore                                                       |
| `--json`                | Format results as machine-readable JSON                                                             |

### Operation flow

- **On a live, initialized server**:
  1. Verifies server status is live (`connected (live)`).
  2. Uploads the `.dop` archive to the server.
  3. Prompts for interactive confirmation (requiring typing `RESTORE`) unless `--yes` is specified.
  4. Decrypts and authenticates the archive using the provided `--key` (or the server's existing master key).
  5. If `--key` was provided, re-keys restored secret metadata to the target server's existing `~/.dopbase/master.key`.
  6. Replaces SQLite database tables, runs any pending migrations, and preserves the active administrator session.
- **On a first-run uninitialized server**:
  - Restoring requires the one-time first-run setup token.

> [!IMPORTANT]
> `dopbase restore` strictly requires the server to be live and connected (`server_status: connected (live)`).
> Offline restores are rejected to prevent file system races and corruption.

## Server administration

Offline maintenance commands operate directly on the server database:

```bash
dopbase admin reset-password admin@example.com
dopbase admin factory-reset
dopbase admin factory-reset --no-backup
```

`admin reset-password` performs offline recovery on the host machine. Because it accesses SQLite files directly, the background or foreground server process must be stopped first to avoid database locks.

| Option                     | Description                                       |
| -------------------------- | ------------------------------------------------- |
| `--config <FILE>`          | Server configuration file to load (`server.toml`) |
| `--master-key-file <FILE>` | Path to the server master key file                |

The command requires an interactive terminal, prompts for a new password (12 to 128 characters), updates the stored Argon2id hash, and immediately revokes all active sessions for that account.

`admin factory-reset` also requires the server to be stopped. It displays the
exact data directory, then requires `please-wipe-out-system` and the Dopbase
root password. There is no non-interactive bypass.

By default, the reset saves the whole data directory to a timestamped ZIP next
to that directory, then removes the active data. The ZIP includes every
database, master-key, configuration, backup, log, and local CLI file stored
inside the directory. The next server start creates a fresh installation.

Pass `--no-backup` to remove the data directory without creating the ZIP. The
same confirmation phrase and root password are still required.

## Structured output

Resource and metadata commands support `--json` for automation. Secret
values remain excluded unless the user explicitly selected `--reveal`,
`--stdout`, or an export destination. Errors identify the server, project,
environment, and operation when safe without including request bodies, tokens,
or plaintext values.
