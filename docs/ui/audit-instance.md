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
It runs the server's health checks and refreshes every 30 seconds while it is
open. If a refresh fails, the page keeps the last results and says what time
they are from. Health checks need the `instance.view` capability; without it
the page shows only what's stored.

The banner at the top sums up the checks: how many need attention and how
many passed, or that everything looks good. It also shows the version and how
long the server has been up, and, when the `update_check` setting is on,
whether a newer release is out. When the master key file on disk no longer
matches the key the server is using, the banner turns red and says so first.
The server keeps working on the key it loaded, but it won't start again until
the right file is back in place.

Below the banner, four cards show the readings behind the checks:

- **Database**: how long a query takes and how big the database is.
- **Master key**: whether the key file still matches the key in use.
- **Disk**: free space where the data lives.
- **Audit log**: whether the hash chain verified, and when. The chain is
  walked when you open the page, and the result is reused for up to 10
  minutes.

**Last 24 hours** sits beside them: secret reads with an hourly chart, failed
sign-ins, requests refused by the sign-in limits, and live browser and CLI
sessions. The emails and addresses behind failed sign-ins need `audit.read`.

**Needs attention** lists the failing checks, most severe first, each with a
button that fixes it when you can use one: **Create backup**, **Review
tokens**, **How to set it**, and so on. Passing checks fold into one line;
**Show all** opens them. When every check passes, the card lists them all
with what each one saw. Root also gets **Download key in use** when the key
file changed. You confirm your password first.

**Run checks** runs everything again now, including the audit chain walk. It
works once every 10 seconds. **Copy diagnostics** copies a plain-text report
of the page to paste into an issue.

**What's stored** counts projects, environments, secrets, people, AI agents,
active runner tokens, active AI agent tokens, and backups.

Configuration still loads at startup and requires a restart to change. See
[self-hosting operations](/self-hosting/operations).

## Factory reset

Root also sees a **Factory reset** card at the bottom. It deletes everything
counted above, plus sessions and the audit log, and the server goes back to
first-run setup. Configuration and the master key stay, and backups you
downloaded aren't touched. The steps stay folded until you choose **Show
reset steps**. **Reset this instance** stays off until you tick
**I've downloaded a backup and the master key**, type `FACTORY RESET`, and
enter the root password.

Read [factory reset](/self-hosting/factory-reset) before using this action.
