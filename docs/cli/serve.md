---
title: "dopbase serve"
description: "dopbase serve starts the self-hosted HTTP server, REST API, SQLite storage, and embedded Admin UI in a single command."
---

# `dopbase serve`

`dopbase serve` starts the self-hosted HTTP server, REST API, SQLite storage,
and embedded Admin UI. The Swagger UI is disabled by default and enabled with
`--docs` (see [API documentation](#api-documentation)).

```bash
dopbase serve
```

The local defaults are:

```text
Admin UI:   http://localhost:8840
API:        http://localhost:8840/api/v1
Config:     ~/.dopbase/server.toml
```

The `Swagger: http://localhost:8840/api/docs` line is printed only when the
API documentation is enabled (with `--docs` or `docs = true` in server.toml).

## Server configuration

A basic configuration only needs a port. Dopbase derives the remaining local settings:

```toml
version = 1
port = 8840
```

With no `public_url` configured, a loopback server derives it from the port
(`http://localhost:8840`). The same applies on the command line:

```bash
dopbase serve --port 8840
```

To expose the server beyond localhost, set a host and provide its public
address. Dopbase does not derive the public URL from the `Host` header:

```bash
dopbase serve --host 0.0.0.0 --public-url https://dopbase.example.com
```

```toml
version = 1
host = "0.0.0.0"
port = 8840
public_url = "https://dopbase.example.com"
```

Advanced deployments (reverse proxies, Docker port mappings, path prefixes)
that need the raw socket can use `bind_address` instead of `port`/`host`.
The two styles cannot be mixed:

```toml
version = 1
bind_address = "127.0.0.1:8840"
public_url = "https://dopbase.example.com"
```

Individual settings resolve from command option, matching environment variable,
`server.toml`, then the default derived from the selected data directory.
Supported overrides are `--config`, `--port`, `--host`, `--public-url`,
`--bind-address`, `--database-url`, `--shutdown-grace-seconds`, `--docs`/`--no-docs`,
and `--master-key-file`, with corresponding `DOPBASE_*` environment variables.

By default, all runtime files live in `~/.dopbase`. Select another directory
with the global `--data-dir <dir>` option or `DOPBASE_DATA_DIR`. Data-directory
selection resolves in this order: CLI option, environment variable, default.

`public_url` is required when binding beyond loopback. Dopbase does not trust
forwarded headers in v0.0.8, and TLS termination remains an operator concern.

## API documentation

The Swagger UI at `/api/docs` and the OpenAPI document at
`/api/v1/openapi.json` are disabled by default. Enable them for one run with
`dopbase serve --docs`, disable them again with `--no-docs`, or enable them
persistently with `docs = true` in `server.toml` or `DOPBASE_DOCS=true`. The
command-line flags override the environment variable, which overrides the
configuration file.

## Run in the background

`--background` starts the server as a detached daemon (macOS and Linux). The
command returns as soon as the server is ready; the real startup error, such as
a bind failure, is reported to the terminal:

```bash
dopbase serve --background
```

The daemon writes two files into the data directory:

```text
~/.dopbase/dopbase.pid   PID file (process ID, version, bind address)
~/.dopbase/serve.log     stdout and stderr of the server
```

The one-time setup token of an uninitialized daemon is printed by the starting
command and is also written to `serve.log`.

Stop the daemon with `dopbase stop`:

```bash
dopbase stop                    # graceful shutdown, then force after 10s
dopbase stop --timeout 30       # extend the grace period
dopbase --data-dir /tmp/x stop  # target a non-default data directory
```

`stop` sends SIGTERM, waits for the graceful shutdown described below, and
escalates to SIGKILL after the timeout. It removes stale PID files and fails
with a clear error when no daemon is running. A second `serve --background`
refuses to start while a daemon is active for the same data directory, and a
foreground `serve` fails on the database lock while any instance owns it.

## Startup and shutdown

Migrations run before the listener opens. A new database receives a random
master key with owner-only permissions. An existing database fails closed if
its configured key is missing, malformed, or incorrect.

An uninitialized server prints its one-time setup token to dedicated startup
stderr output. The token is never sent through structured request logging.

SQLite uses WAL mode so reads can continue while writes are committed. SIGINT
and SIGTERM stop new requests, drain active requests, checkpoint the WAL, and
close SQLite. Offline recovery refuses to run while the server owns the
database lock.
