---
title: "Admin UI"
description: "The Dopbase Admin UI is the browser interface for a self-hosted server, embedded in the same executable with nothing extra to install."
---

# Admin UI

The Admin UI is the browser interface for a Dopbase server. It ships inside the
same `dopbase` executable, so there is nothing extra to install or deploy. Start
the server and open it:

```text
http://localhost:8840
```

The UI and CLI use the same REST API and server data. Changes made in the
browser are available to `dopbase run`, `dopbase export`, and other client
commands. CLI changes appear in the UI.

## What you can do in the browser

| Page                                                 | What it does                                                                     |
| ---------------------------------------------------- | -------------------------------------------------------------------------------- |
| [Setup and sign in](./setup-and-sign-in)             | Claim a fresh server, create the administrator, sign in and out                  |
| [Projects and environments](./projects-environments) | Create, rename, and delete projects and environments                             |
| [Managing secrets](./managing-secrets)               | Set, reveal, and delete secrets, and edit a whole environment in a `.env` editor |
| [Import and export](./import-export)                 | Move secrets between a `.env` file and an environment                            |
| [Backups and restoration](./backups)                 | Create and restore full encrypted server snapshots (`.dop` archives)             |
| [Audit and instance status](./audit-instance)        | Read the audit log and check server health                                       |
| [Users and AI agents](./users)                       | Manage human roles and create AI service accounts                                |
| [Account settings](./account)                        | Manage profile details, update passwords, and view session status                |

## Who can sign in

Version 0.1.3 supports root, admin, and member sign-in. Setup creates the
protected root. Root and admin create additional users from Users.
AI agents authenticate through service-account tokens. Applications use
environment-scoped runner tokens to retrieve values. See
[identity and tokens](/reference/identity) for the permission matrix.

## Where the UI fits

Use the UI and CLI for different parts of the workflow:

- Day-to-day secret editing, imports, and reviews happen comfortably in the
  browser.
- Servers, CI jobs, and scripts use the CLI and runner tokens.
- Actions from both interfaces appear in the audit log.

Start the server, open the address above, and follow the
[setup guide](./setup-and-sign-in).
