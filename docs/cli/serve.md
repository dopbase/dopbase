---
title: "Server lifecycle"
description: "Start, stop, restart, inspect, and read logs from a local Dopbase server."
---

# Server lifecycle

The `dopbase server` commands manage the self-hosted HTTP server, REST API,
SQLite storage, and Admin UI.

## Automatic first startup

```bash
dopbase server start
```

On fresh storage, startup creates the database, master key, and configuration
examples, then prints the existing protected web setup link. Open it to create
root or restore a backup. The server keeps running after setup. An initialized
instance starts normally without changing its accounts or credentials.

To create root without opening a browser, supply an email:

```bash
DOPBASE_ROOT_EMAIL=admin@example.com dopbase server start
```

Dopbase generates a password, displays it once, and continues serving. Save the
password, then sign in through the Admin UI or `dopbase login`. Account creation
does not sign you in. Existing instances ignore this input, including invalid
values. Missing or blank values select web setup on fresh storage; invalid
nonblank values fail before creating files.

Foreground and background startup use the same rules. Automatic setup respects
`--no-web-ui`, `DOPBASE_WEB_UI=false`, and `web_ui = false`. With the UI disabled,
supply `DOPBASE_ROOT_EMAIL` or complete explicit CLI setup first. A pending
factory reset still requires explicit setup; automatic startup does not finish
an interrupted reset. Storage and master-key errors never fall back to setup.

Explicit setup remains available when you want terminal prompts, standalone
provisioning, or the web flow with an email prefilled.

For background startup with a root email:

```bash
DOPBASE_ROOT_EMAIL=admin@example.com dopbase --json server start --background
```

The launching command returns the password once. JSON keeps the normal startup
fields and adds `setup` only when this start created root:

```json
{
  "setup": {
    "initialized": true,
    "admin_id": "<root-account-id>",
    "email": "admin@example.com",
    "data_dir": "/srv/dopbase",
    "password": "<generated-password>"
  }
}
```

This excerpt omits the unchanged startup fields. Later starts and restarts omit
`setup`. The managed daemon log and PID metadata do not contain the password.
Foreground output captured by Docker, redirected files, or CI may retain it.

If credential delivery fails after account creation, Dopbase stops the new
server and reports that initialization committed. Background JSON errors include
`initialized: true`. Recover with `dopbase admin reset-password EMAIL` using the
same instance options; another start will not generate replacement credentials.

With a disabled UI and no root email, background `--json` writes an informational
notice to stdout with `success = false`, `info.code = "SETUP_REQUIRED"`,
`message`, `data_dir`, and `config_file`. Other startup failures use stderr.
Foreground startup does not support `--json`.

See [server setup](./setup) for the unchanged explicit setup commands.

## Configuration files

CLI setup creates missing configuration files before creating root. Automatic
startup and web setup create missing references after the listener binds:

- `~/.dopbase/server.toml` describes every server setting, including defaults,
  examples, accepted values, and environment overrides.
- `~/.dopbase/config.toml` describes client connection and default environment
  settings.

All generated settings and table headers are commented out. Uncomment the
settings you need, including the table header for nested settings. Restart
the server after changing `server.toml`.

Setup and startup leave existing files untouched. CLI options and environment variables
apply to the current run and are not saved into the examples. `--data-dir` or
`DOPBASE_DATA_DIR` relocates both files; `--config <FILE>` selects a different
server config path while the client config stays in the data directory.

## Disable the web UI

The web UI is enabled by default. To use only the CLI and REST API, set this
in `server.toml`:

```toml
web_ui = false
```

You can also disable it for a server run:

```bash
DOPBASE_WEB_UI=false dopbase server start
dopbase server start --no-web-ui
dopbase server start --background --no-web-ui
```

Priority is `--no-web-ui`, then `DOPBASE_WEB_UI`, then `web_ui` in
`server.toml`, then the default `true`. File and environment values accept
`true` or `false`. Restart after changing the saved setting. Background restart
keeps the original CLI and environment overrides while rereading the file.

When disabled, UI pages and bundled assets return a JSON 404 response. The
CLI and REST API remain available. Swagger and OpenAPI still use the separate
`docs` setting. Startup displays `Admin UI: disabled`.

`server setup --web` explicitly enables the UI for that entire run, including
after setup finishes. It leaves the saved setting unchanged; the next normal
start follows configuration.

## Run in the foreground

Use `start` while developing or when another process manager handles Dopbase:

```bash
dopbase server start
```

The command stays attached to the terminal. Press Ctrl+C to stop it.

The default listener is `127.0.0.1:8840`. Change the host or port separately:

```bash
dopbase server start --port 9000
dopbase server start --host 0.0.0.0
dopbase server start \
  --host 0.0.0.0 \
  --public-url https://dopbase.example.com
```

When the server binds to a non-loopback address without a public URL, Dopbase
shows `http://SERVER_HOST:<port>`. Replace `SERVER_HOST` with the server's
public IP address or domain. Dopbase does not inspect network routes or call an
external service to discover the address.

Set the public URL to make generated links ready to use:

```bash
dopbase server start \
  --host 0.0.0.0 \
  --public-url http://203.0.113.10:8840
dopbase server start --public-url https://dopbase.example.com
export DOPBASE_PUBLIC_URL=https://dopbase.example.com
```

