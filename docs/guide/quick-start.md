# Quick start

This walkthrough installs Dopbase 0.0.8, starts a local server, imports an
existing `.env` file, and runs an application with its secrets.

::: warning Testing release
Dopbase 0.0.8 is intended for testing and evaluation. The first public release
will be 0.1.0.
:::

## 1. Install Dopbase

The installer downloads the correct macOS or Linux archive from GitHub
Releases, verifies its SHA-256 checksum, and places `dopbase` in
`~/.local/bin`:

```bash
curl -fsSL https://dopbase.com/install.sh | sh
```

Add `~/.local/bin` to `PATH` if the installer asks you to, then confirm the
installation:

```bash
dopbase --version
```

Set `DOPBASE_INSTALL_DIR` to choose another directory. Set `DOPBASE_VERSION` to
install a specific release.

## 2. Start the server

```bash
dopbase serve
```

The default local server exposes:

```text
Admin UI: http://localhost:8840
API:      http://localhost:8840/api/v1
Database: ~/.dopbase/dopbase.db
```

Keep this process running while you use the client.

## 3. Confirm the client configuration

Open another terminal. With no configured server, Dopbase uses the local
default automatically:

```bash
dopbase config
```

```text
Server:          http://localhost:8840
Server source:   default
Authentication:  not logged in
Environment:     none (pass one explicitly)
```

No repository or global config file is required for the implicit local server.
Use `dopbase client connect <server-url>` when targeting another local or
self-hosted instance.

## 4. Sign in

```bash
dopbase login
```

`login` authenticates with the resolved server and saves the token in the
operating system credential store.

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
