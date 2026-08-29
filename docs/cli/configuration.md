# Client configuration

Dopbase keeps client connection state in a machine-global configuration. It
does not create configuration inside an application repository and does not
store an active project or environment.

## Default local server

When no server is configured, the CLI uses:

```text
http://localhost:8376
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

The v0.0.1 schema contains only non-secret client state:

```toml
version = 1
server_url = "https://dopbase.example.com"
```

`server_url` is omitted when the implicit local server is active. Dopbase
writes configuration atomically and restricts file permissions to the current
user where the platform supports them.

The file never stores project selection, environment selection, secret values,
or authentication tokens.

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

After a successful switch, Dopbase removes the previous server's saved
credential. Authenticate separately against the new endpoint:

```bash
dopbase login
```

Return to the implicit local server with:

```bash
dopbase client connect local
dopbase login
```

The `local` alias resolves to `http://localhost:8376`. Dopbase validates the
local server, removes the `server_url` override, and removes the previous
server's credential.

## Credentials

`dopbase login` stores the resulting token in the operating system credential
store, keyed by the normalized server URL. It never writes the token to
`config.toml` or a Dopbase server's SQLite database.

Only one saved connection is active in v0.0.1. Logging in again replaces the
credential for that server. `dopbase logout` removes the active credential but
leaves the selected server unchanged.

If the operating system credential store is unavailable, interactive login
fails with instructions to use `DOPBASE_TOKEN`. Dopbase does not silently fall
back to a plaintext credential file.

## Inspect effective configuration

`dopbase config` displays safe connection status:

```text
Config file:     /home/alex/.dopbase/config.toml
Server:          https://dopbase.example.com
Server source:   config
Authentication:  logged in (credential store)
Environment:     none (pass one explicitly)
```

It never displays a token or secret value. `dopbase config --json` returns the
same safe fields for diagnostics and automation:

```json
{
  "config_file": "/home/alex/.dopbase/config.toml",
  "server_url": "http://localhost:8376",
  "server_source": "default",
  "authentication": "none",
  "environment": null
}
```

The stable server-source values are `argument`, `environment`, `config`, and
`default`. Authentication is `environment`, `credential_store`, or `none`.

## Resolution order

The effective server is resolved in this order:

1. Global `--server <url>` argument
2. `DOPBASE_URL`
3. `server_url` in the global config
4. `http://localhost:8376`

Authentication is resolved in this order:

1. `DOPBASE_TOKEN`
2. The operating system credential matching the normalized active server

A stored credential is used only when its server matches the effective server.
Dopbase never sends a saved token to an endpoint selected by an unrelated
`--server` or `DOPBASE_URL` override.

Command-line and environment-variable overrides affect only the current
process. They never rewrite the global configuration.

## Multiple instances on one machine

Each Dopbase server instance has its own address and server-owned SQLite
database. For example, two local instances might listen on ports `8376` and
`8377`.

Only one endpoint can be globally active; v0.0.1 does not retain named profiles
or token history for previously selected servers:

```bash
dopbase client connect http://localhost:8377
```

Other processes can target a different instance without changing global state:

```bash
DOPBASE_URL=http://localhost:8376 \
DOPBASE_TOKEN=<token-for-8376> \
dopbase run env_01LOCAL -- ./application
```

Client preferences never belong in a server database. The client must resolve
its endpoint before it can know which local or remote server database exists.
