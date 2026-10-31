---
title: "Admin UI: audit log and instance status"
description: "Review what happened on your Dopbase server with the audit log and monitor instance status in the Admin UI."
---

# Audit and instance status

Audit log shows recorded activity. Status shows server health and an overview
of what is stored on your instance. Both sit in the sidebar.

## The audit log

Everyone can open the Audit log page. What it lists depends on your role:

| Role             | Sees                                                            |
| ---------------- | --------------------------------------------------------------- |
| Root and Auditor | Every event (`audit.read_all`)                                  |
| Admin            | Settings events and events on projects you reach (`audit.read`) |
| Everyone else    | Your own actions                                                |

The line under the page title says which applies to you.

Dopbase records sensitive actions such as secret changes, imports, exports,
reveals, token creation and revocation, deletions, and sign-ins. The page lists
them newest first, twenty-five at a time, with **Load more** for the next page.
Each row shows when it happened, the action, who did it (with a tag when it
came through a personal or runner token), and where. On a phone the rows
become cards.

Three filter chips narrow the list. Choose **+ Action**, **+ Actor**, or
**+ Where** and pick a value; the chip then reads, say, `Action:
secret.revealed`, and its **×** clears it. Filters live in the address, so a
filtered view can be shared, and the **Activity** links on agent and account
pages open the log already filtered to that actor.

| Filter | Answers                                          |
| ------ | ------------------------------------------------ |
| Action | "Show me every reveal" or every delete           |
| Actor  | The person, AI agent, or runner token that acted |
| Where  | One project, or one environment inside it        |

### An event's details

Choose a row to open its details on the right: the actor, the credential it
used, where it happened, the secret key or resource, and the address it came
from. Below that, the whole event as JSON with a copy button. Events never
contain secret values.

People with `audit.verify`, such as root and auditors, also see whether the
hash chain checks out up to that event, for example `Chain verified up to
#4812 · sha256 9f2c…a71e`, or where it breaks. **Verify chain** at the top
checks the whole log at once.

The event names and their meanings are documented in
[audit events](/reference/audit-events).

## Instance status

The Status page lives at `/status`, and old `/instance` links still open it.
Four cards at the top show the running version, whether storage is healthy,
whether the master key is available, and how long the server has been up. If
the master key shows a problem, nothing that needs encryption or decryption
works, and the fix lives on the host, not in the browser. **Refresh** reloads
them.

Root sees **Download** on the master key card. It saves `master.key`, the same
file the setup recovery kit and the Backups page give you. You confirm your
password first.

**What's stored** counts projects, environments, secrets, people, AI agents,
active runner tokens, active AI agent tokens, and backups.

Configuration still loads at startup and requires a restart to change. See
[self-hosting operations](/self-hosting/operations).

## Factory reset

Root also sees a **Factory reset** card at the bottom. It deletes everything
counted above, plus sessions and the audit log, and the server goes back to
first-run setup. Configuration and the master key stay, and backups you
downloaded aren't touched. **Reset this instance** stays off until you tick
**I've downloaded a backup and the master key**, type `FACTORY RESET`, and
enter the root password.

Read [factory reset](/self-hosting/factory-reset) before using this action.
