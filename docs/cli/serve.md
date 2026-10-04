---
title: "Server lifecycle"
description: "Start, stop, restart, inspect, and read logs from a local Dopbase server."
---

# Server lifecycle

The `dopbase server` commands manage the self-hosted HTTP server, REST API,
SQLite storage, and Admin UI.

## Initialize before starting

A fresh or reset instance requires `dopbase server setup` before startup.
`server start` and `server start --background` show an informational notice and
exit with status 1 when storage is uninitialized. The notice lists setup methods,
the selected data and configuration paths, and configuration guidance. Startup
does not create a database, master key, or configuration references for a fresh
instance. Existing initialized installations start as usual.

With `--json`, background startup writes the notice to stdout with
`success = false` and `info.code = "SETUP_REQUIRED"`. The `info` object also
contains `message`, `data_dir`, and `config_file`. This replaces the previous
`error.SETUP_REQUIRED` response on stderr. Other startup failures still report
errors on stderr. Foreground startup does not support `--json`.

See [server setup](./setup) for guided setup, generated passwords, and the
existing web flow. `server setup --web` continues serving after setup until
stopped with Ctrl+C.

## Configuration files

CLI setup creates missing configuration files before creating root. Web setup
and startup of an initialized instance create missing references once the
listener binds successfully:

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
| `--docs`                             | Enable Swagger UI and the OpenAPI document   |
| `--no-docs`                          | Disable API documentation for this run       |
| `--master-key-file <FILE>`           | Read the server master key from another file |

Startup prints stop and restart commands with the selected absolute data
directory quoted for the shell. JSON output includes these as `stop_command`
and `restart_command`. Use the printed commands to manage that instance from
another working directory.

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
docs = false
```

## API documentation

Swagger UI and the OpenAPI document are disabled by default. Enable them for
one run with:

```bash
dopbase server start --docs
```

You can also set `docs = true` in `server.toml` or `DOPBASE_DOCS=true`.

Migrations run before the listener opens. Explicit setup creates a random master key with owner-only permissions for
new installations. During shutdown, Dopbase stops new
requests, drains active requests, checkpoints SQLite, and closes the database.
