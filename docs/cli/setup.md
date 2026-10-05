---
title: "Server setup"
description: "Initialize a local Dopbase instance from the terminal or use the existing web setup flow."
---

# Server setup

Startup can initialize a fresh instance automatically. Explicit setup creates
the SQLite database, master key, configuration examples, and protected root account
without requiring automatic startup.
It does not initialize an application project. Use `dopbase init` for that
project and environment workflow after the server is running.

## Automatic first startup

```bash
dopbase server start
```

On fresh storage, startup creates the database, master key, and configuration
examples, then prints the existing protected web setup link. Open it to create
root or restore a backup. The server keeps running after setup. An initialized
instance starts normally without changing its accounts or credentials.

To create root without opening a browser, supply an email:

```bash
DOPBASE_ROOT_EMAIL=admin@example.com dopbase server start
```

Dopbase generates a password, displays it once, and continues serving. Save the
password, then sign in through the Admin UI or `dopbase login`. Account creation
does not sign you in. Existing instances ignore this input, including invalid
values. Missing or blank values select web setup on fresh storage; invalid
nonblank values fail before creating files.

Foreground and background startup use the same rules. Automatic setup respects
`--no-web-ui`, `DOPBASE_WEB_UI=false`, and `web_ui = false`. With the UI disabled,
supply `DOPBASE_ROOT_EMAIL` or complete explicit CLI setup first. A pending
factory reset still requires explicit setup; automatic startup does not finish
an interrupted reset. Storage and master-key errors never fall back to setup.

Explicit setup remains available when you want terminal prompts, standalone
provisioning, or the web flow with an email prefilled.

## Guided setup

```bash
dopbase server setup
dopbase server start
```

When neither `--email` nor a nonblank `DOPBASE_ROOT_EMAIL` is supplied, setup
asks for the root email, a masked password, and password confirmation.
Passwords must contain 12 to 128 characters. The command exits when setup is
complete; it does not start HTTP or sign the CLI in. Cancel with Ctrl+C.

Once the server is running, sign in with `dopbase login` or open the Admin UI.

## Setup without prompts

Provide an email to generate a password:

```bash
dopbase server setup --email admin@example.com
```

You can also supply the email through the environment:

```bash
export DOPBASE_ROOT_EMAIL=admin@example.com
dopbase server setup
```

The email is resolved from `--email` first, then `DOPBASE_ROOT_EMAIL`. Empty or
whitespace-only environment values count as unset. Invalid nonempty values
fail before creating files or starting a listener; they do not fall back to
prompts. A valid explicit `--email` overrides an invalid environment value.

For Docker and cloud deployments, use the same persistent data location on every
start. `DOPBASE_ROOT_EMAIL` also initializes fresh storage during `server start`.
It is a bootstrap input, not a password-reset setting, and is not saved in
`server.toml` or background restart metadata. To restore a backup on fresh
storage, omit this variable during automatic startup or explicitly use
`server setup --web`, where the email only prefills the form.

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

Diagnostics go to stderr. `--email` or a nonblank `DOPBASE_ROOT_EMAIL` skips
prompts in CLI mode and supports `--json`. Guided setup and `--web`
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

To prefill the root email, set `DOPBASE_ROOT_EMAIL` or pass `--email`:

```bash
dopbase server setup --web --email ops+cloud@example.com
```

The printed link includes an encoded email, for example
`/setup?token=<SETUP_TOKEN>&email=ops%2Bcloud%40example.com`. Opening it fills the
token and email fields, then removes those parameters from the address bar.
The email remains editable. You still choose and confirm a password before
submitting the form. With no email supplied, the existing token-only link is
printed.

`--web` explicitly enables the web UI for this run, even when `web_ui = false`
or `DOPBASE_WEB_UI=false`. The UI remains available after setup until you stop
this process. The override is not saved.

The server continues running after setup. Press Ctrl+C to stop it; later, use
`dopbase server start` or `dopbase server start --background`. Normal startup
follows the configured web UI setting. `--no-web-ui` applies only to
`server start`.

Web setup accepts the same listener options as foreground startup, including
`--host`, `--port`, `--public-url`, `--shutdown-grace-seconds`, `--docs`, and
`--no-docs`. It does not support background setup. Listener options require
`--web`.

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

After a completed factory reset, stop the running process and start again to
enter automatic setup. If `.factory-reset.pending` remains after an interrupted
reset, startup stops without deleting more data. Run explicit setup with the
same instance options to finish the reset before starting again.
