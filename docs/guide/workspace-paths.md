---
title: "Workspace paths (design)"
description: "How Dopbase should map workspaces and projects to directories on each machine without storing absolute paths on the server."
---

# Workspace paths (design)

This document describes the target model for linking **workspaces** and
**projects** to directories on disk. It replaces storing a single absolute
**root directory** on the server for every developer and every environment.

Status: **proposed** (not yet fully implemented).

## Problem

Today each workspace row in the server database includes `root_path` (an
absolute path). The CLI resolves the current project by checking whether
`cwd` lies under `root_path + relative_path` fetched from the API.

That breaks down when:

- The **server runs in Docker** (`/data`, optional `/workspace`) but the
  **CLI runs on the host** (`D:\…`, WSL `/mnt/d/…`, macOS `~/…`).
- **Several developers** use the same server with different checkout locations.
- The **Admin UI** asks for a root path the browser cannot discover safely.
- A migration seeds `root_path = '/workspace'`, which does not match real
  laptops.

Docker volumes solve **server state** (SQLite, master key). They do not tell
the CLI where your git clone lives on your machine.

## Principles

| Layer | Responsibility |
| ----- | -------------- |
| **Server** | Logical structure: workspace **name**, project **name**, which project belongs to which workspace, **relative** directory segment per project (optional). Secrets, tokens, audit. |
| **CLI (per machine)** | Absolute **workspace root** on this host, bound to `(server_url, workspace_id)`. |
| **Repository (optional)** | Small committed file naming the workspace and optional path override for **this** clone—no secrets. |

Absolute filesystem paths are **never required** on the server.

## Target data model

### Server (SQLite)

**`workspaces`**

| Column | Notes |
| ------ | ----- |
| `id`, `name`, timestamps | Unchanged |
| `root_path` | **Removed** (or deprecated: ignored by new CLI, dropped in a later release) |

**`project_locations`**

| Column | Notes |
| ------ | ----- |
| `workspace_id` | Unchanged |
| `relative_path` | Path **inside the workspace root** on disk: `api`, `services/payment`, or `.` for the workspace root itself. Still unique per `(workspace_id, relative_path)`. |

The server does not validate that these directories exist. It only prevents
two projects from claiming the same relative slot in one workspace.

### CLI local config

Extend client configuration under `DOPBASE_DATA_DIR` (today `client.toml`), or
add a sibling file `workspace-links.toml`, version **2**:

```toml
version = 2
server_url = "https://dopbase.example.com"

[workspace_roots]
wsp_default = "D:/Programming/Allior"
wsp_acme = "/home/me/work/acme"
```

Rules:

- Keys are **workspace IDs** (`wsp_…`), not names (names can be renamed).
- Values are absolute paths on **this machine**, normalized to forward slashes
  in the file; the CLI canonicalizes when matching `cwd`.
- One map entry per server URL (multi-server setups keep separate files or
  a nested table keyed by normalized server URL).

Optional environment override for containers:

```bash
export DOPBASE_WORKSPACE_ROOT_wsp_default=/workspace
```

Use only when the CLI always runs in the same container with the same mount.

### Optional repo marker (committed)

Walk upward from `cwd` for `.dopbase/config.toml`:

```toml
version = 1
workspace = "default"          # workspace name slug on the server
relative_path = "dopbase"      # optional: overrides server relative_path for this clone
```

- Safe to commit: no secrets, only logical names.
- Helps monorepos where the server still says `relative_path = "api"` but this
  checkout lives at `services/api`.
- If `workspace` is omitted, the CLI uses local roots and longest-prefix match
  only.

Add `.dopbase/config.toml` to team docs; do **not** gitignore unless it
contains machine-specific overrides (prefer local config for those).

## Resolution algorithm

When the CLI needs a project from `cwd` (for example `dopbase run staging`,
or completing a bare environment name):

1. **Canonicalize** `cwd` (existing behavior; case-insensitive prefix match on
   Windows).
2. **Load** `workspace_roots` for the active `server_url`.
3. **Optional:** read `.dopbase/config.toml` from `cwd` or a parent directory.
4. **Fetch** projects and workspaces from the API (as today).
5. For each project with a `project_locations` row:
   - `root = workspace_roots[workspace_id]`
   - Skip if `root` is missing (no match on this machine until linked).
   - `rel = repo_config.relative_path` if repo config applies to this
     project/workspace; else `project.relativePath`.
   - `absolute = join(root, rel)`
   - If `cwd` is under `absolute`, record candidate with score `len(absolute)`.
