# Quick start

This walkthrough shows the planned v0.1 experience for a local, self-hosted server.

::: warning Planned interface
No stable Dopbase release is available yet. The commands below define the intended workflow and may change during implementation.
:::

## 1. Install Dopbase

The planned installer will place the `dopbase` executable on your path:

```bash
curl -fsSL https://dopbase.com/install.sh | sh
```

Until a release and installer are published, treat this command as documentation of intent rather than an available download.

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

## 3. Connect the client

Open another terminal and set the active server:

```bash
dopbase client connect http://localhost:8376
```

This connection applies to later commands until you choose a different server.

## 4. Sign in

```bash
dopbase login
```

`login` authenticates with the active server. Connecting and authenticating are separate operations.

## 5. Import a project

From an application directory:

```bash
cd my-project
dopbase import .env
```

The planned import flow creates or selects a project and environment, then stores every `.env` entry as an individual secret.

## 6. Run the application

```bash
dopbase run -- npm start
```

Dopbase retrieves the selected environment and adds its secrets to the child process. It does not need to write a new `.env` file to disk.

Next, read [server and client](./server-client) to understand where data and credentials live.