You can also set `public_url = "https://dopbase.example.com"` in `server.toml`.
Public URLs may use an IP address or domain. Dopbase warns when a remote URL
uses HTTP because credentials, secrets, and setup tokens are not encrypted.
Use HTTPS for deployments. Dopbase never derives its public URL from request
headers.

## Run in the background

On macOS and Linux, `start --background` starts a managed background process and returns after
the listener is ready:

```bash
dopbase server start --background
dopbase server start --background --port 9000
dopbase server start -b
```

`start` accepts these options in either mode:

| Option                               | Purpose                                      |
| ------------------------------------ | -------------------------------------------- |
| `--background`, `-b`                 | Run a managed background server              |
| `--config <FILE>`                    | Read a different `server.toml` file          |
| `--host <HOST>`                      | Bind to an IP address, default `127.0.0.1`   |
| `--port <PORT>`                      | Listen on a port, default `8840`             |
| `--public-url <URL>`                 | Set the URL clients use to reach the server  |
| `--shutdown-grace-seconds <SECONDS>` | Set the request-drain timeout                |
| `--no-web-ui`                        | Disable the web UI for this run              |
| `--docs`                             | Enable Swagger UI and the OpenAPI document   |
| `--no-docs`                          | Disable API documentation for this run       |
| `--master-key-file <FILE>`           | Read the server master key from another file |

Startup prints `dopbase server stop` and `dopbase server restart`. JSON output
includes these as `stop_command` and `restart_command`. If you use a custom data
directory, add the same `--data-dir` option when managing that instance.

Only one server can use a data directory at a time.

## Check status

`server status` inspects the local server selected by `--data-dir`:

```bash
dopbase server status
dopbase --data-dir /srv/dopbase server status
```

It reports whether the server is stopped or running in the foreground or
background. The command exits with status 0 when the server is running and 1
when it is stopped.

Use `dopbase client status` to inspect the active client endpoint, login, and
default environment. `dopbase status` is an alias for that client command.

## Read background logs

Print the last 100 lines:

```bash
dopbase server logs
```

Choose a line count or watch new output:

```bash
dopbase server logs --lines 50
dopbase server logs --watch
```

Clear existing output and leave the log ready for new entries:

```bash
dopbase server logs --clean
dopbase server logs --clean --watch
```

When combined with `--watch`, `--clean` waits for output written after the
log was cleared.

The log remains available after the background server stops. A foreground
server writes to its attached terminal instead.

## Stop the background server

```bash
dopbase server stop
dopbase server stop --timeout 30
```

`stop` sends SIGTERM and waits up to 10 seconds by default. It uses SIGKILL if
the process does not finish before the timeout. Stop a foreground server with
Ctrl+C.

## Restart the background server

```bash
dopbase server restart
dopbase server restart --timeout 30
dopbase server restart --json
```

`restart` requires a running managed background server. It preserves the original
CLI options and server environment overrides, rereads the configuration file,
and starts the replacement in the background. Saved overrides still take
precedence over file settings. Relative paths use the original working directory.
To change launch options or environment overrides, stop the server and start it again.

Dopbase validates configuration before stopping the server and returns only when
the replacement is ready. If configuration is invalid, the existing server keeps
running. If startup fails after shutdown, the command reports the failure and
points to the background log. There is a brief interruption during restart.

The private PID file stores server launch settings and paths for restart. It does
not store master-key contents, credentials, or the full process environment.
A server started before this metadata was introduced must be stopped and started
once with the updated binary before `restart` can be used.

If the server is stopped, use `server start --background`. Stop a foreground
server with Ctrl+C. Background lifecycle commands are supported on macOS and Linux.

## Migrating from up and down

| Previous command                   | Replacement                                                      |
| ---------------------------------- | ---------------------------------------------------------------- |
| `dopbase server up`                | `dopbase server start --background` or `dopbase server start -b` |
| `dopbase server down`              | `dopbase server stop`                                            |
| `dopbase server down --timeout 30` | `dopbase server stop --timeout 30`                               |

`up` and `down` have been replaced. They show a blue `Info:` notice in terminals
that support color and exit with status 1 without starting or stopping a server.
With `--json`, they write an informational payload to stdout with
`info.code = "COMMAND_REPLACED"`, the replacement command, and `success = false`.
Update scripts to use the replacement commands.

## Storage and configuration

Runtime files live in `~/.dopbase` by default. Select another directory with
the global `--data-dir <DIR>` option or `DOPBASE_DATA_DIR`.

The database location is fixed:

```text
<data-dir>/dopbase.db
```

`database_url`, `DOPBASE_DATABASE_URL`, and `--database-url` are ignored.
If an older installation points to another SQLite file, stop the server and
move that file to `<data-dir>/dopbase.db` before upgrading.

Configure the listener with `host` and `port`. The old `bind_address`,
`DOPBASE_BIND_ADDRESS`, and `--bind-address` settings are ignored.

```toml
version = 1
host = "127.0.0.1"
port = 8840
web_ui = true
docs = false
```

## API documentation

Swagger UI and the OpenAPI document are disabled by default. Enable them for
one run with:

```bash
dopbase server start --docs
```

You can also set `docs = true` in `server.toml` or `DOPBASE_DOCS=true`.

Migrations run before HTTP requests are served. Setup and automatic startup
create a random master key with owner-only permissions for new installations.
During shutdown, Dopbase stops new
requests, drains active requests, checkpoints SQLite, and closes the database.
