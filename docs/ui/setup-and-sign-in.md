---
title: "Setup and sign in"
description: "Claim a fresh Dopbase server with the one-time setup token, create the administrator account, and sign in to the Admin UI."
---

# Setup and sign in

A fresh Dopbase server has no users. The first visit to the Admin UI walks you
through claiming it.

## First-run setup

When the server starts for the first time it prints a one-time setup token:

```text
Setup token: dbsetup_xxxxxxxxxxxxxxxx
```

Open the Admin UI and you land on the setup page (`/setup`). The page provides two setup options:

### Option 1: Claim a fresh instance

1. Enter the one-time **setup token** from the server startup output. With
   `dopbase serve --background`, the token is also written to `~/.dopbase/serve.log`.
2. Provide an **email address** for the administrator account.
3. Enter a secure **password** of at least 12 characters (128 at most).

The setup token works once. Once claimed, the server is initialized and redirects to the workspace.

### Option 2: Restore from an existing backup

If you are restoring an existing installation or spinning up a replacement instance from a backup:

1. Switch to the **Restore from Backup** tab.
2. Select or drag-and-drop your encrypted `.dop` backup file.
3. Click **Restore & initialize server**.

The server validates the backup archive against its master encryption key, restores all projects, environments, secrets, runner tokens, and administrator credentials, and closes the setup window. You are then redirected to the sign-in page to log in using the administrator credentials restored from the backup.

There is no second administrator, no invitation flow, and no password reset by
email. Offline recovery with the master key is the fallback; see
[identity and tokens](/reference/identity).

## Signing in

Sign in with the email and password you chose during setup.

Sign-in behavior:

- Too many failed attempts triggers a rate limit. Wait a moment and try again.
- A wrong email or a wrong password shows the same message. The login screen
  does not reveal whether an account exists.
- If you followed a link to a specific page, signing in returns you there.
- The login screen shows whether the server is reachable. That status check is
  public and carries no secret data.

## Sessions

Signing in creates a browser session stored in a cookie. Sessions expire after
eight hours idle or twenty-four hours total, whichever comes first. When a
session ends, the next navigation sends you back to the login screen and keeps
your destination.

Logging out revokes the server session and clears it from the browser.

## Your account

The Account page shows the signed-in email and lets you change the password.
Changing the password signs out every human session everywhere, including the
one that changed it. You sign back in with the new password.

Any other signed-in browser is also signed out.
