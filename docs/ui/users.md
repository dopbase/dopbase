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

Adding people and changing their email or role needs `DOPBASE_RBAC=true` on
the server. Without it, the page still lists everyone, and admins can set
someone a new password and disable, enable, or delete accounts. There's no
**Add user**, **Edit** becomes **Set password**, and the Access and Groups tabs
are read-only. See
[optional features](/self-hosting/operations#optional-features).

When the instance has a [user limit](/self-hosting/operations#people-limit),
the page shows the count next to **Add user**, such as `3 of 5 active users`.
Root, AI agents and disabled accounts don't count. Once the limit is reached,
**Add user** turns off and the page says so. To add or enable a user, disable
or delete an active user, or ask your instance administrator to increase the
limit.

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
   **Single sign-on** instead and they sign in through the provider with an
   account that has the same verified email. Only the emails you add here can
   sign in; see
   [Who can sign in](/guide/single-sign-on#who-can-sign-in).
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
  or **via** a group, which links to the group. When a grant on one environment
  overrides the grant on its project, a box under the table lists that
  environment with the project's level next to the one it gets instead, or
  **No access** if the environment is blocked. Use a
  row's **⋯** menu to change the level or remove a direct grant. To remove
  several, tick their boxes, or the box in the header for every row the
  filter shows, then choose **Remove access** and confirm. If one fails, the
  ones before it stay removed and the dialog keeps the rest. Grants that come
  through a group are changed on the group and have no box. Rows with nothing
  to change, such as an all-projects grant, have no menu.
- **Groups**: the groups they belong to. **Add to a group** and **Take out**
  need `groups.manage`.
- **Tokens**: the personal tokens they made, with what each can do, where, and
  when it was last used. People with `tokens.revoke_others` can revoke them.
- **Danger zone**: unlink single sign-on, disable, or delete the account.

Root's page has no tabs, because root reaches everything without grants, can't
join a group, and has no personal tokens. The **Account** card shows root's
email, account ID with a copy button, how it signs in, whether single sign-on
is linked, and when it last signed in. Next to it, **Full access**
explains why grants, groups and tokens don't apply, with a link to
[AI agents](./ai-agents) for scripts and AI tools.

If you can read the whole audit log, **Recent activity** lists root's five
latest events: when, the action, what it touched (such as
`maya@example.com · Read`), and the address it came from.
**See all in Audit log** opens the log filtered to root. A link to one of
root's old tabs opens this page.

Choose **Edit** at the top to change their email, role, or password. Leave the
password empty to keep it. Changing someone signs them out everywhere, and
changing their role drops their access to every project at once; their grants
stay. Nobody changes root or their own account from here.

## Giving access

Choose **Give access** on the Access tab. It opens a page with three steps:

1. **Project**: pick one from the list of projects you manage. Each shows how
   many environments it has and the level they already hold on it, if any.
   Type to search once you manage more than six.
2. **Where**: the whole project, which covers environments added later, or
   some environments. Tick as many as you like; each one becomes its own
   grant. A filter box appears when the project has more than six.
3. **Level**: how far they reach. Project manager only applies to a whole
   project, and levels above their role are shown but can't be picked.

**Review**, under the steps, lists every grant before you save. A grant that
replaces one shows both levels, for example Names only → Read. A
whole-project grant below Project manager doesn't reach protected
environments, so Review names those and you can give them separately. AI
agents and groups use the same page.

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
