---
title: "Account settings"
description: "Manage your profile, update passwords, view session status, and understand offline password recovery in the Dopbase Admin UI."
---

# Account settings

The Account page lets you manage your personal profile, update your password,
and view your active session details.

Open **Account** from the user menu in the sidebar or navigation bar.

## Profile overview

The profile card displays your identity details:

- **Email**: Your account email address.
- **Role**: Your assigned permission level (`root`, `admin`, or `member`).
- **Last login**: The timestamp and relative time of your last successful sign-in.

## Session security

Dopbase tracks your authentication freshness to protect sensitive operations:

- **Active session**: Standard signed-in state.
- **Recently authenticated**: When you have re-entered your password within the
  re-authentication window. This badge indicates that you can perform sensitive
  actions like revealing secret values or exporting environments without an
  immediate password challenge.

## Changing your password

To update your password:

1. Enter your **Current password**.
2. Enter a **New password** (minimum 12 characters, maximum 128 characters).
3. Confirm the new password.
4. Click **Update password**.

Changing your password invalidates your existing sessions across other browsers
and updates your credentials immediately.

## Offline password recovery

If an administrator or user forgets their password and cannot sign in, passwords
can be reset offline directly on the machine running the Dopbase server:

```bash
dopbase admin reset-password user@example.com
```

This command interacts directly with the local SQLite database and server
configuration. For security, it requires terminal access to the host machine and
revokes active sessions for that account. See the
[command reference](/cli/commands#server-administration) for details.
