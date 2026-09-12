---
title: "Server lifecycle"
description: "Start, stop, inspect, and read logs from a local Dopbase server."
---

# Server lifecycle

The `dopbase server` commands manage the self-hosted HTTP server, REST API,
SQLite storage, and Admin UI.

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

On macOS and Linux, `up` starts a managed background process and returns after
the listener is ready:

```bash
dopbase server up
dopbase server up --port 9000
```

`start` and `up` accept the same server options:

| Option                               | Purpose                                      |
| ------------------------------------ | -------------------------------------------- |
| `--config <FILE>`                    | Read a different `server.toml` file          |
| `--host <HOST>`                      | Bind to an IP address, default `127.0.0.1`   |
| `--port <PORT>`                      | Listen on a port, default `8840`             |
| `--public-url <URL>`                 | Set the URL clients use to reach the server  |
| `--shutdown-grace-seconds <SECONDS>` | Set the request-drain timeout                |
| `--docs`                             | Enable Swagger UI and the OpenAPI document   |
| `--no-docs`                          | Disable API documentation for this run       |
| `--master-key-file <FILE>`           | Read the server master key from another file |

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
dopbase server down
dopbase server down --timeout 30
```

`down` sends SIGTERM and waits up to 10 seconds by default. It uses SIGKILL if
the process does not finish before the timeout. Stop a foreground server with
Ctrl+C.

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

Migrations run before the listener opens. New installations create a random
master key with owner-only permissions. During shutdown, Dopbase stops new
requests, drains active requests, checkpoints SQLite, and closes the database.
