---
title: "Docker integration"
description: "Inject Dopbase secrets into docker run, Docker Compose, and docker exec, and manage a containerized server from the host CLI."
---

# Docker integration

Dopbase integrates with Docker in two ways:

1. **`dopbase docker …`** — fetch secrets from the server and pass them to
   **your application** containers through a temporary `--env-file`.
2. **`dopbase --container …`** — run **local** `server` and `admin` commands
   inside the Dopbase server container via `docker exec` (container name from
   `DOPBASE_CONTAINER`, default `dopbase`).

Deploying the Dopbase server itself in Docker is covered in
[Run Dopbase in Docker](/self-hosting/docker).

## Prerequisites

- Docker CLI on `PATH` (`docker version` succeeds).
- An authenticated Dopbase client (`dopbase login` or `DOPBASE_TOKEN`).
- Reveal permission for the target environment (export uses the same access
  checks as `dopbase export`).

## Environment references

All `dopbase docker` subcommands accept the same references as other client
commands:

```text
env_482731                          # Environment ID
payment-service/production          # Project and environment
workspace1/api/staging              # Workspace, project path or name, environment
```

Use workspace-qualified references when two projects share a name in different
workspaces. See [Environment references](./commands#environment-references).

## `docker env-file`

Write a Docker-compatible env file without running a container:

```bash
dopbase docker env-file payment-service/production -o staging.env
dopbase docker env-file workspace1/api/staging
```

Stdout omits `-o`. Lines use raw `KEY=value` form (no shell quoting). Multiline
values are rejected because Docker env files cannot represent them safely.

## `docker run`

Forward a full `docker run` command. Dopbase inserts `--env-file` immediately
after `run`:

```bash
dopbase docker run payment-service/production -- \
  docker run --rm --name api my-registry/api:latest
```

The temporary env file is created with restrictive permissions and removed when
the command finishes.

## `docker compose`

Same pattern for Compose:

```bash
dopbase docker compose workspace1/api/staging -- \
  docker compose -f deploy/stack.yml up --build
```

Dopbase inserts `--env-file` right after the `compose` subcommand in the
forwarded argv list.

## `docker exec`

Run a command in the **Dopbase server container** with an environment's secrets
injected through `--env-file`:

```bash
dopbase docker exec workspace1/api/staging -- admin reset-password admin@example.com
```

For `admin` and `server`, write only the subcommand after `--`; Dopbase runs
`docker exec … dopbase <subcommand>` in the container named by
`DOPBASE_CONTAINER` (default `dopbase`).

To inject secrets into **application** containers, use `docker run` or
`docker compose` instead.

When stdin is a TTY, Dopbase adds `-it` to `docker exec`.

## `--container` (server administration)

When the Dopbase server runs in Docker, administer it from the host without
installing a second binary and **without** loading an environment's secrets:

```bash
dopbase --container server status
dopbase --container server logs --watch
dopbase --container admin reset-password admin@example.com
```

`--container` accepts `admin` plus `server status` and `server logs`. It rejects
`server setup`, `start`, `stop`, `restart`, `up`, and `down`: the container
runtime owns that process. Remote client workflows (`secret`, `token`, `run`,
`docker`, …) use `--server` / `dopbase client connect` instead.

## Alternative: export and `docker exec`

For scripts that already use export, Docker format streams env lines to stdin:

```bash
dopbase export payment-service/staging --stdout --format docker |
  docker exec --env-file=/dev/stdin my-container node script.js
```

Place `--env-file` before the container name. To run `admin` or `server` in the
Dopbase container with secrets loaded, use `dopbase docker exec <ENV_REF> -- …`
instead of a pipeline.

## CI example

Create a runner token once (human login), then in GitLab or GitHub Actions:

```bash
export DOPBASE_URL=https://dopbase.example.com
export DOPBASE_TOKEN=dbs_…
dopbase docker compose workspace1/api/staging -- docker compose -f ci/compose.yml up -d
```

Runner tokens can export without interactive password confirmation. Rotate
tokens per environment and job class.

## Security notes

- Injected env files exist briefly on disk. Restrict host permissions and avoid
  shared build agents without ephemeral workspaces.
- `docker run` / `compose` / `exec` reveal plaintext secrets in the container
  environment. Prefer short-lived containers and minimal secret sets.
- Audit events are recorded for secret export paths used under the hood.

## Related

- [Import and export](./commands#import-and-export) — `--format docker`
- [Run a process](./commands#run-a-process) — non-Docker injection with
  `dopbase run`
- [Run Dopbase in Docker](/self-hosting/docker) — container start and stop
- [Server lifecycle](./serve) — native foreground and background server processes
