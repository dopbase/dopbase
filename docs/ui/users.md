---
title: "Admin UI: users"
description: "Add people to Dopbase, give each one a role, see where their access to projects comes from, and disable or delete accounts in the Admin UI."
---

# Users

Open **Users** to see everyone who signs in to the instance. Anyone with
`users.view`, such as an admin or an auditor, can open it. Adding, changing,
disabling, and removing accounts needs `users.manage`, and happens in the
Admin UI only. There is no CLI command for it. Bots and CI jobs have their own
page, [AI agents](./ai-agents).

The table shows each person's role, their groups, how they sign in, and when
they last signed in. Filter by email, or add a **Role** or **Status** chip to
narrow the list to one role, or to active, disabled, or single sign-on
accounts. On a phone the table becomes a list of cards.

## Roles

Every account except root holds one role. The role decides what it may do on
the instance and how deep into project data it can ever reach. Which projects
it actually reaches comes from grants.

- **Root** is the account created during setup. It can do everything, and
  nobody else can change, disable, or delete it.
- **Admin** manages people, groups, and the projects they manage, and creates
  projects.
- **Member** works on the projects and environments they are given.
- **Auditor** reads the whole audit log and who has access, and never sees a
  secret value.

Root can add [custom roles](./roles), which show a `custom` tag next to the
role name. The role picker lists every role that signs in, but only roles
within your own can be picked: an admin sees Auditor greyed out with "Your
role can't give this one", because auditors read the whole audit log and
admins don't.

## Add a person

1. Choose **Add user**.
2. Enter their email and a temporary password. With single sign-on on, pick
   **Single sign-on** instead and they sign in through the provider.
3. Pick a role.
4. Choose **Add user**, then share the sign-in details securely.

You land on their **Access** tab, because a new person reaches no project
until someone gives them access. An admin that root adds starts as project
manager of every project; an admin that another admin adds starts with
nothing. Dopbase doesn't send invitation emails.

## A person's page

Choose someone's email to open their page. It has five tabs:

- **Overview**: email, role, the deepest level their role allows, how they
  sign in, and how many grants and groups they have.
- **Access**: every grant that reaches them, and where it comes from: direct,
  or **via** a group, which links to the group. A note under the table says
  when a grant on one environment overrides the grant on its project. Choose
  **Give access** to give them a project or one environment, or use a row's
  **⋯** menu to change the level or remove a direct grant. Grants that come
  through a group are changed on the group.
- **Groups**: the groups they belong to. **Add to a group** and **Take out**
  need `groups.manage`.
- **Tokens**: the personal tokens they made, with what each can do, where, and
  when it was last used. People with `tokens.revoke_others` can revoke them.
- **Danger zone**: unlink single sign-on, disable, or delete the account.

Choose **Edit** at the top to change their email, role, or password. Leave the
password empty to keep it. Changing someone signs them out everywhere, and
changing their role drops their access to every project at once; their grants
stay. Nobody changes root or their own account from here.

## Disable or turn back on

**Disable account**, from the Danger zone or a row's **⋯** menu, signs the
person out everywhere and stops their tokens. Nothing is deleted: their role,
grants, groups, and tokens stay, and AI agents they own stop working too. The
row shows a red `disabled` tag.

**Turn account back on** lets them sign in again with the access they had
before. Tokens that haven't expired work again.

## Delete someone

Choose **Delete user** and type their email to confirm. Their grants, group
memberships, and tokens go with the account; audit events stay. You can only
manage people whose role and access are within your own. Someone who owns AI
agents can't be deleted until the agents are given another owner on the
[AI agents](./ai-agents) page.

See [identity and access](/reference/identity) for every rule.
