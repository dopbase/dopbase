---
title: "Admin UI: backups and restore"
description: "Manage system-wide encrypted backups, download and upload .dop archives, and restore server snapshots from the Admin UI."
---

# Backups and restoration

Only root can use this page. Version {{version}} backups include human users,
service accounts, and tokens. Backups from earlier releases are not supported.
:::

The **Backups** page in the Admin UI lets you create, inspect, download, delete,
and restore point-in-time system snapshots. Unlike per-environment `.env` export
and import, a backup captures the complete server state: all projects,
environments, secrets, version histories, runner tokens, and administrator
accounts.

## Navigating to Backups

From anywhere in the Admin UI, click **Backups** under **Instance** in the
sidebar. Only root sees it.

The page shows:

- A banner reminding you to keep the master key with your backups, with
  **Download master key**.
- A table of the `.dop` archives stored on the server (`~/.dopbase/backups/`),
  with each one's size and when it was made. Each row's **⋯** menu holds
  **Download**, **Restore**, and **Delete backup**. On a phone the table
  becomes cards with the same menu.
- **Create backup** and **Upload** at the top.

## Creating a backup

1. Choose **Create backup**.
2. Optionally enter a name prefix (e.g. `pre-v2-deploy`). Left blank, Dopbase
   generates a timestamped name (`dopbase_backup_YYYYMMDD_HHMMSS.dop`).
3. Confirm.

The server creates a consistent SQLite snapshot, wraps it in an
XChaCha20-Poly1305 envelope using the server's master key, and stores it in the
server backups directory. The new backup appears in the list right away.

## Downloading the master encryption key

Every Dopbase instance generates a 32-byte master encryption key stored at
`~/.dopbase/master.key`. This key encrypts all `.dop` backup containers and
derives the keys that protect stored project secrets.

Choose **Download master key** in the banner, or **Download** on the master
key card on [Status](./audit-instance#instance-status), and confirm your
password if asked. Your browser saves `master.key`. Store it in a password
manager or vault, not on the server. The recovery kit saved during
[setup](./setup-and-sign-in#save-the-recovery-kit) holds the same key in hex.

> [!IMPORTANT]
> To restore a `.dop` backup on a new or different Dopbase server, the matching
> master key is required. Without it, the new server cannot decrypt the backup
> or the secrets stored within it.

## Downloading a backup

Choose **Download** in a backup's **⋯** menu to save the encrypted `.dop`
archive. Because it is encrypted with the server's master key, it is safe to
keep in off-site or cold storage.

## Uploading a backup

You can upload `.dop` files downloaded earlier or moved from another server:

1. Choose **Upload**.
2. Pick the `.dop` file.
3. If the backup was made on a different server, give its master key: its
   `master.key` file or the 64-character hex key.
4. Confirm.

Dopbase checks the archive's authentication tag with the given master key, or
this server's, before adding it to the list. Corrupted, modified, or
incompatible backups are rejected.

## Restoring from a backup

Restoring replaces the entire server state with the snapshot contents:

1. Find the backup in the table, or upload one first.
2. Choose **Restore** in its **⋯** menu.
3. The dialog warns that current projects, environments, and secrets will be
   replaced. Type `RESTORE` and confirm.

During restoration:

- The server acquires an exclusive write lock on the database.
- Existing database tables are replaced with the snapshot data.
- Any outstanding schema migrations are executed automatically.
- If a cross-server master key was supplied, Dopbase re-keys the restored secret
  metadata to the target server's existing `master.key` without replacing it.
- An audit record (`backup.restored`) is written to the audit log.

> [!WARNING]
> Restoring overwrites existing data with the snapshot contents. Secrets created
> or modified since the snapshot was taken will be lost unless you have exported
> them or backed them up previously.

## Deleting a backup

Choose **Delete backup** in its **⋯** menu and type the file name to confirm.
The file is removed from this server; copies you downloaded aren't affected.

## Restoring on first-run setup

On fresh storage, `dopbase server start` without `DOPBASE_ROOT_EMAIL` opens
the same protected setup flow. Explicit `server setup --web` remains useful
when an email is set or the web UI is disabled. During restoration, avoid
automatic root creation so the backup supplies the original accounts.

When initializing a brand-new Dopbase instance, start the existing web setup
listener with `dopbase server setup --web`. It prints a setup link and token.

1. Open the setup link in your browser.
2. Switch to **Restore from backup**.
3. Enter the one-time setup token printed by the server.
4. Choose your `.dop` backup file.
5. If migrating from another server, give its master key: the `master.key`
   file, or the 64-character key from its recovery kit.
6. Choose **Restore backup**.
7. Once complete, you will be redirected to the sign-in screen to log in with your restored administrator credentials.
