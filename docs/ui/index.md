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

The UI talks to the same REST API the CLI uses. Anything you set up through the
browser shows up for `dopbase run`, `dopbase export`, and the other client
commands, and the other way around.

## What you can do in the browser

| Page                                                 | What it does                                                                     |
| ---------------------------------------------------- | -------------------------------------------------------------------------------- |
| [Setup and sign in](./setup-and-sign-in)             | Claim a fresh server, create the administrator, sign in and out                  |
| [Projects and environments](./projects-environments) | Create, rename, and delete projects and environments                             |
| [Managing secrets](./managing-secrets)               | Set, reveal, and delete secrets, and edit a whole environment in a `.env` editor |
| [Import and export](./import-export)                 | Move secrets between a `.env` file and an environment                            |
| [Audit and instance status](./audit-instance)        | Read the audit log and check server health                                       |

## Who can sign in

v0.0.8 supports exactly one human administrator. The first person to reach an
uninitialized server claims it with the setup token printed at startup. After
that, only that administrator signs in. Machine work uses environment-scoped
runner tokens instead; see [identity and tokens](/reference/identity).

## Where the UI fits

The UI and the CLI cover different moments of the same work:

- Day-to-day secret editing, imports, and reviews happen comfortably in the
  browser.
- Servers, CI jobs, and scripts use the CLI and runner tokens.
- Both produce audit events, so the log tells one complete story.

Start the server, open the address above, and follow the
[setup guide](./setup-and-sign-in).
