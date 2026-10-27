---
title: "Run Dopbase in Docker"
description: "Deploy the Dopbase server as a container image with persistent data, configuration, health checks, and upgrades from GHCR."
---

# Run Dopbase in Docker

The official container image ships the same `dopbase` binary as the release
installers: Admin UI, API, and CLI entrypoint in one image. Docker is the
supported way to run Dopbase on **Windows** and a common choice for Linux
servers when you already operate a container stack.

## Image registry

Every push to the default branch publishes a `linux/amd64` image to GitHub
Container Registry (GHCR):

```text
ghcr.io/<owner>/dopbase:latest
ghcr.io/<owner>/dopbase:main
ghcr.io/<owner>/dopbase:<version>
```

Replace `<owner>` with your fork or organization (for example `an1by`). Pin a
specific tag in production instead of relying on `latest` alone.

Pull the image:

```bash
docker pull ghcr.io/<owner>/dopbase:latest
```

Private packages require `docker login ghcr.io` with a token that has
`read:packages` scope.

## Minimal Compose stack

The container expects:

| Mount / setting | Purpose |
| --------------- | ------- |
| `./data` → `/data` | SQLite database, audit data, and `master.key` |
| `./server.toml` → `/etc/dopbase/server.toml` | Bind address, port, public URL, docs flag |
| `DOPBASE_DATA_DIR=/data` | Keep state inside the volume |
| `DOPBASE_MASTER_KEY_PATH=/data/master.key` | Master key next to the database |

Example `docker-compose.yml`:

```yaml
services:
  dopbase:
    image: ghcr.io/<owner>/dopbase:latest
    container_name: dopbase
    restart: unless-stopped
    ports:
      - "8840:8840"
    volumes:
      - ./data:/data
      - ./server.toml:/etc/dopbase/server.toml:ro
    environment:
      DOPBASE_DATA_DIR: /data
      DOPBASE_MASTER_KEY_PATH: /data/master.key
    command: ["server", "start", "--config", "/etc/dopbase/server.toml"]
    healthcheck:
      test:
        [
          "CMD",
          "/usr/local/bin/dopbase",
          "--data-dir",
          "/data",
          "server",
          "status",
        ]
      interval: 30s
      timeout: 10s
      retries: 5
      start_period: 120s

volumes: {}
```

Example `server.toml` for a single-host deployment behind TLS on a reverse
proxy:

```toml
version = 1
host = "0.0.0.0"
port = 8840
public_url = "https://dopbase.example.com"
docs = false
web_ui = true
```

Set `DOPBASE_ROOT_EMAIL` on the service to create the root account with a
generated password on the first `server start`. The password and setup link
are printed once in the container logs. An initialized container ignores that
variable on later starts. Set `DOPBASE_WEB_UI=false` or `web_ui = false` to
serve the API without the Admin UI.

Start the stack:

```bash
docker compose up -d
```

Open the Admin UI at the configured `public_url` (or `http://localhost:8840`
during local testing). Complete setup with the bootstrap token shown in the
container logs on first start.

## Client on the host

Install the `dopbase` CLI on your laptop or CI runner (or use a throwaway
client container with the same image). Point it at the published port:

```bash
dopbase client connect http://localhost:8840
dopbase login
dopbase client status
```

Use `https://` when TLS terminates at your proxy and `public_url` matches that
hostname.

## Operating the server inside the container

The image entrypoint is `dopbase`, and the default command is foreground
`server start`. Docker Compose starts, stops, and restarts that process
(`docker compose up -d`, `stop`, `restart`). Do not use `server up`,
`server down`, `server start --background`, `server stop`, or
`server restart` inside the container.

From the host, forward inspection and admin commands with `--container`:

```bash
dopbase --container server status
dopbase --container server logs --lines 100
dopbase --container admin reset-password admin@example.com
```

Set `DOPBASE_CONTAINER` when the container is not named `dopbase`.
`--container` runs `docker exec … dopbase …` with your global flags.
It does not replace the remote client; use `dopbase client connect` for
project, secret, and token commands against the HTTP API.

See [Docker and secrets injection](/cli/docker) for `dopbase docker run`,
`compose`, `exec`, and `env-file`.

## Data durability and backups

- Treat `./data` (or your named volume) as the source of truth. Back it up
  with filesystem snapshots or Dopbase `.dop` exports from the Admin UI.
- Keep `master.key` with every backup. Without the key, encrypted data is not
  recoverable.
- Do not bake secrets into the image. Inject runtime configuration through
  volumes and environment variables.

Read [Storage and backups](./storage-backups) and
[Encryption keys](./encryption-keys) before production use.

## Upgrades

1. Back up `/data` and confirm you have `master.key`.
2. Pull the new image tag.
3. Recreate the container (`docker compose up -d`).

Database migrations run automatically when the new binary starts. Review release
notes before upgrading across major versions.

## Network and security

- Bind `0.0.0.0` only when the container network is trusted or fronted by a
  proxy. Prefer publishing `127.0.0.1:8840:8840` and SSH tunnels for admin
  access when possible.
- Terminate TLS outside the container (Caddy, nginx, Traefik, cloud load
  balancer).
- Restrict who can reach the API and Admin UI. Dopbase authentication is
  required for secret access; network exposure still enlarges your attack
  surface.

## Windows

Dopbase does not ship a native Windows binary. Run the Linux container above
with Docker Desktop, connect the CLI from WSL or Git Bash, and use
`http://localhost:8840` as the server URL.

## Related guides

- [Operations](./operations) — TLS, monitoring, and incident response
- [Docker CLI integration](/cli/docker) — inject secrets into application
  containers
- [Environment references](/cli/commands#environment-references) —
  `WORKSPACE/PROJECT/ENVIRONMENT` when the same project name exists in multiple
  workspaces
