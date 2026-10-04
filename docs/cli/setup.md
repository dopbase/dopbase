---
title: "Server setup"
description: "Initialize a local Dopbase instance from the terminal or use the existing web setup flow."
---

# Server setup

Run setup once for each local instance before starting it. Setup creates the
SQLite database, master key, configuration examples, and protected root account.
It does not initialize an application project. Use `dopbase init` for that
project and environment workflow after the server is running.

## Guided setup

```bash
dopbase server setup
dopbase server start
```

Setup asks for the root email, a masked password, and password confirmation.
Passwords must contain 12 to 128 characters. The command exits when setup is
complete; it does not start HTTP or sign the CLI in. Cancel with Ctrl+C.

Once the server is running, sign in with `dopbase login` or open the Admin UI.

## Setup without prompts

Provide an email to generate a password:

```bash
dopbase server setup --email admin@example.com
```

The command prints the generated password once after creating the account.
Save it before closing the terminal. Dopbase stores its password hash and does
not write the password to application logs. Terminal recordings, redirected
output, and CI logs can still retain the printed value.

For automation, request JSON and send stdout to a protected destination:

```bash
dopbase --data-dir /srv/dopbase --json server setup --email admin@example.com
```

Successful JSON output contains these fields. The values below are placeholders:

```json
{
  "initialized": true,
  "admin_id": "<root-account-id>",
  "email": "admin@example.com",
  "data_dir": "/srv/dopbase",
  "password": "<generated-password>"
}
```

Diagnostics go to stderr. `--email` never prompts. Guided setup and `--web`
reject `--json`; guided setup also requires a terminal.

If setup commits but cannot deliver its output, it exits with an error and
reports that the instance is initialized. JSON errors include `"initialized": true`. Recover the password with `dopbase admin reset-password EMAIL` using the
same instance options. Running setup again never replaces an existing account.

## Web setup

```bash
dopbase server setup --web
```

This starts the existing foreground server and prints its setup token and link.
Open the link to create root or restore a backup. Creating root starts a browser
session and opens Projects. Restoring a backup opens Sign in.

The server continues running after setup. Press Ctrl+C to stop it; later, use
`dopbase server start` or `dopbase server start --background`.

Web setup accepts the same listener options as foreground startup, including
`--host`, `--port`, `--public-url`, `--shutdown-grace-seconds`, `--docs`, and
`--no-docs`. It does not support background setup. `--web` and `--email` cannot
be combined. Listener options require `--web`.

First-run CLI restore still works against this listener. In another terminal:

```bash
dopbase restore ./backup.dop --key /path/to/source/master.key --setup-token '<SETUP_TOKEN>' --yes
```

Use the endpoint printed by web setup. If it differs from the client default,
pass `--server URL` to the restore command. See [backup and restore](/guide/backups-and-restore).

## Select the local instance

Setup and startup use the same configuration precedence. Global `--data-dir`
selects the local instance; `--config` selects its server configuration and
`--master-key-file` selects its encryption key.

```bash
dopbase --data-dir /srv/dopbase server setup --config /etc/dopbase/server.toml
dopbase --data-dir /srv/dopbase server start --config /etc/dopbase/server.toml
```

Setup rejects `--server` because it operates on local storage. It preserves
existing configuration and master keys and does not change the saved client
endpoint or default environment. Only one setup or server process may use an
instance at a time.

## Existing and reset instances

An initialized instance needs no setup again. Setup refuses to replace its root
account. Use offline password recovery if access is lost.

After a factory reset, stop the running process and run setup before restarting.
Explicit setup also completes a pending reset left by an interrupted operation.

Fresh or uninitialized storage cannot be started with `server start`, including
background startup. The command exits with instructions to run setup first.
