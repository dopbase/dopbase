# Server and client

Dopbase uses one executable in two roles. The server protects and serves data. The client tells a server what you want to do.

```text
Self-hosted server or Dopbase Cloud
                │
                │ REST API
                ▼
         Dopbase CLI client
                │
                ▼
      Your application process
```

## The server

`dopbase serve` starts a self-hosted Dopbase instance. The server owns:

- Encrypted secret records and their metadata
- Projects and environments
- Human users, machine identities, and service tokens
- Authentication and authorization
- Audit records
- Database migrations
- The REST API and admin interface

Self-hosted storage is planned to use SQLite by default. The master encryption key must remain outside that database.

## The client

The client is every command that talks to a server, including `login`, `set`, `get`, `import`, `export`, and `run`.

First choose the active endpoint:

```bash
dopbase client connect http://localhost:8376
```

Then authenticate:

```bash
dopbase login
```

The client stores connection and authentication configuration locally. It does not become a second source of truth for project secrets.

## Self-hosted and Cloud

The same client and API model apply to both deployment types:

```bash
# Self-hosted
dopbase client connect http://localhost:8376

# Dopbase Cloud
dopbase client connect <dopbase-cloud-url>
```

Dopbase Cloud is a managed server endpoint. It does not control or depend on an independent self-hosted installation.

## Connection failures

If the active endpoint is unavailable, client operations should stop with a clear connection error. The client must not silently switch to another server, use stale secret values, or fall back to Cloud.
