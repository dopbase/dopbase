---
title: "ADR 0001: Maintain the an1by fork alongside upstream"
description: Why an1by/dopbase exists, what it adds beyond dopbase/dopbase, and how we merge upstream without losing fork behavior.
---

# ADR 0001: Maintain the an1by fork alongside upstream

- **Status:** Accepted
- **Date:** 2026-03-28
- **Scope:** Repository [an1by/dopbase](https://github.com/an1by/dopbase) only. Does not apply to official [dopbase/dopbase](https://github.com/dopbase/dopbase) releases.

## Context

Dopbase is a single-binary secrets manager (server, Admin UI, API, CLI). We run it
in Allior’s environment with requirements upstream does not target as first-class
product features:

- **Workspaces** — group projects on disk, qualified references
  (`workspace/project/environment`), workspace-scoped runner tokens (`dbw_`).
- **Docker** — GHCR images, `dopbase docker` for injecting secrets into arbitrary
  containers, and `dopbase --container` for admin commands inside the server
  container.
- **Windows** — native CLI builds, `install.ps1`, zip artifacts, and path
  conventions (Git Bash, `%VAR%`).

Upstream moves quickly (for example 0.1.9 lifecycle CLI, 0.1.10 SSO, audit chain,
timestamp migrations). Rebasing a published `main` would rewrite history and break
consumers of `an1by/dopbase`. We still need security fixes and core product
improvements from upstream.

## Decision

1. **Track upstream `main`** with **merge commits** into this repository (branches
   such as `merge/upstream-0.1.x`, then fast-forward or merge into `main`). No
   force-push of shared history.

2. **Adopt the upstream CLI and server contract** for each merged release (breaking
   changes included). Do not keep deprecated upstream commands working in the fork
   (for example `server up` / `dopbase run --token` after 0.1.9).

3. **Preserve fork-only capabilities** listed below unless an ADR explicitly retires
   them. Conflict resolution favors upstream for shared files, then re-applies fork
   behavior (workspaces, Docker, Windows).

4. **Document fork-only schema** as timestamped SQL migrations **after** the
   upstream chain for that release. Legacy four-digit versions `14`–`17` on existing
   fork databases map to those files by **checksum** via `adopt_legacy_versions` in
   `app/src/services/db.rs`, same mechanism as upstream’s 0.1.9 → timestamp rename.

5. **Container lifecycle** is owned by Docker Compose (or the orchestrator), not by
   background `dopbase server` commands inside the image. `dopbase --container` may
   run `server status`, `server logs`, and `admin` only; it must reject
   `setup` / `start` / `stop` / `restart` / `up` / `down` (including
   `start --background`).

## Fork-only capabilities

| Area | Behavior |
| ---- | -------- |
| **Workspaces** | Tables `workspaces`, `project_locations`, `workspace_tokens`; API and CLI `workspace`; UI workspace-scoped project URLs; `dbw_` tokens via `runner_may_access_environment`. |
| **References** | Environment and project refs accept `WORKSPACE/PROJECT/ENV` and `/PROJECT/ENV` (default workspace). |
| **Export** | `dbs_` uses scoped export API (audit/revision); `dbw_` uses runtime secrets API from the CLI. |
| **Docker** | `Dockerfile`, `.github/workflows/container.yml` (+ optional `container-arm.yml`); `dopbase docker run \| compose \| exec \| env-file`. |
| **`--container`** | `docker exec` into `DOPBASE_CONTAINER` (default `dopbase`). |
| **Windows** | Release zip + `scripts/install.ps1`; update hint in CLI; path expansion for Windows shells. |
| **Dotenv** | Triple-quoted multiline values in `.env` parsing (fork extension; not in upstream). |

Design notes for workspace paths on disk: [Workspace paths (design)](/guide/workspace-paths).

## Upstream merge process

1. Fetch `dopbase/dopbase` `main` (for example `refs/remotes/upstream/main`).
2. Merge into `merge/upstream-<version>` from current `main`.
3. Resolve conflicts using the table in merge plans: upstream SSO/audit/secrets/UI;
   fork workspaces, `container.rs`, Windows release workflow, `DOPBASE_CONTAINER`.
4. Replace numeric migrations with upstream timestamp files; append fork workspace
   pairs with **unchanged SQL** so checksum adoption works.
5. Run `cargo check`, migration tests (including fork legacy `1`–`17` → timestamps),
   and targeted CLI tests (`workspace`, `container`, `dotenv`, export tokens).
6. Commit with `chore!: merge upstream …` and a `BREAKING CHANGE` footer when the
   upstream release is breaking.

## Compatibility

| Artifact | Opens fork DB with workspaces? | Notes |
| -------- | ------------------------------ | ----- |
| **an1by/dopbase** binary | Yes | Includes workspace migrations after upstream head. |
| **Official dopbase** binary | No | Unknown migrations `14`–`17` / fork timestamp names. |
| **DB after 0.1.10 migrate** | — | Do not downgrade to 0.1.9; backup before upgrade. |

## Consequences

- **Positive:** Security and product features from upstream without abandoning
  workspaces or Docker workflows; clear merge playbook for future 0.1.x releases.
- **Negative:** Permanent merge and test burden; documentation must distinguish fork
  vs upstream; two migration tails (upstream + four workspace files) forever unless
  workspaces are upstreamed or removed via ADR.
- **Operational:** Upgrade fork instances with backup; use official images/binaries
  only on databases created without fork migrations.

## Related material

- [CONTEXT.md](../../CONTEXT.md) — domain terms (workspace, project, environment).
- [app/migrations/README.md](../../app/migrations/README.md) — migration order and
  fork workspace section.
- [Self-hosting with Docker](/self-hosting/docker) — compose owns the server process.
