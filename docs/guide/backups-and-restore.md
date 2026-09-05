---
title: "Backup and disaster recovery"
description: "Complete guide to Dopbase full-system encrypted snapshots (.dop), dual-input cross-server restoration, master encryption keys, and disaster recovery."
---

# Backup and disaster recovery

Dopbase includes a native, full-system backup and disaster recovery engine. Unlike
per-environment `.env` export and import, a backup captures the complete server
state in an encrypted archive: all projects, environments, secret keys and
version histories, runner tokens, and administrator accounts.

Backups are packaged in authenticated, encrypted containers with the `.dop` file
extension and protected by your server's 32-byte master encryption key.

---

## Architecture & security model

### The `.dop` container

A `.dop` backup file is a cryptographically protected container structured as follows:

1. **Inner Archive**: A standard ZIP archive containing:
   - `dopbase.db`: A consistent SQLite snapshot produced using SQLite's online vacuum/backup API.
   - `manifest.json`: Metadata including schema version, creation timestamp, Dopbase server version, and record counts.
2. **Authenticated Encryption (AEAD)**: The entire archive is encrypted using **XChaCha20-Poly1305** with a fresh, cryptographically secure 24-byte nonce and a 16-byte authentication tag.
3. **Envelope Header**: The file starts with magic bytes `DOPBASE_BK1\0`, followed by the 24-byte nonce and the authenticated ciphertext.

```text
+-----------------------------------------------------------------------+
| Magic: "DOPBASE_BK1\0" | Nonce: 24 bytes                         |
+-----------------------------------------------------------------------+
| Encrypted Payload: Zip (dopbase.db + manifest.json)                   |
+-----------------------------------------------------------------------+
| Poly1305 Authentication Tag: 16 bytes                                 |
+-----------------------------------------------------------------------+
```

Because the outer container is fully authenticated, tampering with or modifying any byte
causes decryption to fail immediately before any database or file system changes occur.

### Root-of-trust: The master key

Every Dopbase instance is initialized with a high-entropy 256-bit (32-byte) root
encryption key saved to disk at:

```bash
~/.dopbase/master.key
```

This master key serves two critical roles:

1. It derives the encryption keys used to encrypt project secret values inside the SQLite database.
2. It encrypts and authenticates `.dop` backup containers.

> [!IMPORTANT]
> Because secret values stored inside `dopbase.db` are encrypted using keys derived
> from your instance's master key, **a `.dop` backup file cannot be decrypted or restored
> onto a new server without that original master key**.

---

## Disaster recovery scenarios

There are two primary recovery scenarios:

### Scenario 1: Same-server recovery

You are rolling back or restoring a snapshot on an existing, operational Dopbase
server that is still using its original `master.key`:

- **Required Inputs**: The `.dop` backup file only.
- **Workflow**: Dopbase uses the running server's active master key to authenticate and decrypt the `.dop` file, then replaces database tables atomically.

### Scenario 2: Cross-server migration / Fresh instance rebuild

You are spinning up a new server, rebuilding after total hardware loss, or
migrating to a new machine:

When you start a fresh Dopbase instance (`dopbase serve`), it generates a _new, unique_
master key and an empty database. If you attempt to restore your `.dop` backup using the
new server's temporary master key, decryption fails because the cryptographic keys do not match.

To solve this smoothly and securely, Dopbase provides **Dual-Input Restoration with Automatic Re-Keying**:

- **Required Inputs**:
  1. The `.dop` backup archive.
  2. The source server's **master key** (either as the `master.key` file or a 64-character hex string).
- **Automatic Re-Keying (Zero Key Collisions)**:
  1. Dopbase uses the provided source master key to decrypt and verify the `.dop` archive.
  2. Dopbase automatically **re-keys** the snapshot's database to the target server's existing master key (unwrapping and re-encrypting all secret keys and verification metadata).
  3. The target server's existing `master.key` file on disk remains **completely untouched**, preserving machine key isolation.
  4. The uploaded source key is discarded immediately from memory after decryption and re-keying.
  5. The re-keyed backup is stored on the server so that subsequent restores can run directly using the server's master key without requiring any key input.

---

## Live server requirement

> [!IMPORTANT]
> Both `dopbase backup` and `dopbase restore` require an active, reachable server
> (`server_status: connected (live)`).

Before initiating a backup or restore, Dopbase connects to the server to verify health,
ensure cryptographic services are ready, and confirm that the instance is in a safe
operational state. If the server is stopped or disconnected, the CLI halts and guides you
to start the server:

```bash
dopbase serve --background
```

---

## Command line (CLI) workflows

### 1. Creating a backup

To create a system snapshot on your active server:

