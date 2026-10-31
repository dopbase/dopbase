---
title: "Admin UI: roles"
description: "Create custom roles that decide what people and AI agents may do and how deep their grants reach, as root in the Dopbase Admin UI."
---

# Roles

A role decides what its holders may do on the instance and how deep into
project data their grants can ever reach. Which projects they reach still
comes from grants. Root opens **Roles** from the sidebar; anyone else who
follows a link to it sees a page saying they need `roles.manage`.

The table lists each role with a `built-in` or `custom` tag, the deepest level
it reaches, how many capabilities it holds, how holders sign in, and how many
accounts hold it.

## Built-in roles

Admin, Member, Auditor, and AI agent are built in. They can't be changed or
deleted. Opening one shows its settings read only. Choose **Duplicate** to
start a custom role from them. See
[identity and access](/reference/identity#roles-and-capabilities) for what
each one holds.

## Create a role

Choose **New role**, or **Duplicate** in a role's **⋯** menu, and fill in:

- **Basics**: a name and an optional description, shown in the role picker on
  Users.
- **Project data**: the deepest level any grant can give holders. Higher
  grants are cut down to this level. A Viewer might stop at Read.
- **What holders can do on this instance**: the capabilities, grouped under
  People, Projects, Audit, Tokens, and Instance. Each shows its name, such as
  `users.view`. Root's own powers, such as backups and single sign-on, aren't
  listed.
- **Personal tokens**: appears when the role holds personal tokens. Pick
  whether tokens reach project data or the audit log only, what they can do
  (Names only or Read), and their longest lifetime: 30, 90, or 365 days.
- **Signs in with**: password or single sign-on, for people, or token only,
  for AI agents and CI. Token-only accounts always have an owner. This is
  picked once and can't change later.

Then choose **Create role**.

Some examples:

| Role       | Holders can                                                                    | Reaches at most | Tokens                      |
| ---------- | ------------------------------------------------------------------------------ | --------------- | --------------------------- |
| Viewer     | Hold personal tokens                                                           | Read            | Project data, read          |
| Supervisor | See people, read the audit log for their projects, see who can reach a project | Read            | None                        |
| CI bot     | Hold personal tokens (token only)                                              | Read            | Project data, read, 30 days |

## Change or delete a role

Open a custom role, or choose **Edit** in its **⋯** menu, change it, and
choose **Save role**. Everyone holding it gets the new settings on their next
request. Lowering how far the role reaches lowers its tokens' limit too.

Choose **Delete role** and type the role's name. A role someone holds can't be
deleted: the dialog says how many accounts hold it, so give them another role
first.

## Who can give a role

On **Users**, people can pick only the roles within their own: every
capability of the role must be one they hold, and its level no deeper than
theirs. Root can give any role.

Role changes are recorded in the audit log as `role.created`, `role.updated`,
and `role.deleted`.
