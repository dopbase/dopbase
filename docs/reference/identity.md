---
title: "Identity and access"
description: "Root, people, and AI agents; roles and capabilities; grants and groups on projects and environments; sessions, personal tokens, and runner tokens."
---

# Identity and access

Dopbase has three kinds of identity:

- **Root**, the one account created during first-run setup. It can do
  everything and is the way back in when everything else fails.
- **Accounts**, for people and AI agents. Each holds one role, which says what
  it may do on the instance, and reaches projects and environments through
  grants.
- **Runner tokens**, for servers and CI jobs. Each reads the values of one
  environment.

People sign in with a password or single sign-on. AI agents never sign in;
they use personal tokens. A personal token always acts as one account and
never reaches further than that account does.

## People

Passwords are hashed with Argon2id. Browser sessions have an eight-hour idle
and 24-hour absolute lifetime. CLI sessions are opaque bearer tokens with a
30-day idle and 90-day absolute lifetime and are stored in an encrypted local
session file. Offline password recovery verifies the master key, requires
the server to be stopped, and revokes every human session. A disabled account
can't sign in, and its sessions and tokens stop working.

### Single sign-on

Root can let people sign in to the Admin UI through an OpenID Connect provider.
A sign-in links to an existing account by verified email the first time, then
by the provider's subject identifier. It never creates an account, and it never
links root or an AI agent. While it is on, administrators can create accounts
without a password, and root can require single sign-on for everyone else. Root
always keeps its password, because it alone holds `password.breakglass`. See
[single sign-on](/guide/single-sign-on).

### Browser login

`dopbase login --browser` signs the CLI in through the Admin UI, so you never
type your password in the terminal. The CLI prints a link and a short code such
as `WDJB-MJHT`, then waits. Copy the link into a browser on any device, sign
in, check that the page shows the same code and the machine you expect, and
approve it. The CLI saves an ordinary CLI session in the encrypted session
file. The page then closes its tab after 5 seconds, unless you choose to keep
it open. Some browsers only let a page close tabs a script opened, so the page
may ask you to close it yourself. `--browser-open` does the same and also opens the link in this machine's
browser.

If this browser is already signed in, the link opens straight on the request
and shows which account it will sign in, with a button to switch accounts.
Otherwise the page asks you to sign in first and then brings you back to the
code. Approving needs a recent confirmation, like other sensitive actions: if
you last entered your password or used single sign-on more than 10 minutes
ago, the page asks you to confirm before it approves. Nothing signs you out.
Because a signed-in browser opens the request right away, check the code and
the device before approving, and deny any link someone else sent you.

Approving gives that terminal a session for up to 90 days. A code expires after
10 minutes and works once, and one client can have at most 20 requests waiting.
When single sign-on is on, plain `dopbase login` uses this flow too;
`--password` forces the prompt. The CLI token goes straight from the server to
the CLI and never reaches the browser. If you did not start a request, deny it;
the waiting CLI stops with an error.

## Machine identities

CI jobs, servers, containers, deployment systems, and automation use
environment-scoped runner tokens.

```bash
dopbase login --token
dopbase run env_482731 -- npm start
```

The prompt does not echo the token. A provisioning script can pipe it instead:

```bash
printf '%s' "$RUNNER_TOKEN" | dopbase login --token
```

CI and deployment platforms should inject `DOPBASE_TOKEN` from their secret
store. Authentication uses `DOPBASE_TOKEN`, then the encrypted saved credential.
An invalid environment credential stops the command; Dopbase does not fall back
to a saved credential.

`dopbase login` and `dopbase login --token` encrypt their credential in the
extensionless `session` file under the Dopbase data directory. The separate
`session-key` file holds the random local encryption key. The global TOML
config contains the selected server but never the token. A saved credential is
used only for its matching server. Saving another credential replaces it.

The encrypted payload also caches the administrator email for offline
`dopbase client status` output. The password is never stored.

Human `secret get --reveal` and `export` operations in the official CLI require
interactive password confirmation every time. An account that cannot use a
password, because it has none or single sign-on is required, approves a code in
the Admin UI instead. Personal tokens skip the prompt; the server checks that
the token reaches values. Runner tokens can export their
assigned environment without a prompt, using `DOPBASE_TOKEN` or a saved runner
credential. Direct HTTP clients follow the server's authentication policy;
browser exports require recent authentication, with a password or single
sign-on, and a valid CSRF header.

For application servers, create a runner token scoped to one environment:

```bash
dopbase token create payment-service/production \
  --name production-server --role runner
```

The plaintext token is displayed only once. A runner can retrieve, inject, and
export values from its assigned environment. It cannot change secrets or access
another environment. Exports are audited with the runner token ID and secret
count, without recording values. Production and staging servers should always
use different runner tokens. Runner tokens have no expiry by default. Use
`--expires-in 12h` or `--expires-in 7d` when creating one for a short job.
Durations must be whole hours or days, up to 1,095 days.

## Roles and capabilities

