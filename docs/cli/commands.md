---
title: "Command reference"
description: "Reference for every Dopbase CLI command in v0.0.13: connections, authentication, projects, environments, and secret operations."
---

# Command reference

This page defines the implemented v0.0.13 command surface. Dopbase does not infer a
project or environment from the current directory. Management commands receive
an environment reference directly; `run` may use a server-scoped default from
the user configuration.

## Connections and authentication

| Command                               | Purpose                             |
| ------------------------------------- | ----------------------------------- |
| `dopbase serve`                       | Start a self-hosted server          |
| `dopbase serve --background`          | Start the server as a daemon        |
| `dopbase stop`                        | Stop the background server          |
| `dopbase client connect <server-url>` | Validate and save another server    |
| `dopbase client connect local`        | Return to the implicit local server |
| `dopbase login`                       | Authenticate with the active server |
| `dopbase logout`                      | Remove the active saved credential  |
| `dopbase status`                      | Show safe effective client settings |
| `dopbase update`                      | Check GitHub for a newer release    |

When no server is configured, client commands use `http://localhost:8840`.
`client connect` validates a new endpoint, asks for interactive confirmation,
stops the current managed background server when present, deletes the previous
encrypted CLI session, clears the saved default environment, and then saves the
new endpoint. A foreground server must first be stopped with Ctrl+C. It does
not select a project or environment, stop remote or unrelated local servers,
or revoke browser sessions.

## Server lifecycle

