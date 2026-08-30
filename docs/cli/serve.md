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

The easy path only needs a port — Dopbase derives everything else:

```toml
version = 1
port = 8840
```

With no `public_url` configured, a loopback server derives it from the port
(`http://localhost:8840`). The same applies on the command line:

```bash
dopbase serve --port 8840
```

To expose the server beyond localhost, set a host and tell Dopbase its public
address — it refuses to guess (it does not trust the `Host` header):

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
