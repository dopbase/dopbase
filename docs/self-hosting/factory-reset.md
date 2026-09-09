---
title: "Factory reset"
description: "Understand what factory reset removes and how root confirms this destructive action in the Danger Zone."
---

# Factory reset

Factory reset clears your Dopbase accounts and project data so you can start
setup again. Only root can use it from the Admin UI or from the server host.

::: danger Full instance reset
Factory reset removes every project, environment, secret, user account, access
token, audit record, and backup from the active instance. This includes the
root account and signs everyone out. The Admin UI deletes this data. The host
command moves the complete data directory to a quarantine path.

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

### Admin UI

1. Sign in as root and open **Instance**.
2. In **Danger Zone**, choose **Review factory reset**.
3. Review the counts of accounts, projects, secrets, tokens, and backups
   that will be removed.
4. Acknowledge that the deletion is permanent.
5. Type `FACTORY RESET` exactly and enter your root password.
6. Choose **Permanently reset instance** only when you are ready.

If you are unsure, cancel before the final confirmation.

### Server host

Stop the Dopbase server, then run:

```bash
dopbase admin factory-reset
```

The command reads the local server configuration and refuses to continue if
the server still holds the database lock. It displays the exact data directory,
then requires `please-wipe-out-system` and the Dopbase root password. The
phrase is case-sensitive and does not allow extra spaces. There is no flag to
bypass the prompts.

Use `--data-dir` or `--config` when the instance does not use the default path.
The command moves the whole data directory to a timestamped quarantine path.
Every database, master-key, configuration, backup, log, and local CLI file
stored inside it moves too. Files configured outside the data directory are not
moved. Remove the quarantine directory yourself after checking that you no
longer need it.

## After reset

Everyone must use new credentials after setup. Applications and agents will
need new access tokens.

Factory reset does not uninstall the Dopbase binary. The Admin UI keeps the
deployment settings and master key. The host command moves any settings and
key stored inside the data directory, so the next start uses defaults unless
you provide configuration elsewhere. Neither method removes files saved on
client devices or securely erases the underlying disk.

Restart the server to obtain a new setup token, then follow
[setup and sign in](/ui/setup-and-sign-in).