```bash
# Timestamped backup saved on the server (~/.dopbase/backups/)
dopbase backup
```

To assign a custom name prefix and save a local copy to your current directory:

```bash
dopbase backup pre-v2-deploy --output ./pre-v2-deploy.dop
```

**CLI Output:**

```text
Connecting to server at http://localhost:8840...
Server status: connected (live)
Creating snapshot "pre-v2-deploy"...
  ✓ Server database snapshotted
  ✓ Encrypted archive generated (142.8 KB)
  ✓ Stored on server: pre-v2-deploy_20260905_120000.dop
Downloading backup copy to ./pre-v2-deploy.dop...
  ✓ Download complete: ./pre-v2-deploy.dop (142.8 KB)

NOTICE: This backup is encrypted with this server's master key (~/.dopbase/master.key).
To restore on a new or different server, the master key will be required.
```

### 2. Restoring on the same server

When restoring on the same machine that created the backup:

```bash
dopbase restore ./pre-v2-deploy.dop
```

The CLI prompts for confirmation before overwriting data:

```text
Connecting to server at http://localhost:8840...
Server status: connected (live)
File: ./pre-v2-deploy.dop (142.8 KB)

WARNING: Restoring will overwrite all current projects, environments, and secrets
on http://localhost:8840 with the contents of this snapshot.
Existing administrator sessions will remain valid.

Proceed with restoring from the backup? [y/N] y

Uploading backup archive...
  ✓ Archive uploaded and verified
Restoring database tables...
  ✓ Tables replaced and migrations applied
  ✓ System restored successfully from "pre-v2-deploy_20260905_120000.dop"
```

Pass `--yes` to skip the destructive confirmation prompt. Initialized restores
still require an interactive administrator password confirmation:

```bash
dopbase restore ./pre-v2-deploy.dop --yes
```

### 3. Restoring on a new server (Cross-Server Migration)

When restoring onto a newly installed or different Dopbase server, supply the original
master key using the `-k, --key` option:

```bash
# Using the master.key file from the source server (and the target setup token on first run)
dopbase restore ./pre-v2-deploy.dop --key /path/to/source/master.key --setup-token dbs_...

# Or using the 64-character hex string
dopbase restore ./pre-v2-deploy.dop --key 4a2f8b9c01234567... --setup-token dbs_...
```

Dopbase verifies and decrypts the backup against the provided key, automatically re-keys
the database to the new server's existing master key, and restores the database tables
without modifying the target server's `~/.dopbase/master.key` file.

---

## Admin UI workflows

The Dopbase Admin UI provides point-and-click management for backups and disaster recovery.

### Downloading your master key

You should store a secure copy of your instance's master key in your team's password
manager or cold storage vault.

1. Navigate to **Backups** in the top navigation bar.
2. Click **Download Master Key** in the upper-right actions toolbar.
3. If your administrator session requires re-authentication, enter your password to confirm.
4. Your browser downloads `master.key` (a 32-byte binary file).

### Restoring on first-run setup

When setting up a fresh Dopbase instance for the first time, navigate to the Web UI at `http://localhost:8840`:

1. On the welcome screen, switch to the **Restore from Backup** tab.
2. Enter the one-time setup token printed by the target server.
3. Select your `.dop` backup file.
4. Under **Source Master Key**, upload the `master.key` file from your previous server (or paste its 64-character hex key).
5. Click **Restore System**. Dopbase re-keys all secrets to the new server's master key.
6. Once complete, you are redirected to the login screen where you can immediately sign in with your restored administrator credentials.

### Backups dashboard

From **Admin UI > Backups**, administrators can:

- **Create snapshots**: Click **New backup** to take a consistent snapshot at any time.
- **Download snapshots**: Click the download icon next to any backup in the list.
- **Upload snapshots**: Click **Upload backup** to add a previously taken `.dop` file. If the file is from another server, provide its master key in the upload dialog. Dopbase will safely re-key the snapshot using this server's master key so it can be restored anytime.
- **Restore snapshots**: Click **Restore** next to any snapshot in the table, review the warning, type `RESTORE`, and confirm.

---

## Best practices for disaster recovery

1. **Back up your master key separately**:
   Never store your `master.key` in the same directory or archive as your `.dop` backup files. Store your master key in an encrypted password manager (1Password, Bitwarden) or hardware security module (HSM).
2. **Automate off-site replication**:
   Use `dopbase backup [name] --output /mnt/secure-backups/daily.dop` in an automated cron job or systemd timer, and replicate the `.dop` file to offsite S3 or cloud storage.
3. **Periodically practice recovery drills**:
   Spin up a disposable container or local test instance and execute `dopbase restore <file.dop> --key <key>` to ensure your team is familiar with the disaster recovery procedure.
