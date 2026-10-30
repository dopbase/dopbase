---
title: "Admin UI: account settings"
description: "Manage your profile, password, and personal tokens, view session status, and understand offline password recovery in the Dopbase Admin UI."
---

# Account settings

The Account page lets you manage your personal profile, update your password,
and view your active session details.

Click your email at the bottom of the sidebar to open the account menu, then
choose **Account**. **Personal tokens** in the same menu jumps to that section
of the page.

## Profile overview

The profile card displays your identity details:

- **Email**: Your account email address.
- **Role**: The name of your role, such as Admin, Member, or a custom role.
- **Last login**: The timestamp and relative time of your last successful sign-in.

## Session security

Dopbase tracks your authentication freshness to protect sensitive operations:

- **Active session**: Standard signed-in state.
- **Recently authenticated**: When you have re-entered your password within the
  re-authentication window. This badge indicates that you can perform sensitive
  actions like revealing secret values or exporting environments without an
  immediate password challenge.

## Personal tokens

A personal token lets a script or an AI agent on your machine act as you,
through `DOPBASE_TOKEN`. It never reaches more than you do: if you lose access
to a project, your tokens lose it too.

Choose **New token** and pick:

- a **name**, such as the machine or agent that will use it;
- what it **can** do: read names only, read values, or also change secrets,
  up to what your role allows;
- **where**: every project you reach, or one project;
- when it **expires**. Personal tokens always expire.

Dopbase shows the token once, so copy it straight into a secret store. The card
lists your active tokens; choose **Revoke** to stop one immediately. If your
role's tokens read the audit log, as an auditor's do, the card creates audit
tokens instead, for a SIEM or another log collector.

The card isn't shown for root, which has no personal tokens, or for roles
without `tokens.personal`. You can also use `dopbase token personal` in the
CLI. See [AI agent access](/guide/ai-agent-access) for handing a token to an
agent.

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
