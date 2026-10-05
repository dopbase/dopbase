---
title: "Quick start"
description: "Install Dopbase, start a local secrets server, import an existing .env file, and run an application with its secrets."
---

# Quick start

This walkthrough installs Dopbase {{version}}, starts a local server, imports an
existing `.env` file, and runs an application with its secrets.

::: warning Backup compatibility
Databases and backups from unsupported earlier releases cannot be used. Keep the
matching `master.key` with every backup. This command change requires no database
migration; already initialized instances do not need setup again.
:::

## 1. Install Dopbase

The installers download the correct release archive from GitHub Releases and
verify its SHA-256 checksum.

On macOS or Linux, install to `~/.local/bin`:

```bash
curl -fsSL https://dopbase.com/install.sh | sh
```

On Windows, run Dopbase in a Linux container with Docker. Dopbase does not
provide a native Windows binary or PowerShell installer.

Add the reported installation directory to `PATH` if the installer asks you
to, then confirm the installation:

```bash
dopbase -v
```

Set `DOPBASE_INSTALL_DIR` to choose another directory and
`DOPBASE_VERSION` to install a specific release. Mirrors can set
`DOPBASE_REPOSITORY_URL`. An explicit
`DOPBASE_DOWNLOAD_BASE_URL` still overrides the complete release download path.

## 2. Start the server

```bash
dopbase server start
```

Fresh storage automatically enters the existing web setup flow and prints a
protected setup link. Keep this process running.

## 3. Complete setup

Open the printed setup link. Enter your root email and a password of 12 to 128
characters, then confirm it. Setup signs you into the browser and opens Projects;
the server continues serving. Existing instances skip setup and show Sign in.

For startup without browser setup:

```bash
DOPBASE_ROOT_EMAIL=admin@example.com dopbase server start
```

This creates root on fresh storage and displays a generated password once. Save
it and sign in with it. Subsequent starts preserve the account and ignore the
bootstrap email. Guided terminal setup remains available through
`dopbase server setup`, followed by startup.

To restore a backup, use the web setup link without supplying a bootstrap email,
or run `dopbase server setup --web`. See [server setup](/cli/setup) for disabled
web UI settings, background credential output, and interrupted-reset recovery.

The default local server exposes:

```text
Dopbase
Secure, Simple and Private
Version {{version}}

Public URL: http://localhost:8840
Bind:       127.0.0.1:8840
Admin UI:   http://localhost:8840
API:        http://localhost:8840/api/v1
Config:     ~/.dopbase
```

The same address serves the Admin UI in a browser. If root was created from an
email supplied at startup, sign in with the generated credentials. The [Admin UI guide](/ui/) covers every screen.

Keep this process running while you use the client.

## 4. Confirm the client configuration

Open another terminal. With no configured server, Dopbase uses the local
default automatically:

```bash
dopbase client status
```

```text
Server:          http://localhost:8840
Server status:   connected (live)
Server source:   default
Authentication:  none
Identity:        none
Email:           none
Environment:     none (set with `dopbase env default <ENVIRONMENT_REF>`)
```

No repository or global config file is required for the implicit local server.
Use `dopbase client connect <server-url>` when targeting another local or
self-hosted instance.

## 5. Sign in

```bash
dopbase login
```

`login` authenticates with the resolved server and saves the token in the
encrypted local session file.

## 6. Initialize a project

From an application directory with an existing `.env` file:

```bash
cd my-project
dopbase init
```

Dopbase shows the variable count and asks for a new target such as
`my-project/development`. It then creates the project and environment in one
transaction and stores every `.env` entry as an encrypted secret. After the
import, it asks whether to delete `.env`, with Yes selected by default.

## 7. Run the application

Use the readable environment reference while developing:

```bash
dopbase run my-project/development -- npm start
```

Dopbase retrieves that environment and adds its secrets to the child process.
It does not write a new `.env` file to disk.

For production and staging deployments, use immutable environment IDs and
separate scoped tokens. Read [target projects and environments](/cli/environment-targeting)
for the complete workflow.
