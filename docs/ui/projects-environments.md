---
title: "Projects and environments"
description: "How the Admin UI Workspace organizes projects, environments, and secrets, and how to navigate between them."
---

# Projects and environments

The Workspace is the main screen of the Admin UI. It shows the model described
in [projects, environments, and secrets](/guide/projects-environments-secrets):
a list of projects on the left, the selected project's environments, and the
secrets of the selected environment.

## The project rail

The left rail lists every project. When you arrive with nothing selected,
Dopbase opens the first project and its first environment so you land on
something usable instead of an empty screen.

- **Create a project** with the new-project control in the rail. Names are
  unique on the server, so pick the application's name: `payment-service`,
  not `backend`.
- **Rename** a project from its menu. Existing links that used the old name
  stop working, because names are part of the address.
- **Delete** a project and Dopbase shows what goes with it (every environment
  and every secret inside) before you confirm. Deletion cannot be undone.

## Environments

Selecting a project lists its environments. Each one holds its own set of
secret values, so `DATABASE_URL` in `development` can point somewhere entirely
different from `DATABASE_URL` in `production`.

- **Create** an environment with a name such as `development`, `staging`, or
  `production`.
- **Rename** it from its menu.
- **Delete** it and Dopbase counts what will be removed, secrets and runner
  tokens alike, before you confirm.

## Addresses you can share

Selection lives in the URL, not in some hidden app state:

```text
/workspace/p/payment-service/e/env_01ABCDEF...
```

The project part uses the name; the environment part uses its immutable ID.
Bookmark a production environment, paste a link to a teammate, or put the URL
in a runbook. The link opens the same place later.

The environment ID in the address is the same ID the CLI accepts. Copy it from
the URL when a `dopbase run` or deployment config needs it.

## Runner tokens

Each environment has a **Tokens** tab next to its secrets. This is where
machine identities come from:

- **Create** a token with a name such as `production-server`. Dopbase shows
  the plaintext token once. Copy it before closing the dialog because Dopbase
  cannot display it again.
- **Revoke** a token when a server is decommissioned or a token may have
  leaked. Revocation is immediate.

A runner token can read the values of its one environment so `dopbase run` can
inject them. It cannot change secrets, export them, or see any other
environment. Give each deployed workload its own token. See
[identity and tokens](/reference/identity) for the full rules.
