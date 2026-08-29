# `dopbase serve`

`dopbase serve` starts the self-hosted HTTP server, REST API, Swagger UI,
SQLite storage, and embedded Admin UI.

```bash
dopbase serve
```

The local defaults are:

```text
Admin UI:   http://localhost:8376
API:        http://localhost:8376/api/v1
Swagger:    http://localhost:8376/api/docs
Data:       ~/.dopbase
Database:   ~/.dopbase/dopbase.db
Config:     ~/.dopbase/server.toml
Master key: ~/.dopbase/master.key
```

## Server configuration

```toml
version = 1
bind_address = "127.0.0.1:8376"
public_url = "http://localhost:8376"
database_url = "sqlite:///home/alex/.dopbase/dopbase.db"
shutdown_grace_seconds = 10

[master_key]
provider = "file"
path = "~/.dopbase/master.key"
```

By default, all runtime files live in `~/.dopbase`. Select another directory
with the global `--data-dir <dir>` option or `DOPBASE_DATA_DIR`. Data-directory
selection resolves in this order: CLI option, environment variable, default.

Individual settings resolve from command option, matching environment variable,
`server.toml`, then the default derived from the selected data directory.
Supported overrides are `--config`, `--bind-address`, `--public-url`,
`--database-url`, `--shutdown-grace-seconds`, and `--master-key-file`, with
corresponding `DOPBASE_*` environment variables.

`public_url` is required when binding beyond loopback. Dopbase does not trust
forwarded headers in v0.0.1, and TLS termination remains an operator concern.

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
