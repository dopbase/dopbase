---
title: "Identity and tokens"
description: "Root, admin, member, and AI-agent access, human sessions, and environment-scoped runner tokens."
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

## Machine identities

CI jobs, servers, containers, deployment systems, and automation use
environment-scoped runner tokens.

```bash
export DOPBASE_TOKEN=dbs_xxxxxxxxxxxxxxxxx
dopbase run env_01ABCDEF -- npm start
```

`DOPBASE_TOKEN` is preferred over a saved human login when it is present.

Interactive `dopbase login` encrypts its token in the extensionless `session`
file under the Dopbase data directory. The separate `session-key` file holds
the random local encryption key. The global TOML config contains the selected
server but never the token. A saved credential is used only for its
matching server.

The encrypted payload also caches the administrator email for offline
`dopbase client status` output. The password is never stored.

Plaintext `secret get --reveal` and `export` operations in the official CLI
require interactive password confirmation every time. This is a CLI safety
gate; direct HTTP clients continue to follow the server's existing recent-
authentication policy.

For application servers, create a runner token scoped to one environment:

```bash
dopbase token create payment-service/production \
  --name production-server --role runner
```

The plaintext token is displayed only once. A runner can retrieve and inject
values from its assigned environment, but cannot change secrets, export them,
or access another environment. Production and staging servers should always
use different runner tokens.

## AI agents

The `ai_agent` role belongs to named service accounts with bearer tokens,
not email/password sessions. Agents can read project and environment metadata,
secret names, versions, timestamps, editor layouts, and instance counts. They
cannot reveal, export, or retrieve runtime secret values, mutate projects,
manage accounts, list runner tokens, or read audit history.

Agent tokens are displayed only once when created. They expire
after 30 days by default, a supplied expiry must be in the future and within
90 days. Revoking a token or deleting its account prevents subsequent access.
See [users and AI agents](/ui/users) for account management.

## Permission model

The four roles are static. All projects are shared across the instance; no
user-to-project assignments, custom roles, or configurable permissions exist.
Runner tokens remain a separate environment-scoped credential type.

| Operation                                                                  | Root | Admin | Member | AI agent |
| -------------------------------------------------------------------------- | ---- | ----- | ------ | -------- |
| Read project/environment and secret metadata                               | Yes  | Yes   | Yes    | Yes      |
| Manage projects, environments, secrets, imports/exports, and runner tokens | Yes  | Yes   | Yes    | No       |
| Manage non-root human users and AI accounts                                | Yes  | Yes   | No     | No       |
| Read audit events, including filtered history                              | Yes  | Yes   | No     | No       |
| View instance status                                                       | Yes  | Yes   | Yes    | Yes      |
| Full-instance backups and master-key download                              | Yes  | No    | No     | No       |
| Factory reset in the Admin UI                                              | Yes  | No    | No     | No       |

No human can delete their own account or change their own role. An admin can
manage another admin, but cannot modify root. Root is created only during first-run setup. Root changes its password through Account settings.
After an account is updated, that user needs to sign in again.

These roles are available in version 0.1.0. See [product status](/guide/product-status)
for the current operational boundaries.

Full-instance backup listing, creation, upload, download, restore, and deletion
are root-only because snapshots contain every project and human account.

## Factory reset

The root-only Danger Zone factory reset deletes every account (including root),
all projects, environments, secrets, runner tokens, AI accounts and their tokens, audit history, and
server-held backups. It preserves deployment configuration and the master key,
then reopens first-run setup. The Admin UI requires the root password, an
explicit acknowledgment, and typing `FACTORY RESET` all sessions are signed
out. Backups downloaded elsewhere are not affected. See
[factory reset](/self-hosting/factory-reset) for the confirmation steps and deletion scope.

## Token handling

Tokens must be scoped, revocable, and hidden from logs. Operators should use the narrowest permissions available and rotate a token immediately if it may have been exposed.

`dopbase run` removes Dopbase authentication variables before starting the
child process. Dopbase does not accept tokens as command-line arguments because
they may be exposed through process inspection or shell history.

The session and key files are restricted to the current user where the platform
supports it. An attacker that can read both files can decrypt the token; use
`DOPBASE_TOKEN` when an externally managed credential is required.
