---
title: "Environment variables"
description: "Environment variables supported by the Dopbase CLI and self-hosted server, with sample values and precedence rules."
---

# Environment variables

Dopbase reads the following environment variables. Command-line options take
precedence when both are set. For server settings, environment variables take
precedence over values in `server.toml`.

| Variable                         | Sample value                  | <div style="width: 360px;">What it is used for</div>                                                                                                                                                |
| -------------------------------- | ----------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `DOPBASE_TOKEN`                  | `dbs_xxxxxxxxxxxxxxxxx`       | Supplies a client credential before the encrypted credential saved by `dopbase login`. Runner tokens support `run` and scoped exports; AI agent tokens can read metadata only.                      |
| `DOPBASE_URL`                    | `https://dopbase.example.com` | Selects the server for client commands when the global `--server` option is not set.                                                                                                                |
| `DOPBASE_ENV`                    | `payment-service/production`  | Selects the environment for `dopbase run` when no positional environment is given. An immutable ID such as `env_482731` also works.                                                                 |
| `DOPBASE_ROOT_EMAIL`             | `ops@example.com`             | Bootstrap root email for fresh `server start` or CLI setup; generates a password once. Initialized startup ignores it. Explicit `setup --email` overrides it; `setup --web` only prefills the form. |
| `DOPBASE_DATA_DIR`               | `/srv/dopbase`                | Changes the directory used for server data and local CLI configuration. The default is `~/.dopbase`.                                                                                                |
| `DOPBASE_HOST`                   | `0.0.0.0`                     | Sets the network interface used by `dopbase server start`, `dopbase server start --background`, and `dopbase server setup --web`. The default is `127.0.0.1`.                                       |
| `DOPBASE_PORT`                   | `9000`                        | Sets the server port. The default is `8840`.                                                                                                                                                        |
| `DOPBASE_PUBLIC_URL`             | `https://dopbase.example.com` | Sets the URL shown to clients and used in generated links. Without it, a network bind uses a detected HTTP address and prints a warning.                                                            |
| `DOPBASE_DOCS`                   | `true`                        | Enables or disables Swagger UI and the OpenAPI document. Accepted values are `true` and `false`. The default is `false`.                                                                            |
| `DOPBASE_WEB_UI`                 | `false`                       | Enables or disables the web UI. Accepts `true` or `false`; defaults to `true`. `server start --no-web-ui` overrides it, and `server setup --web` enables the UI for that run.                       |
| `DOPBASE_MASTER_KEY_PATH`        | `/srv/dopbase/master.key`     | Reads the server master key from the given file instead of `<data-dir>/master.key`.                                                                                                                 |
| `DOPBASE_SHUTDOWN_GRACE_SECONDS` | `30`                          | Sets how long the server waits for in-flight requests during shutdown. The default is `10` seconds.                                                                                                 |

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
export DOPBASE_ROOT_EMAIL=ops@example.com
export DOPBASE_DATA_DIR=/srv/dopbase
export DOPBASE_HOST=0.0.0.0
export DOPBASE_PORT=9000
export DOPBASE_PUBLIC_URL=https://dopbase.example.com
export DOPBASE_MASTER_KEY_PATH=/run/secrets/dopbase-master-key

dopbase server start
```

Use the same persistent data location on subsequent starts. On fresh storage,
this command creates root and prints its generated password once; save it before
closing the terminal. Initialized startup ignores the bootstrap email and never
changes credentials. Blank values select web setup when the UI is enabled.
Invalid nonempty values fail before creating files on fresh storage. With a
disabled UI and no email, complete explicit setup first. See
[server setup](./setup) for background JSON output, recovery, and web prefilling.

Do not commit `DOPBASE_TOKEN` or a master key to source control. Supply them
through your shell, process manager, CI secret store, or deployment platform.

`dopbase run` removes `DOPBASE_TOKEN` from the child process before it starts
the application.
