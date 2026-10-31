---
title: "Admin UI: account settings"
description: "Manage your profile, password, and personal tokens, view session status, and understand offline password recovery in the Dopbase Admin UI."
---

# Account settings

Click your email at the bottom of the sidebar to open the account menu, then
choose **Account**. **Personal tokens** in the same menu opens that tab
directly.

The page has two tabs, each with its own address:

- **Profile & sign-in** (`/account`)
- **Personal tokens** (`/account/tokens`), or **Audit tokens**
  (`/account/audit-tokens`) when your role's tokens read the audit log, as an
  auditor's do

Root, which has no personal tokens, and roles without `tokens.personal` see
the profile only.

## Profile & sign-in

The **Profile** card shows your email, your role, the groups you belong to,
and when you last signed in.

The **Single sign-on** card links your account to the provider, or unlinks
it, when single sign-on is on.

The **Password** card opens a dialog to change your password: your current
password, then a new one of 12 to 128 characters, twice. Changing it signs you
out everywhere, this browser too, and you sign back in with the new one. If you
only ever signed in with single sign-on, the button sets a password instead.

## Personal tokens

A personal token lets a script or an AI agent on your machine act as you,
through `DOPBASE_TOKEN`. It never reaches more than you do: if you lose access
to a project, your tokens lose it too.

The tab lists your tokens with what each can do, where, when it was last used,
and when it expires. A row's **⋯** menu revokes it.

Choose **New token**:

- **Start from**: **AI agent** picks Names only for 30 days, **Script** picks
  Read for 90 days. Change anything and it becomes **Custom**.
- **Name**: lowercase letters, numbers, and hyphens, such as `claude-laptop`.
- **Can**: Names only, Read, or Write. Levels above what your role allows are
  shown but can't be picked.
- **Where**: every project you reach, one project, or one environment.
- **Expires**: 7, 30, or 90 days, or a custom number of days, up to your role's
  limit. Personal tokens always expire.

A line at the bottom sums it up, for example "claude-laptop can list names in
payments until Nov 26, 2026, and never more than you can." Dopbase shows the
token once, so copy it straight into a secret store.

## Audit tokens

Auditors' tokens read the audit log only, for a SIEM or another log collector.
They can't see projects or secrets. The tab lists them with when each was last
used and when it expires. A row's **⋯** menu copies the name, opens its
activity in the audit log, or revokes it. A **Pull events** snippet shows the
`curl` command to read the log with the token.

You can also use `dopbase token personal` in the CLI. See
[AI agent access](/guide/ai-agent-access) for handing a token to an agent.

## Forgot your password?

Ask an admin to set a new one from Users. Root resets a password on the
server with `dopbase admin reset-password <email>`; see the
[command reference](/cli/commands#server-administration). The sign-in page's
**Forgot password?** link says the same.
