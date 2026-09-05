---
title: "Quick start"
description: "Install Dopbase, start a local secrets server, import an existing .env file, and run an application with its secrets."
---

# Quick start

This walkthrough installs Dopbase 0.0.14, starts a local server, imports an
existing `.env` file, and runs an application with its secrets.

::: warning Testing release
Dopbase 0.0.14 is intended for testing and evaluation. The first public release
will be 0.1.0.
:::

## 1. Install Dopbase

The installers download the correct release archive from GitHub Releases and
verify its SHA-256 checksum.

On macOS or Linux, install to `~/.local/bin`:

```bash
curl -fsSL https://dopbase.com/install.sh | sh
```

On Windows x64, run PowerShell and install to `%LOCALAPPDATA%\Dopbase\bin`:

```powershell
irm https://raw.githubusercontent.com/dopbase/dopbase/0.0.14/scripts/install.ps1 -OutFile install.ps1
.\install.ps1
Remove-Item install.ps1
```

Add the reported installation directory to `PATH` if the installer asks you
to, then confirm the installation:

```bash
dopbase --version
```

Set `DOPBASE_INSTALL_DIR` to choose another directory and
`DOPBASE_VERSION` to install a specific release. The PowerShell installer also
accepts `-InstallDir` and `-Version` parameters. Mirrors can set
`DOPBASE_REPOSITORY_URL` (or PowerShell `-RepositoryUrl`); an explicit
`DOPBASE_DOWNLOAD_BASE_URL` still overrides the complete release download path.

## 2. Start the server

```bash
dopbase serve
```

The default local server exposes:

```text
╭──────────────────────────────────────────────────────────────────╮
│  Dopbase                                                     │
│  Secure, Simple and Private                                  │
│  Version 0.0.14                                              │
│                                                              │
│  Admin UI:   http://localhost:8840                           │
│  API:        http://localhost:8840/api/v1                    │
│  Config:     /Users/venobi/.dopbase                          │
╰──────────────────────────────────────────────────────────────────╯
```

The same address serves the Admin UI in a browser. The first visit walks you
through claiming the server with the setup token; the [Admin UI guide](/ui/)
covers every screen.

Keep this process running while you use the client.

## 3. Confirm the client configuration

Open another terminal. With no configured server, Dopbase uses the local
default automatically:

```bash
dopbase status
```

```text
Server:          http://localhost:8840
Server status:   connected (live)
Server source:   default
Authentication:  none
Identity:        none
Email:           none
Environment:     none (set with `dopbase env default <project/environment>`)
```

No repository or global config file is required for the implicit local server.
Use `dopbase client connect <server-url>` when targeting another local or
self-hosted instance.

## 4. Sign in

```bash
dopbase login
```

`login` authenticates with the resolved server and saves the token in the
encrypted local session file.

## 5. Bootstrap a project

From an application directory with an existing `.env` file:

```bash
cd my-project
dopbase init my-project development --from .env
```

Dopbase atomically creates the project and its `development` environment, then
stores every `.env` entry as an individual secret. It prints the immutable
environment ID without writing Dopbase configuration into the repository.

## 6. Run the application

Use the readable environment reference while developing:

```bash
dopbase run my-project/development -- npm start
```

Dopbase retrieves that environment and adds its secrets to the child process.
It does not write a new `.env` file to disk.

For production and staging deployments, use immutable environment IDs and
separate scoped tokens. Read [target projects and environments](/cli/environment-targeting)
for the complete workflow.