Every account except root holds one role. A role is data, not code. It holds:

- **Capabilities**: what holders may do on the instance, such as manage people
  or read the audit log.
- **A level ceiling**: the deepest level any grant can give holders. A grant
  above it is refused, and access never goes past it.
- **A sign-in mode**: people who sign in, or accounts that use tokens only.
- **A token policy**: whether holders' personal tokens read project data or the
  audit log, the highest level they reach, and how long they last.

These roles are built in and can't be changed:

| Role     | Capabilities                                                                                                                                                                                 | Ceiling         | Personal tokens                                               |
| -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------- | ------------------------------------------------------------- |
| Admin    | `users.view`, `users.manage`, `groups.manage`, `grants.manage`, `projects.create`, `audit.read`, `audit.verify`, `access.report`, `instance.view`, `tokens.personal`, `tokens.revoke_others` | project manager | Project data, up to write, 30 days by default, at most a year |
| Member   | `tokens.personal`                                                                                                                                                                            | manage          | Project data, up to write, 30 days by default, at most a year |
| Auditor  | `users.view`, `audit.read`, `audit.read_all`, `audit.verify`, `access.report`, `instance.view`, `tokens.personal`                                                                            | metadata        | The audit log only, 30 days by default, at most a year        |
| AI agent | `tokens.personal`                                                                                                                                                                            | read            | Project data, up to read, 30 days by default, at most 90 days |

Root creates custom roles on the [Roles](/ui/roles) page, for example a Viewer
that reads values but never changes them, or a token-only role for CI bots.
Changing a custom role changes what everyone holding it can do on their next
request. A role someone holds can't be deleted.

| Capability             | Lets holders                                                                           |
| ---------------------- | -------------------------------------------------------------------------------------- |
| `users.view`           | See people, groups, roles, and where someone's access comes from                       |
| `users.manage`         | Add, change, and remove accounts, AI agents included, whose access is within their own |
| `groups.manage`        | Create groups and change their members                                                 |
| `grants.manage`        | Reserved. Giving access depends on the project manager level today                     |
| `projects.create`      | Create projects. The creator becomes the new project's project manager                 |
| `access.report`        | See who reaches any project they can see, without managing it                          |
| `audit.read`           | Read audit events on settings and on the projects they reach                           |
| `audit.read_all`       | Read every audit event                                                                 |
| `audit.verify`         | Verify the audit log's hash chain                                                      |
| `instance.view`        | See instance status                                                                    |
| `tokens.personal`      | Hold personal tokens                                                                   |
| `tokens.revoke_others` | Revoke other people's personal tokens when their access is within their own            |

Root also holds `roles.manage`, `grants.all_projects`, `sso.manage`,
`instance.backup`, `instance.reset`, and `password.breakglass`. No role can be
given these, so root alone manages roles, single sign-on, backups, and factory
reset, grants access to every project at once, and keeps password sign-in when
single sign-on is required.

## Access to projects and environments

A **grant** gives a user or a group one level on all projects, one project, or
one environment. The levels build on each other:

| Level           | Allows                                                                                                  |
| --------------- | ------------------------------------------------------------------------------------------------------- |
| Names only      | See the project and environment, secret names, versions, history, and the `.env` layout                 |
| Read            | Also reveal values, export, run with secrets, and read old versions                                     |
| Write           | Also set, delete, import, and roll back secrets                                                         |
| Manage          | Also rename, protect, or delete the environment and manage its runner tokens                            |
| Project manager | Given on a project only. Also rename or delete the project, add environments, and decide who reaches it |

To work out what an account reaches in one environment, Dopbase:

1. Collects the account's own grants and those of its groups.
2. Keeps the most specific kind that applies: a grant on the environment wins
   over one on its project, which wins over one on all projects.
3. Takes the highest level among those.
4. Caps it at the role's ceiling. For an AI agent, also at its owner's access.
   For a personal token, also at the token's own scope and level.

A project manager manages every environment of the project. A grant of **No
access** on one environment cuts it out of a wider grant, and a lower
environment grant narrows a wider one.

An environment marked **protected**, such as production, ignores project and
all-projects grants below project manager. Only a grant on the environment
itself, or a project manager, reaches it.

Anything an account can't reach doesn't exist for it: lists leave it out and
requests for it answer 404. Changes apply on the account's next request, tokens
included.

### Groups

A group is a named set of accounts. A grant to a group reaches everyone in it,
and leaving the group takes that access away. Groups are flat, never hold root,
and never add capabilities. See [groups](/ui/groups).

### Who can change access

Grants, group members, and protection are changed from the Admin UI only,
with a recent sign-in. These rules stop anyone from handing out more than they
hold:

- You grant only on projects you manage, at levels up to project manager.
  Grants on all projects are root's.
- Nobody grants to themselves or to root, and a grant to a person stays within
  their role's ceiling.
- You change a group only when you manage every project it reaches.
- You give only roles within your own capabilities and ceiling.
- You change, delete, or unlink an account only when its role and its access
  are within yours. Only root changes root.
