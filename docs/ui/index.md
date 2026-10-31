---
title: "Admin UI overview"
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
| [Project access](./project-access)                   | Decide who reaches a project and protect production                              |
| [Users](./users)                                     | Add people, give them a role, and disable or delete accounts                     |
| [AI agents](./ai-agents)                             | Add accounts for bots and CI, and create or replace their token                  |
| [Groups](./groups)                                   | Give a team access to projects at once                                           |
| [Roles](./roles)                                     | Create custom roles, as root                                                     |
| [Account settings](./account)                        | Manage profile details, update passwords, and view session status                |

## Finding your way around

The sidebar groups pages into **Workspace** (Projects, Audit log), **People &
access** (Users, AI agents, Groups, Roles) and **Instance** (Status, Backups, Single
sign-on). It only lists pages your role can open, and a group with none of
them doesn't show. If you follow a link to a page your role can't open, the
console says which capability it needs instead of sending you somewhere else.

Your email and role name sit at the bottom of the sidebar. Click them for
Account, Personal tokens, the docs and Sign out.

## On phones and tablets

The UI works on screens down to phone width. Below 1024 pixels the sidebar
becomes a top bar, and its menu button opens the navigation. Tables keep the
name column and the actions; the other fields move to a second line under
each row. Dialogs open from the bottom of a phone screen and scroll, so their
buttons stay in view.

## Who can sign in

Setup creates the protected root account. People with `users.manage`, such as
admins, add other people from **Users** and AI agents from **AI agents**, and each person sees
only the projects and pages their role and grants allow. AI agents use personal
tokens and never sign in. Applications use environment-scoped runner tokens.
See [identity and access](/reference/identity).

## Where the UI fits

Use the UI and CLI for different parts of the workflow:

- Day-to-day secret editing, imports, and reviews happen comfortably in the
  browser.
- Servers, CI jobs, and scripts use the CLI and runner tokens.
- Actions from both interfaces appear in the audit log.

Start the server, open the address above, and follow the
[setup guide](./setup-and-sign-in).
