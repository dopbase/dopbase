---
title: "Factory reset"
description: "Understand what factory reset removes and how root confirms this destructive action in the Danger Zone."
---

# Factory reset

Factory reset clears your Dopbase accounts and project data so you can start
setup again. Only root can use it, from **Instance → Danger Zone**.

::: danger Permanent deletion
Factory reset deletes every project, environment, secret, user account, access
token, audit record, and backup stored on the server. This includes the root
account and signs everyone out.

This action cannot be undone from the console. Do not continue unless you
understand what will be removed and no longer need the data.
:::

## Before you continue

- Check that you are connected to the correct instance.
- Tell other users and stop applications that depend on its secrets.
- Keep any data you need outside this instance. Backups left on the server
  will also be deleted.
- Make sure you have access to the server to complete setup afterward.

## Confirm the reset

1. Sign in as root and open **Instance**.
2. In **Danger Zone**, choose **Review factory reset**.
3. Review the counts of accounts, projects, secrets, tokens, and backups
   that will be removed.
4. Acknowledge that the deletion is permanent.
5. Type `FACTORY RESET` exactly and enter your root password.
6. Choose **Permanently reset instance** only when you are ready.

If you are unsure, cancel before the final confirmation.

## After reset

Everyone must use new credentials after setup. Applications and agents will
need new access tokens.

Factory reset does not uninstall Dopbase or change its deployment settings.
It does not remove backups you downloaded elsewhere or files saved on client
devices. It is not a secure disk-erasure tool.

Restart the server to obtain a new setup token, then follow
[setup and sign in](/ui/setup-and-sign-in).
