---
title: "Backups and restoration"
description: "Manage system-wide encrypted backups, download and upload .dop archives, and restore server snapshots from the Admin UI."
---

# Backups and restoration

The **Backups** page in the Admin UI lets you create, inspect, download, delete,
and restore point-in-time system snapshots. Unlike per-environment `.env` export
and import, a backup captures the complete server state: all projects,
environments, secrets, version histories, runner tokens, and administrator
accounts.

## Navigating to Backups

From anywhere in the Admin UI, click **Backups** in the top navigation bar.

The Backups dashboard displays:

- A list of all available `.dop` backup archives stored on the server (`~/.dopbase/backups/`).
- File size, creation timestamp, and quick action controls (Download, Restore, Delete) for each backup.
- A button to **Create Backup** on demand.
- A button to **Upload Backup** from your computer.

## Creating a backup

1. Click **Create Backup**.
2. Optionally enter a custom name prefix (e.g. `pre-v2-deploy`). If left blank,
   Dopbase automatically generates a timestamped name
   (`dopbase_backup_YYYYMMDD_HHMMSS.dop`).
3. Click **Create Snapshot**.

The server creates a consistent SQLite snapshot, wraps it in an XChaCha20-Poly1305
envelope using the server's master key, and stores it in the server backups
directory. The new backup appears immediately in the backups list.

## Downloading the master encryption key

Every Dopbase instance generates a 32-byte master encryption key stored at
`~/.dopbase/master.key`. This key encrypts all `.dop` backup containers and derives
the keys that protect stored project secrets.

To download a copy for safe keeping:

1. Click **Download Master Key** in the header actions toolbar.
2. If your session requires re-authentication, enter your administrator password to confirm.
3. Your browser downloads `master.key`. Store this file securely in a password manager or vault.

> [!IMPORTANT]
> To restore a `.dop` backup on a new or different Dopbase server, the matching
> `master.key` is required. Without it, the new server cannot decrypt the backup
> or the secrets stored within it.

## Downloading a backup

Click the **Download** icon next to any backup in the list to download the
encrypted `.dop` archive to your local device. Because the archive is encrypted
with the server's master key, it is safe to store in off-site backup storage or
cold storage.

## Uploading a backup

You can upload `.dop` files previously downloaded or transferred from another
server:

1. Click **Upload Backup**.
2. Select or drag-and-drop the `.dop` file from your device.
3. **Cross-Server Restores (Optional)**: If the backup was created on a different
   Dopbase server, provide its master key (either by selecting its `master.key` file
   or pasting its 64-character hex key).
4. Click **Upload & Verify**.

Dopbase verifies the cryptographic authentication tag of the archive using the
provided master key (or this server's master key) before accepting it into the
available backups list. Corrupted, modified, or incompatible backups are rejected
immediately.

## Restoring from a backup

Restoring replaces the entire server state with the snapshot contents:

1. Locate the desired backup in the table, or upload a `.dop` file via **Upload Backup**.
2. Click the **Restore** button next to the backup.
3. A confirmation dialog appears, warning that restoring will replace current
   projects, environments, and secrets with the snapshot data.
4. Type `RESTORE` to confirm and click **Restore Snapshot**.

During restoration:

- The server acquires an exclusive write lock on the database.
- Existing database tables are replaced with the snapshot data.
- Any outstanding schema migrations are executed automatically.
- If a cross-server master key was supplied, Dopbase re-keys the restored secret
  metadata to the target server's existing `master.key` without replacing it.
- Your current administrator session is preserved, allowing you to continue
  working without needing to log in again.
- An audit record (`backup.restored`) is written to the audit log.

> [!WARNING]
> Restoring overwrites existing data with the snapshot contents. Secrets created
> or modified since the snapshot was taken will be lost unless you have exported
> them or backed them up previously.

## Restoring on first-run setup

When initializing a brand-new Dopbase instance:

1. Open `http://localhost:8840` in your browser.
2. Select the **Restore from Backup** tab.
3. Enter the one-time setup token printed by the server.
4. Choose your `.dop` backup file.
5. If migrating from another server, select its `master.key` file or paste the 64-character hex key under **Source Master Key**.
6. Click **Restore System**.
7. Once complete, you will be redirected to the sign-in screen to log in with your restored administrator credentials.
