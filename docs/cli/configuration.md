---
title: "Client configuration"
description: "How the Dopbase CLI stores machine-global connection state and resolves the effective server for client commands."
---

# Client configuration

Dopbase keeps client connection state and an optional run default in a
machine-global configuration. It does not create configuration inside an
application repository.

## Default local server

When no server is configured, the CLI uses:

```text
http://localhost:8840
```

This default is implicit. Dopbase does not need to create a configuration file
just to connect to a server started with the default `dopbase serve` settings.

If a configured remote server is unavailable, Dopbase fails clearly. It never
falls back to localhost, Cloud, or another endpoint.

## Configuration file

The per-user locations are:

```text
Unix and macOS: ~/.dopbase/config.toml
Windows:        %USERPROFILE%\.dopbase\config.toml
```

Use the global `--data-dir <dir>` option or `DOPBASE_DATA_DIR` to relocate this
file together with the default server data. The CLI option takes precedence
over the environment variable.

The v0.0.14 schema contains only non-secret client state:

```toml
version = 1
server_url = "https://dopbase.example.com"

[default_environment]
server_url = "https://dopbase.example.com"
environment_id = "env_01ABCDEF"
```

`server_url` is omitted when the implicit local server is active. Dopbase
writes configuration atomically and restricts file permissions to the current
user where the platform supports them.

The optional default contains only an immutable environment ID and the server
URL that owns it. The file never stores secret values or authentication tokens.

## Select a server

Select a self-hosted or Cloud endpoint with:

```bash
dopbase client connect https://dopbase.example.com
```

`connect` accepts an absolute HTTP or HTTPS URL. It normalizes the endpoint,
rejects embedded credentials, query strings, and fragments, then verifies that
the endpoint is a compatible Dopbase server. The new server is saved only after
validation succeeds. A failed connection leaves the previous configuration and
credential unchanged.

After validation, Dopbase asks for confirmation. A successful switch stops the
current managed background server when present, removes the previous encrypted
CLI session and key, and clears the previous default environment. A foreground
server must be stopped with Ctrl+C first. Remote and unrelated local servers
are not stopped. Authenticate separately against the new endpoint:

```bash
dopbase login
```

Return to the implicit local server with:

```bash
dopbase client connect local
dopbase login
```

The `local` alias resolves to `http://localhost:8840`. Dopbase validates the
local server, removes the `server_url` override, and removes the previous
server's credential.

Persistent switching is rejected while `DOPBASE_URL` is set because that
environment variable would override the newly saved endpoint.

## Credentials

`dopbase login` stores the resulting token in an encrypted extensionless
`session` file beside `config.toml`, scoped to the normalized server URL. Its
random 32-byte key is stored separately in `session-key`. The token is never
written to `config.toml` or a Dopbase server's SQLite database.

The encrypted payload also caches the normalized administrator email so
`dopbase status` can identify the login without contacting the server. Older
sessions remain valid but show an unknown email until the next login.

Only one saved connection is active in v0.0.14. Logging in again replaces the
credential for that server. `dopbase logout` removes the active credential but
leaves the selected server unchanged.

On Unix, the data directory is mode `0700` and both session files are mode
`0600`. On Windows, they inherit the user profile directory ACL. Separating the
key protects a copied `session` file by itself, but an attacker that can read
both files can decrypt the token. Use `DOPBASE_TOKEN` when the credential must
be managed externally.

## Encrypted run cache

`dopbase run` stores its latest successfully fetched runtime environments under
`run-cache/` beside `config.toml`. Cache payloads use authenticated
XChaCha20-Poly1305 encryption and an independent random key in `run-cache-key`.
Server-derived cache filenames and lock files contain no secret values; cache
files and the key are mode `0600` inside mode `0700` directories on Unix.

The encryption key for each server cache is derived from the local cache key,
the normalized server URL, and the exact active session or `DOPBASE_TOKEN`.
Consequently, another credential cannot unlock an existing cache, even if it
can access the same environment. A successful live run under a new credential
replaces the inaccessible cache for that server.

The cache has no automatic expiry because it exists to support extended
outages. Offline use can therefore inject values that were changed or revoked
on the unavailable server. `run` always reports cached use, its UTC fetch time,
and its age. Delete `run-cache/` and `run-cache-key` to remove all locally
cached runtime values; the next successful live run creates fresh material.

## Inspect effective configuration

`dopbase status` displays safe connection status:

```text
Config file:     /home/alex/.dopbase/config.toml
Server:          https://dopbase.example.com
Server status:   connected (live)
Server source:   config
Authentication:  encrypted_session
Identity:        admin
Email:           admin@example.com
Environment:     env_01ABCDEF (default)
```

It never displays a token or secret value. `dopbase status --json` returns the
same safe fields for diagnostics and automation:

```json
{
  "config_file": "/home/alex/.dopbase/config.toml",
  "server_url": "http://localhost:8840",
  "server_status": "offline",
  "status_source": "cache",
  "server_source": "default",
  "authentication": "none",
  "identity": "none",
  "email": null,
  "environment": null
}
```

The server status is `connected` with a `live` source when the health check
succeeds. It is `offline` with a `cache` source when Dopbase cannot be reached;
the remaining fields still come from safe local configuration and session
metadata. The health probe times out after three seconds.

The stable server-source values are `argument`, `environment`, `config`, and
`default`. Authentication is `environment`, `encrypted_session`, or `none`.
Identity is `admin`, `runner`, or `none`. An environment credential is reported
as a runner identity and has no email.

## Resolution order

The effective server is resolved in this order:

1. Global `--server <url>` argument
2. `DOPBASE_URL`
3. `server_url` in the global config
4. `http://localhost:8840`

Authentication is resolved in this order:

1. `DOPBASE_TOKEN`
2. The encrypted session matching the normalized active server

The environment used by `dopbase run` is resolved in this order:

1. Positional environment reference
2. A non-empty `DOPBASE_ENV`
3. The saved default when its server URL matches the effective server

Set or clear the default with `dopbase env default <environment>` and
`dopbase env default --clear`. A server override never reuses a default saved
for another endpoint.

A stored credential is used only when its server matches the effective server.
Dopbase never sends a saved token to an endpoint selected by an unrelated
`--server` or `DOPBASE_URL` override.

Command-line and environment-variable overrides affect only the current
process. They never rewrite the global configuration.

## Multiple instances on one machine

Each Dopbase server instance has its own address and server-owned SQLite
database. For example, two local instances might listen on ports `8840` and
`8377`.

Only one endpoint can be globally active; v0.0.14 does not retain named profiles
or token history for previously selected servers:

```bash
dopbase client connect http://localhost:8377
```

Other processes can target a different instance without changing global state:

```bash
DOPBASE_URL=http://localhost:8840 \
DOPBASE_TOKEN=<token-for-8840> \
dopbase run env_01LOCAL -- ./application
```

Client preferences never belong in a server database. The client must resolve
its endpoint before it can know which local or remote server database exists.