6. Return the **longest** matching path (most specific project directory).

**Legacy fallback (transition):** if `workspace_roots` is empty and the API
still returns `rootPath`, use it once and print a deprecation warning pointing
to `dopbase workspace link`.

## CLI commands

### `dopbase workspace link`

Bind the workspace root on **this machine** to a directory.

```bash
# From the workspace root (or any subdirectory)
dopbase workspace link

# Explicit workspace and path
dopbase workspace link --workspace default --root D:/Programming/Allior
dopbase workspace link --workspace default --root .
```

Behavior:

- Default path: `.` (canonicalized).
- Resolves `--workspace` by name slug or ID; default workspace when omitted if
  unambiguous.
- Writes `workspace_roots` in local client config.
- Prints the stored absolute path and workspace name.

### `dopbase workspace roots`

List local bindings for the current server (IDs, names, paths). No server
mutation.

### `dopbase project link`

Attach **this directory** (or a given path) to a server project inside a
workspace:

```bash
cd services/api
dopbase project link payment-service
```

Behavior:

1. Require a local workspace root that contains `cwd` (from `workspace link`).
2. Compute `relative_path` from workspace root to target directory.
3. `PATCH /projects/{ref}/location` with `workspaceId` + `relativePath`.
4. Optionally write or update `.dopbase/config.toml` when `--mark-repo` is set.

### `dopbase init`

After creating a project/environment, if `workspace link` exists for the active
workspace, call the same logic as `project link` with `relative_path` derived
from `cwd` (instead of defaulting `relative_path` to project name only).

## Admin UI changes

| Today | Target |
| ----- | ------ |
| Workspace dialog: name + **Root directory** | Workspace dialog: **name only** |
| Hint about CLI on developer machine | Link to this guide and `dopbase workspace link` |
| Project directory in UI | Optional advanced field **relative path** only, or “Set with CLI” |
| Rail shows `relativePath` | Keep; hide server `rootPath` |

The UI may show **“Not linked on this machine”** only in a future desktop
shell; the web UI should not pretend to know local paths.

## Docker and self-hosting

| Scenario | Recommendation |
| -------- | ---------------- |
| CLI on host, server in Docker | `dopbase workspace link` on the host with the real checkout root. |
| Dev container with repo at `/workspace` | `dopbase workspace link --root /workspace` inside the container, or `DOPBASE_WORKSPACE_ROOT_*` in `devcontainer.json`. |
| CI runner | `workspace link` in job setup, or always pass full `WORKSPACE/PROJECT/ENV` refs and skip cwd resolution. |
| Server container volume | Still only `/data`; no change to workspace linking model. |

## Migration plan

1. **Phase A — additive**
   - Local `workspace_roots` + `workspace link` / `project link`.
   - Resolver prefers local roots; fallback to server `rootPath` + warning.
   - Docs and UI copy updated.

2. **Phase B — API**
   - `WorkspaceInput.root_path` optional on create; default omitted.
   - PATCH workspace no longer requires `root_path`.

3. **Phase C — schema**
   - Migration drops `workspaces.root_path`.
   - OpenAPI and UI remove the field.

4. **Phase D — defaults**
   - Stop inserting `/workspace` for new installs.
   - Quick start: `login` → `workspace link` → `init` / `project link`.

Backups restore `project_locations` as today; local links remain on each
machine and are not part of `.dop` exports.

## Security and privacy

- Local paths stay in `~/.dopbase` (or `DOPBASE_DATA_DIR`); not sent to the
  server or audit log.
- Repo marker contains only workspace name and relative path.
- No change to secret handling or export permissions.

## Open questions

- **Per-server file vs single `client.toml`:** nesting `workspace_roots` under
  each saved `server_url` avoids collisions when one laptop talks to prod and
  staging servers.
- **Multiple roots per workspace:** rare (split checkouts); defer unless needed.
- **Name-based `workspace link`:** match slug; conflict if duplicate names—prefer
  ID in config file.

## Related

- [Projects, environments, and secrets](./projects-environments-secrets)
- [Environment targeting](/cli/environment-targeting)
- [Server and client](./server-client)
