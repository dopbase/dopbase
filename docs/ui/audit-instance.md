---
title: "Audit and instance status"
description: "Review what happened on your Dopbase server with the audit log and monitor instance status in two read-only Admin UI screens."
---

# Audit and instance status

Two read-only screens answer the questions "what happened?" and "how is the
server doing?".

## The audit log

Every meaningful action in Dopbase creates an audit event: secret changes,
imports, exports, reveals, token creation and revocation, deletions, sign-ins.
The Audit page lists them, newest first, twenty-five at a time with a
**Load more** control for the next page.

Four filters narrow the list:

| Filter      | Answers                                          |
| ----------- | ------------------------------------------------ |
| Action      | "Show me every reveal" or every delete           |
| Project     | Everything that happened inside one project      |
| Environment | One environment only                             |
| Actor       | Who did it — the administrator or a runner token |

Changing any filter reloads the list from the start.

The event names and their meanings are documented in
[audit events](/reference/audit-events). If you are looking for the history of
one specific value, filter by environment and action rather than scrolling.

## Instance status

The Instance page reports the state of the server, read-only:

- **Version** — the running Dopbase version.
- **Endpoint** — the public address of this server.
- **Database** — whether the SQLite storage is healthy.
- **Master key** — whether the encryption key is available. If this shows a
  problem, nothing that needs encryption or decryption will work, and the
  fix lives on the host, not in the browser.

The page also states plainly that configuration is restart-only. Server
settings come from `~/.dopbase/server.toml` and the process environment, read
once at startup. The Admin UI deliberately offers no configuration editing:
changing a server's settings from its own web interface is a risk this product
declines. See [self-hosting operations](/self-hosting/operations) for what to
change on the host and how.

## What these pages do not do

Neither page changes anything. The audit log is append-only, and the instance
page exposes no file paths, keys, or private configuration. They are for
checking, not fixing.
