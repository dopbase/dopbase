---
title: "Admin UI: setup and sign in"
description: "Set up a fresh Dopbase instance in the browser, create the protected root account, then sign in to the Admin UI and manage your sessions."
---

# Setup and sign in

Start the server:

```bash
dopbase server start
```

Fresh storage enters protected browser setup automatically. An initialized
instance starts normally and shows Sign in.

For startup without browser setup, supply `DOPBASE_ROOT_EMAIL`. Dopbase creates
root on fresh storage, prints a generated password once, and continues serving.
Sign in with those credentials. Existing instances ignore the bootstrap email.

Guided terminal setup remains available with `dopbase server setup`, followed
by startup. It creates no login session. See [server setup](/cli/setup) for
background output, disabled web UI settings, and interrupted-reset recovery.

## Web setup

To use the existing browser setup or restore a backup:

```bash
dopbase server setup --web
```

Explicit web setup and automatic fresh startup without a root email print the
same one-time token and link. Explicit `--web` also enables a disabled UI for
that run:

```text
Dopbase setup token (shown once):
setup_<TOKEN>

Or open this link to fill it in automatically:
http://localhost:8840/setup?token=setup_<TOKEN>
```

### Claim a fresh instance

1. Open the setup link, which fills the token field, or paste the printed token.
2. Enter the root email.
3. Enter a password of 12 to 128 characters. A meter under the field rates it;
   a long passphrase from a password manager rates best.
4. Choose **Create root account**.

The token works once. A checklist beside the form shows where you are: root
account, recovery kit, then sign-in for your team and a first project, which
you do from the console.

### Save the recovery kit

Right after root is created, setup shows the server's master key. Every secret
and every backup is encrypted with it, so restoring a backup on another
machine needs it.

1. Choose **Download recovery kit**. It saves a text file with the master key
   in hex, the server's address, root's email, and the restore steps. You can
   also show the key, or copy it, with the buttons beside it.
2. Store the file somewhere other than the server, such as a password manager.
3. Tick **I've stored the recovery kit outside this server** and choose
   **Continue**. Continue stays off until you tick it.

You land on Projects. Root can download the master key again from
[Status](./audit-instance) or [Backups](./backups) while the server runs.

The server keeps running until you stop it with Ctrl+C. Later starts use
`dopbase server start` or `server start --background`.

### Restore from a backup

1. Switch to **Restore from backup**.
2. Paste the setup token and choose the encrypted `.dop` backup.
3. If the backup comes from another server, give its master key: the
   `master.key` file, or the 64-character key from its recovery kit.
4. Choose **Restore backup**.

Restoring brings back every project, account, and secret, closes setup, and
sends you to Sign in. Use an account from the backup.

Root and admin can create additional users through [Users](./users).
Invitations and password reset by email are not available. Offline recovery
with the master key remains the fallback. See [identity and tokens](/reference/identity).

## Signing in

Sign in with the email and password you chose during setup, or the generated
password returned by startup or explicit CLI setup. The page names the server
you are signing in to, and its footer shows the Dopbase version.

- A wrong email or a wrong password shows the same message, so the page never
  reveals whether an account exists. From the second wrong try it says how many
  are left before a 15-minute pause, and during the pause how long to wait.
- **Caps Lock is on** appears under the password while Caps Lock is on.
- The eye button shows or hides the password as you type.
- When single sign-on is on, **Continue with** the provider sits above the
  form. When root has turned passwords off, the form is replaced by a note that
  the server uses that provider. Root can still choose **Signing in as root?
  Use your password**.
- **Forgot password?** and **Trouble signing in?** open a page that explains
  who can help: an admin resets your password from Users, and root uses
  `dopbase admin reset-password` on the server. Dopbase doesn't send reset
  emails.
- If you followed a link to a specific page, signing in returns you there.

## Approving a CLI sign-in

`dopbase login --browser` opens a page that shows the code from the terminal,
character by character. Before you approve, it lists the machine name, the
client (for example `dopbase 0.2.0 on macOS`), the address the request came
from, when it was made, and a countdown to when it expires. Beside it, a
mirror shows what the terminal should print, so you can compare the two. The
page also says which account will be signed in and for how long.

- **Approve** signs that terminal in with your access.
- **Deny**, or **It doesn't match**, refuses it. Nothing is shared with the
  terminal.
- When the countdown reaches zero, the request can't be approved; run
  `dopbase login --browser` again for a new code.

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
