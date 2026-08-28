# Quick start

This walkthrough shows the planned v0.1 experience for a local, self-hosted
server.

::: warning Planned interface
No stable Dopbase release is available yet. The commands below define the
intended workflow and may change during implementation.
:::

## 1. Install Dopbase

The planned installer will place the `dopbase` executable on your path:

```bash
curl -fsSL https://dopbase.com/install.sh | sh
```

Until a release and installer are published, treat this command as
documentation of intent rather than an available download.

## 2. Start the server

```bash
dopbase serve
```

The default local server is expected to expose:

```text
Admin UI: http://localhost:8376
API:      http://localhost:8376/api
Database: ./dopbase.db
```

Keep this process running while you use the client.

## 3. Confirm the client configuration

Open another terminal. With no configured server, Dopbase uses the local default
automatically:

```bash
dopbase config
```

```text
Server:          http://localhost:8376
Server source:   default
Authentication:  not logged in
Environment:     none (pass one explicitly)
```

No repository or global config file is required for the implicit local server.
Use `dopbase client connect <server-url>` only when targeting another local,
self-hosted, or Cloud instance.

## 4. Sign in

```bash
dopbase login
```

`login` authenticates with the resolved local server and saves the token in the
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
It does not need to write a new `.env` file to disk.

For production and staging deployments, use immutable environment IDs and
separate scoped tokens. Read [target projects and environments](/cli/environment-targeting)
for the complete workflow.
