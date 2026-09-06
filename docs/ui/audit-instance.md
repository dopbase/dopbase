---
title: "Audit and instance status"
description: "Review what happened on your Dopbase server with the audit log and monitor instance status in the Admin UI."
---

# Audit and instance status

Audit shows recorded activity; Instance shows server health and an overview
of what is stored on your instance.

## The audit log

The Audit page is available to root and admin.

Dopbase records sensitive actions such as secret changes, imports, exports,
reveals, token creation and revocation, deletions, and sign-ins. The Audit page
lists the events newest first, twenty-five at a time, with a **Load more**
control for the next page.

Four filters narrow the list:

| Filter      | Answers                                      |
| ----------- | -------------------------------------------- |
| Action      | "Show me every reveal" or every delete       |
| Project     | Everything that happened inside one project  |
| Environment | One environment only                         |
| Actor       | The administrator or runner token that acted |

Changing any filter reloads the list from the start.

The event names and their meanings are documented in
[audit events](/reference/audit-events). If you are looking for the history of
one specific value, filter by environment and action rather than scrolling.

## Instance status

The Instance page reports:

- The running Dopbase version
- How long the server has been running and when the summary was updated
- Whether storage is healthy
- Whether the master key is available. If this shows a
  problem, nothing that needs encryption or decryption will work, and the
  fix lives on the host, not in the browser.

The page reports project, environment, secret, human-user, AI-agent, active
runner-token, active agent-token, and backup counts. All human roles can view
it. AI accounts also have read-only access to the summary.

Configuration still loads at startup and requires a restart to change. See
[self-hosting operations](/self-hosting/operations).

## Danger Zone

Root also sees **Danger Zone** on the Instance page. Its factory-reset action
permanently removes accounts, project data, audit history, and server backups.
Reading the status summary does not reset anything.

Read [factory reset](/self-hosting/factory-reset) before using this action.
