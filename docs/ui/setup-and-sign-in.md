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

Open the Admin UI and you land on the setup page. Enter three things:

1. The setup token from the server output. With `dopbase serve --background`,
   the token is also written to `~/.dopbase/serve.log`.
2. An email address for the administrator account.
3. A password of at least 12 characters (128 at most).

The token works once. After setup completes, visiting `/setup` redirects to the
workspace. If the server reports that setup is complete, sign in instead.

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
