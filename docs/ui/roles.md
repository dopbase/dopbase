---
title: "Admin UI: roles"
description: "Create custom roles that decide what people and AI agents may do and how deep their grants reach, as root in the Dopbase Admin UI."
---

# Roles

A role decides what its holders may do on the instance and how deep into
project data their grants can ever reach. Which projects they reach still
comes from grants. Root opens **Roles** from the sidebar; nobody else can
change roles.

## Built-in roles

Admin, Member, Auditor, and AI agent are built in. They can't be changed or
deleted. See [identity and access](/reference/identity#roles-and-capabilities)
for what each one holds.

## Create a role

Choose **New role** and set:

- **Name** and an optional description, shown in the role picker on Users.
- **Reaches at most**: the deepest level any grant can give holders. A Viewer
  might stop at Read.
- **Holders**: people who sign in, or accounts that only use tokens, like AI
  agents. Token-only accounts always have an owner. This can't change later.
- **Holders can**: the capabilities, such as seeing people, creating projects,
  or reading the audit log. Root's own powers, such as backups and single
  sign-on, can't be given to a role.
- **Their tokens read**: project data, up to a level, or the audit log only.
  This appears when the role holds personal tokens.
- How long their tokens last by default, and at most.

Some examples:

| Role       | Holders can                                                                    | Reaches at most | Tokens                      |
| ---------- | ------------------------------------------------------------------------------ | --------------- | --------------------------- |
| Viewer     | Hold personal tokens                                                           | Read            | Project data, read          |
| Supervisor | See people, read the audit log for their projects, see who can reach a project | Read            | None                        |
| CI bot     | Hold personal tokens (tokens only)                                             | Read            | Project data, read, 30 days |

## Change or delete a role

Choose **Edit** to change a custom role. Everyone holding it gets the new
settings on their next request. Lowering how far the role reaches lowers its
tokens' limit too.

Choose **Delete** and type the role's name. A role someone holds can't be
deleted; give them another role first.

## Who can give a role

On **Users**, people see only the roles within their own: every capability of
the role must be one they hold, and its level no deeper than theirs. Root can
give any role.

Role changes are recorded in the audit log as `role.created`, `role.updated`,
and `role.deleted`.
