---
title: "Admin UI: users and AI agents"
description: "Add people and AI agents to Dopbase, give each one a role, and see where their access to projects comes from in the Admin UI."
---

# Users and AI agents

Open **Users** to see everyone on the instance. Anyone with `users.view`, such
as an admin or an auditor, can open it. Adding, changing, and removing accounts
needs `users.manage`, and happens in the Admin UI only. There is no CLI command
for it.

## Roles

Every account except root holds one role. The role decides what it may do on
the instance and how deep into project data it can ever reach. Which projects
it actually reaches comes from grants, given on each project's
[Access](./project-access) page.

- **Root** is the account created during setup. It can do everything, and
  nobody else can change or delete it.
- **Admin** manages people, groups, and the projects they manage, and creates
  projects.
- **Member** works on the projects and environments they are given.
- **Auditor** reads the whole audit log and who has access, and never sees a
  secret value.
- **AI agent** uses tokens only and never reaches more than its owner.

Root can add [custom roles](./roles). The role picker lists only roles within
your own: an admin can't make someone an auditor, because auditors read the
whole audit log and admins don't.

## Add a person

1. Choose **Add User**.
2. Enter their email and a password. With single sign-on on, you can leave the
   password empty and they sign in through the provider.
3. Pick a role.
4. Save and share the sign-in details securely.

A new member reaches no project until someone gives them access. An admin that
root adds starts as project manager of every project; an admin that another
admin adds starts with nothing. Dopbase doesn't send invitation emails.

## Edit or remove someone

Choose **Edit** to change an email, role, or password. Leave the password empty
to keep it. Changing someone signs them out everywhere, and changing their role
drops their access to every project at once; their per-project grants stay.

Choose **Delete** and type the email to confirm. You can't delete or change
your own role, and you can only manage people whose role and access are within
your own. Someone who owns AI agents can't be deleted until the agents are
deleted or given another owner.

## See where access comes from

Choose **Access** beside someone to see their role's ceiling, their groups, and
every grant that reaches them, their own and their groups'. Grants are changed
on each project's [Access](./project-access) page.

## AI agents

Choose **Add AI agent** and give it a name. AI agents have no email and no
password, and you become the agent's owner. It reaches only what it is granted,
never past Read, and never more than you do. If you are disabled, so is the
agent.

Choose **Get token** beside an agent to create its personal token. Pick whether
the token reads names only or values too, and when it expires, at most 90 days
from now. If you have a password, you confirm it. A new token revokes the
agent's previous one. Dopbase shows the token once, so copy it straight into a
secret store.

Give the agent access on a project's Access page before handing it the token.
The [AI agent access guide](/guide/ai-agent-access) shows how to pass the token
to an agent and what it can do with it.

Choose **Delete** and type the agent's name to remove it with its tokens.

See [identity and access](/reference/identity) for every rule.
