---
title: "Environment variables"
description: "Environment variables supported by the Dopbase CLI and self-hosted server, with sample values and precedence rules."
---

# Environment variables

Dopbase reads the following environment variables. Command-line options take
precedence when both are set. For server settings, environment variables take
precedence over values in `server.toml`.

| Variable | Sample value | What it is used for |
| --- | --- | --- |
| `DOPBASE_TOKEN` | `dbs_xxxxxxxxxxxxxxxxx` | Authenticates client commands with a runner or AI agent token. It takes precedence over the encrypted session created by `dopbase login`. |
| `DOPBASE_URL` | `https://dopbase.example.com` | Selects the server for client commands when the global `--server` option is not set. |
| `DOPBASE_ENV` | `payment-service/production` | Selects the environment for `dopbase run` when no positional environment is given. An immutable ID such as `env_482731` also works. |
| `DOPBASE_DATA_DIR` | `/srv/dopbase` | Changes the directory used for server data and local CLI configuration. The default is `~/.dopbase`. |
| `DOPBASE_HOST` | `0.0.0.0` | Sets the network interface used by `dopbase server start` and `dopbase server up`. The default is `127.0.0.1`. |
| `DOPBASE_PORT` | `9000` | Sets the server port. The default is `8840`. |
| `DOPBASE_PUBLIC_URL` | `https://dopbase.example.com` | Sets the URL shown to clients and used in generated links. A non-loopback host requires an HTTPS public URL. |
| `DOPBASE_DOCS` | `true` | Enables or disables Swagger UI and the OpenAPI document. Accepted values are `true` and `false`; the default is `false`. |
| `DOPBASE_MASTER_KEY_PATH` | `/srv/dopbase/master.key` | Reads the server master key from the given file instead of `<data-dir>/master.key`. |
| `DOPBASE_SHUTDOWN_GRACE_SECONDS` | `30` | Sets how long the server waits for in-flight requests during shutdown. The default is `10` seconds. |

## Examples

Run an application against a remote environment:

```bash
export DOPBASE_URL=https://dopbase.example.com
export DOPBASE_TOKEN=dbs_xxxxxxxxxxxxxxxxx
export DOPBASE_ENV=env_482731

dopbase run -- ./payment-service
```

Start a self-hosted server with environment-based configuration:

```bash
export DOPBASE_DATA_DIR=/srv/dopbase
export DOPBASE_HOST=0.0.0.0
export DOPBASE_PORT=9000
export DOPBASE_PUBLIC_URL=https://dopbase.example.com
export DOPBASE_MASTER_KEY_PATH=/run/secrets/dopbase-master-key

dopbase server start
```

Do not commit `DOPBASE_TOKEN` or a master key to source control. Supply them
through your shell, process manager, CI secret store, or deployment platform.

`dopbase run` removes `DOPBASE_TOKEN` from the child process before it starts
the application.
