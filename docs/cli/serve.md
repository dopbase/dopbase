# `dopbase serve`

`dopbase serve` starts a self-hosted Dopbase server.

```bash
dopbase serve
```

The planned local defaults are:

```text
Admin UI: http://localhost:8376
API:      http://localhost:8376/api
Database: ./dopbase.db
```

::: warning Planned interface
Network binding, TLS, data-directory, master-key, and logging flags have not been finalized. Do not build production automation around these defaults yet.
:::

## What starts with the server

The server executable is expected to include:

- The HTTP server and REST API
- Authentication and authorization
- Encryption and decryption services
- SQLite access and database migrations
- The admin interface
- Audit logging

The Vue admin interface can be embedded into the Rust executable at compile time, so users do not need to operate separate frontend and backend deployments.

## Local development

Binding to localhost is suitable for local evaluation because it does not expose the service to other machines by default. Production deployments will need an explicit network and TLS configuration, a protected master key, backups, monitoring, and an upgrade plan.

Read [self-hosting operations](/self-hosting/operations) before exposing a server beyond a local machine.
