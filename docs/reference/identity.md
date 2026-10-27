---
title: "Identity, roles, and tokens"
description: "How Dopbase handles identity: root, admin, and member roles, AI agent accounts, human sessions, and environment-scoped runner tokens."
---

# Identity and tokens

Dopbase needs identities for people and for software.

## Human users

The instance has one protected human `root` account, created during first-run
setup, plus `admin` and `member` accounts. All three human roles have full
project, environment, secret, and runner-token access in the Admin UI and
existing human CLI operations. Root and admin manage human and AI accounts.
Root alone manages full-instance backups and factory reset. Other users cannot change or delete the root account.

Passwords are hashed with Argon2id. Browser sessions have an eight-hour idle
and 24-hour absolute lifetime. CLI sessions are opaque bearer tokens with a
30-day idle and 90-day absolute lifetime and are stored in an encrypted local
session file. Offline password recovery verifies the master key, requires
the server to be stopped, and revokes every human session.

### Single sign-on

Root can let people sign in to the Admin UI through an OpenID Connect provider.
A sign-in links to an existing admin or member account by verified email the
first time, then by the provider's subject identifier. It never creates an
account. While it is on, administrators can create accounts without a password,
and root can require single sign-on for everyone else. Root always keeps its
password. See [single sign-on](/guide/single-sign-on).

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
the Admin UI instead. Runner tokens can export their
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

### Workspace runner tokens

Monorepos and shared CI runners can use a workspace-scoped token instead of
one token per environment. The token prefix is `dbw_` (token ids use `wtk_`).
It can call runtime secret export for any environment whose project is linked
to that workspace on the server. Projects without a `project_locations` row
are never included, even on the same instance.

```bash
dopbase token create --workspace default --name ci-runner --role workspace
dopbase token list --workspace default
```

Workspace tokens share the same expiry, revocation, and `dopbase login --token`
behavior as environment runner tokens. They still cannot list metadata, mutate
secrets, reveal, export through human-only paths, or access audit history.

## AI agents

The `ai_agent` role belongs to named service accounts with bearer tokens,
not email/password sessions. Agents can read project and environment metadata,
secret names, versions, timestamps, editor layouts, and instance counts. They
cannot reveal, export, or retrieve runtime secret values, mutate projects,
manage accounts, list runner tokens, or read audit history.

Agent tokens are displayed only once when created. They expire after 30 days
by default. The Admin UI also offers no expiry and custom durations of up to
three years. Revoking a token or deleting its account prevents subsequent access.
Pass an agent token to the CLI through `DOPBASE_TOKEN`, just like a runner
token. `dopbase client status` identifies it as `ai_agent` without displaying
the token. Use this role when an agent needs configuration context for tasks
such as checking variable names or preparing an `.env.example` file. It is not
a runtime credential. See [AI agent access](/guide/ai-agent-access) for a
practical workflow and [users and AI agents](/ui/users) for account management.

## Permission model

The four roles are static. All projects are shared across the instance. No
user-to-project assignments, custom roles, or configurable permissions exist.
Runner tokens remain separate environment- and workspace-scoped credential types.

| Operation                                                                  | Root | Admin | Member | AI agent |
| -------------------------------------------------------------------------- | ---- | ----- | ------ | -------- |
| Read project/environment and secret metadata                               | Yes  | Yes   | Yes    | Yes      |
| Manage projects, environments, secrets, imports/exports, and runner tokens | Yes  | Yes   | Yes    | No       |
| Manage non-root human users and AI accounts                                | Yes  | Yes   | No     | No       |
| Read audit events, including filtered history                              | Yes  | Yes   | No     | No       |
| View instance status                                                       | Yes  | Yes   | Yes    | Yes      |
| Full-instance backups and master-key download                              | Yes  | No    | No     | No       |
| Factory reset in the Admin UI or offline host CLI                          | Yes  | No    | No     | No       |

No human can delete their own account or change their own role. An admin can
manage another admin, but cannot modify root. Root is created only during first-run setup. Root changes its password through Account settings.
After an account is updated, that user needs to sign in again.

These roles are available in version {{version}}. See [product status](/guide/product-status)
for the current operational boundaries.

Full-instance backup listing, creation, upload, download, restore, and deletion
are root-only because snapshots contain every project and human account.

## Factory reset

The root-only factory reset deletes every account (including root), all
projects, environments, secrets, runner tokens, AI accounts and their tokens,
audit history, and server-held backups. It preserves deployment configuration
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