`serve --background` and `stop` manage a detached server; see
[serve](./serve#run-in-the-background) for PID and log file locations.
`stop --timeout <seconds>` extends the grace period before the daemon is
force-stopped (default 10).

## Check for updates

`dopbase update` compares the running version against the latest release tag on
GitHub. It is informational only and never modifies the binary:

```text
dopbase 0.0.13 is up to date (latest release 0.1.0).
```

When a newer release exists, the command prints the current version, the latest
version, the release URL, and a reminder that Dopbase does not self-update.
Install the new release with `scripts/install.sh`, `scripts/install.ps1`, or the release archive. The
command exits with status 0 whether or not an update is available, and with
status 1 when the release cannot be queried. Run it with `--json` for
automation; no other command contacts GitHub.

The server resolution order is:

1. A global `--server <url>` argument
2. `DOPBASE_URL`
3. The endpoint saved by `dopbase client connect`
4. `http://localhost:8840`

Machine authentication uses `DOPBASE_TOKEN` in preference to a token saved by
`login` in the encrypted local session file. A saved credential is used
only when it matches the resolved server. Dopbase will not accept a token as a
CLI argument because command-line arguments can be exposed through shell
history and process inspection.

`dopbase status` displays the config path, resolved server and its source,
authentication source, cached identity, login email, and the saved default
environment. It performs a short health check and reports `connected (live)`
or `offline (cache)` without failing when the server is unavailable. It never
displays token contents. See
[client configuration](./configuration) for the TOML schema and override
behavior.

## Environment references

Every environment belongs to one project, so an environment reference contains
all the context needed by secret, import, export, token, and run commands.

Commands accept either form:

```text
env_01ABCDEF...             # Immutable ID
payment-service/production # Readable reference
```

Use readable references interactively. Use immutable IDs in CI and deployment
configuration because an environment ID does not change when its project or
environment is renamed.

Project names are unique within one Dopbase server. Environment names are
unique within their project. IDs belong to the Dopbase server that created
them; an ID from one server cannot address a resource on another server.

## Bootstrap a project

`init` creates a project, its first environment, and its secrets in one
operation:

```bash
dopbase init payment-service development --from .env
```

The command validates the complete file before changing server state. The
project, environment, and imported secrets are then created atomically. If the
project name already exists or any entry is invalid, the command fails without
leaving a partially created project.

`init` does not create or modify a file in the application repository. After a
successful import it prints the new project ID, environment ID, and secret
count, but never secret values.

## Project commands

| Command                                       | Purpose                  |
| --------------------------------------------- | ------------------------ |
| `dopbase project create <name>`               | Create an empty project  |
| `dopbase project list`                        | List accessible projects |
| `dopbase project show <project>`              | Show project metadata    |
| `dopbase project rename <project> <new-name>` | Rename a project         |
| `dopbase project delete <project>`            | Delete a project         |

`<project>` accepts an immutable project ID or project name. Deleting a project
also deletes its environments, secrets, and scoped tokens. Dopbase shows the
affected resource counts and requires confirmation; automation must pass
`--yes`.

## Environment commands

| Command                                       | Purpose                   |
| --------------------------------------------- | ------------------------- |
| `dopbase env create <project> <name>`         | Create an environment     |
| `dopbase env default <environment>`           | Set the run default       |
| `dopbase env default --clear`                 | Clear the run default     |
| `dopbase env list [<project>]`                | List environments         |
| `dopbase env show <environment>`              | Show environment metadata |
| `dopbase env rename <environment> <new-name>` | Rename an environment     |
| `dopbase env delete <environment>`            | Delete an environment     |

Deleting an environment also deletes its secrets and scoped tokens. The
operation requires confirmation or `--yes` and is recorded in the audit log.

## Secret commands

| Command                                           | Purpose                     |
| ------------------------------------------------- | --------------------------- |
| `dopbase secret list <environment>`               | List keys and safe metadata |
| `dopbase secret set <environment> <key>`          | Create or update a value    |
| `dopbase secret get <environment> <key>`          | Read safe metadata          |
| `dopbase secret get <environment> <key> --reveal` | Explicitly reveal a value   |
| `dopbase secret delete <environment> <key>`       | Delete one secret           |

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
```

Import merges by default: keys in the file are created or updated, while
remote keys absent from the file remain unchanged. `--dry-run` reports the
number of additions, updates, unchanged values, and deletions without changing
server state.

Use `--replace` to make the remote environment match the file exactly. Replace
shows the keys that would be deleted and requires confirmation; non-interactive
use also requires `--yes`.

The complete file is parsed and validated before any secret changes are made.
Blank lines and comments are ignored, empty values are valid, and variable or
command substitution is not performed. Duplicate or invalid keys fail the
entire import. Output includes key names and counts when useful, but never
values.

Export requires an explicit destination:

```bash
dopbase export payment-service/staging --output .env.staging
dopbase export payment-service/staging --stdout
```

`--output` and `--stdout` are mutually exclusive. File export refuses to
overwrite an existing path unless `--force` is passed and creates the file with
restrictive permissions where the platform supports them. Export and stdout
reveal plaintext values and therefore require reveal permission and create an
audit event. The CLI also requires interactive password confirmation for every
export. Non-interactive export is intentionally rejected.

The same workflows exist in the Admin UI, with a visual review and dry-run
summary before anything is stored; see [import and export](/ui/import-export).

## Runner tokens

Create a different token for each deployed environment:

```bash
dopbase token create payment-service/production \
  --name production-server --role runner
```

| Command                                                          | Purpose               |
| ---------------------------------------------------------------- | --------------------- |
| `dopbase token create <environment> --name <name> --role runner` | Create a runner token |
| `dopbase token list <environment>`                               | List token metadata   |
| `dopbase token revoke <token-id>`                                | Revoke a token        |

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

Automation may set `DOPBASE_ENV` instead:

```bash
DOPBASE_ENV=env_01ABCDEF dopbase run -- ./payment-service
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
matching cache after a five-second live-fetch deadline. Cached entries do not
expire, so the warning includes their fetch time and age. Authentication,
authorization, not-found, malformed-response, missing-cache, or cache-integrity
failures stop before the child starts. Once started, signals are forwarded and
the child's exit status is returned to the calling shell.

## Structured output

Resource and metadata commands support `--json` for automation. Secret
values remain excluded unless the user explicitly selected `--reveal`,
`--stdout`, or an export destination. Errors identify the server, project,
environment, and operation when safe without including request bodies, tokens,
or plaintext values.
