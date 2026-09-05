---
title: "Audit and instance status"
description: "Review what happened on your Dopbase server with the audit log and monitor instance status in two read-only Admin UI screens."
---

# Audit and instance status

Two read-only screens answer the questions "what happened?" and "how is the
server doing?".

## The audit log

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

The read-only Instance page reports:

- The running Dopbase version
- The server's public endpoint
- Whether the SQLite storage is healthy
- Whether the master key is available. If this shows a
  problem, nothing that needs encryption or decryption will work, and the
  fix lives on the host, not in the browser.

The page also shows that configuration changes require a restart. Dopbase reads
settings from `~/.dopbase/server.toml` and the process environment at startup.
The Admin UI does not edit these settings. See
[self-hosting operations](/self-hosting/operations) for the host configuration.

## What these pages do not do

Neither page changes server state. The audit log is append-only, and the
instance page exposes no file paths, keys, or private configuration.