- Nobody changes their own role.

Admins that root creates start as project managers of every project. Admins
that other admins create, and every new member, start with no access until
someone shares a project with them. Changing a person's role drops their
all-projects grant; project grants stay, under the new role's ceiling.

## Personal tokens

A personal token (`dpp_...`) lets a script or an AI agent act as an account
without signing in. It reaches at most what the account reaches at that
moment, within its own scope and level:

- **Scope**: all projects, one project, or one environment. Audit tokens read
  the audit log instead.
- **Level**: names only, read, or write, up to the role's token limit.
- **Expiry**: always, within the role's limit.

A token never holds the account's instance capabilities. It reads and changes
secrets within its level, but can't manage people, grants, projects, or other
tokens. Losing a grant, leaving a group, or a disabled account takes effect on
the token's next request.

Create your own on the [Account](/ui/account) page or with
`dopbase token personal create`, then pass it as `DOPBASE_TOKEN`.
`dopbase access` shows what it reaches. Root has no personal tokens.

You can revoke your own tokens and those of AI agents you own. People with
`tokens.revoke_others` revoke anyone's whose access is within theirs.

## AI agents

An AI agent is an account with the AI agent role, or another role that uses
tokens only. It has a name instead of an email, no password, and an
**owner**: the person who created it. It reaches only the projects granted to
it, never more than its role's read ceiling, and never more than its owner
reaches. A disabled owner disables the agent.

Owners, and people who manage accounts, create the agent's tokens on the
[Users](/ui/users) page. A person who owns agents can't be deleted until the
agents are deleted or given another owner. See
[AI agent access](/guide/ai-agent-access) for both ways to give an agent
access: your own personal token, or a shared agent account.

Service accounts from earlier versions became AI agent accounts owned by root.
They still read secret names on every project, and their `dpa_` tokens keep
working.

## Audit log

Every reader sees what their role allows:

- `audit.read_all`, held by Auditor and root: every event.
- `audit.read`, held by Admin: events on settings and on the projects they
  reach, and their own.
- Everyone else: their own actions.

An auditor's personal tokens read the audit log as the auditor would and never
reach project data, which suits a SIEM. See
[audit events](/reference/audit-events).

## What each role can do

Levels apply per project and environment, so this table shows the most a
built-in role can reach. Root can do everything.

| Operation                                                | Admin              | Member            | Auditor      | AI agent                 |
| -------------------------------------------------------- | ------------------ | ----------------- | ------------ | ------------------------ |
| See projects, environments, and secret names             | With a grant       | With a grant      | With a grant | With a grant             |
| Reveal, export, and run with values                      | With read          | With read         | No           | With read                |
| Change, import, and roll back secrets                    | With write         | With write        | No           | No                       |
| Rename, protect, or delete environments; runner tokens   | With manage        | With manage       | No           | No                       |
| Rename or delete projects, add environments, give access | As project manager | No                | No           | No                       |
| Create projects                                          | Yes                | No                | No           | No                       |
| Manage accounts, AI agents, and groups                   | Within their own   | No                | No           | No                       |
| Read the audit log                                       | Their projects     | Their own actions | All of it    | No                       |
| Hold personal tokens                                     | Project data       | Project data      | Audit only   | Read only, via the owner |
| Roles, single sign-on, backups, factory reset            | No                 | No                | No           | No                       |

Full-instance backup listing, creation, upload, download, restore, and deletion
are root-only because snapshots contain every project and account.

## Factory reset

The root-only factory reset deletes every account (including root and AI
agents), all projects, environments, secrets, runner tokens, personal tokens,
grants, groups, custom roles, audit history, and server-held backups. It preserves deployment configuration
and the master key, then reopens first-run setup. The Admin UI requires the
root password, an explicit acknowledgment, and `FACTORY RESET`. The offline
`dopbase admin factory-reset` command requires the server to be stopped, the
root password, and `please-wipe-out-system`. It saves the whole data directory
as a timestamped ZIP before removing it. Pass `--no-backup` to remove the data
without creating the ZIP. The next `server start` enters automatic setup on
fresh storage. For backup restore, omit `DOPBASE_ROOT_EMAIL` during startup or
use `server setup --web`. Pending resets still require explicit setup.
See
[factory reset](/self-hosting/factory-reset) for the confirmation steps and deletion scope.

## Token handling

Tokens must be scoped, revocable, and hidden from logs. Operators should use the narrowest permissions available and rotate a token immediately if it may have been exposed.

`dopbase run` removes `DOPBASE_TOKEN` before starting the child process. Use a
saved runner credential on a persistent server, or let your deployment platform
inject `DOPBASE_TOKEN`. Environment variables still need protection from logs
and processes that can inspect the runner.

The session and key files are restricted to the current user where the platform
supports it. An attacker that can read both files can decrypt the token. Use
`DOPBASE_TOKEN` when an externally managed credential is required.
