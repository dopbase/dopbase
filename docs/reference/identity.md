---
title: "Identity and tokens"
description: "How Dopbase models identities and access: the human administrator, hashed passwords, sessions, and tokens for software clients."
---

# Identity and tokens

Dopbase needs identities for people and for software.

## Human users

v0.0.12 supports exactly one human administrator with full access to the Admin UI
and human CLI operations.

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
`dopbase status` output. The password is never stored.

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

## Permission model

The single administrator has full v0.0.12 access. Runner tokens can resolve and
retrieve runtime values only for their assigned environment. Organizations,
additional humans, invitations, and advanced policy rules remain later work.

## Token handling

Tokens must be scoped, revocable, and hidden from logs. Operators should use the narrowest permissions available and rotate a token immediately if it may have been exposed.

`dopbase run` removes Dopbase authentication variables before starting the
child process. Dopbase does not accept tokens as command-line arguments because
they may be exposed through process inspection or shell history.

The session and key files are restricted to the current user where the platform
supports it. An attacker that can read both files can decrypt the token; use
`DOPBASE_TOKEN` when an externally managed credential is required.
