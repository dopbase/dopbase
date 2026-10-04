---
title: "Setup and sign in"
description: "Initialize Dopbase through the CLI or existing web flow, then sign in to the Admin UI."
---

# Setup and sign in

Initialize a fresh instance before starting it. The default setup runs in the
terminal and creates the protected root account:

```bash
dopbase server setup
dopbase server start
```

Enter the root email, a password of 12 to 128 characters, and its confirmation.
Then open the Admin UI and sign in. Setup does not create a CLI or browser
session. `dopbase server setup --email admin@example.com` generates a password
and prints it once instead of prompting. See [server setup](/cli/setup).

## Web setup

To use the existing browser setup or restore a backup:

```bash
dopbase server setup --web
```

This starts the foreground server and prints a one-time token and link:

```text
Dopbase setup token (shown once):
setup_<TOKEN>

Or open this link to fill it in automatically:
http://localhost:8840/setup?token=setup_<TOKEN>
```

### Claim a fresh instance

1. Open the setup link, which fills the token field, or paste the printed token.
2. Enter the root email.
3. Enter a password of 12 to 128 characters.

The token works once. Successful setup starts a browser session and redirects
to Projects. The server continues running until you stop it with Ctrl+C.
Later starts use `dopbase server start` or `server start --background`.

### Restore from a backup

1. Switch to **Restore from Backup**.
2. Select the encrypted `.dop` backup.
3. Provide the source master key if the backup belongs to another instance.
4. Choose **Restore & initialize server**.

Restoration preserves the existing flow: it restores the backup, closes setup,
and redirects to Sign in. Use the administrator credentials from the backup.

Root and admin can create additional users through [Users](./users).
Invitations and password reset by email are not available. Offline recovery
with the master key remains the fallback. See [identity and tokens](/reference/identity).

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
Changing the password signs out every session belonging to that account,
including the one that changed it. You sign back in with the new password.

Other users' sessions are unaffected by this password change.
