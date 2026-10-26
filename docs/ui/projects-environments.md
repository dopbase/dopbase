---
title: "Admin UI: projects and environments"
description: "Browse projects and environments in the Dopbase Admin UI, share direct links to an environment, and manage its runner tokens."
---

# Projects and environments

Projects is the main screen of the Admin UI. It shows the model described
in [projects, environments, and secrets](/guide/projects-environments-secrets):
a list of projects on the left, the selected project's environments, and the
secrets of the selected environment.

You see only the projects and environments you have access to, and only the
buttons your access allows. Someone who reads names only, for example, sees the
secrets list without **Reveal**, **Edit**, or **Add secret**. See
[project access](./project-access) for how access is given.

## The project rail

The left rail lists every project you reach. When you arrive with nothing selected,
Dopbase opens the first project and its first environment so you land on
something usable instead of an empty screen.

- **Create a project** with the new-project control in the rail, if your role
  can create projects. Names are unique on the server, so pick the
  application's name: `payment-service`, not `backend`. You become the new
  project's project manager.
- **Access**: the shield beside a project you manage opens its
  [Access](./project-access) page.
- **Rename** a project from its menu. Existing links that used the old name
  stop working, because names are part of the address.
- **Delete** a project and Dopbase shows what goes with it (every environment
  and every secret inside) before you confirm. Deletion cannot be undone.

On screens narrower than 1024 pixels the rail is hidden. The current project
and environment appear at the top of the page as `project / environment`; tap
them to open the rail, and it closes once you pick an environment. On touch
screens the rename and delete buttons are always shown, because there is no
hover to reveal them.

## Environments

Selecting a project lists its environments. Each one holds its own set of
secret values, so `DATABASE_URL` in `development` can point somewhere entirely
different from `DATABASE_URL` in `production`.

- **Create** an environment with a name such as `development`, `staging`, or
  `production`.
- **Rename** it from its menu.
- **Delete** it and Dopbase counts what will be removed, secrets and runner
  tokens alike, before you confirm.

Adding environments needs project manager on the project. Renaming, deleting,
and runner tokens need manage on the environment. A lock marks a
**protected** environment, which only a grant on the environment itself or a
project manager reaches.

## Addresses you can share

Selection lives in the URL, not in some hidden app state:

```text
/projects/p/payment-service/e/env_482731
```

The project part uses the name. The environment part uses its immutable ID.
Bookmark a production environment, paste a link to a teammate, or put the URL
in a runbook. The link opens the same place later.

The environment ID in the address is the same ID the CLI accepts. The selected
environment header shows the ID with a copy button for `dopbase run` and
deployment configuration.

## Runner tokens

Each environment you manage has a **Tokens** tab next to its secrets. This is
where machine identities come from:

- **Create** a token with a name such as `production-server`. Choose no expiry,
  a preset lifetime, or a custom number of hours or days up to three years. Dopbase shows
  the plaintext token once. Copy it before closing the dialog because Dopbase
  cannot display it again.
- **Check expiry** in the token list. Expired tokens cannot authenticate. Create
  a replacement if you need a different lifetime.
- **Revoke** a token when a server is decommissioned or a token may have
  leaked. Revocation is immediate.

A runner token can read and export the values of its assigned environment.
Use `dopbase run` to inject them or `dopbase export` to write a file. It cannot
change secrets or access another environment. Give each deployed workload its own token. See
[identity and access](/reference/identity) for the full rules.
