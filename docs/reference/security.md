---
title: "Security model"
description: "The Dopbase security model: encryption of secret values, separation of master key material, audit records, and access control."
---

# Security model

Dopbase stores credentials that can grant access to databases, cloud accounts, payment providers, and internal services. Its security controls apply when those credentials are stored, served, revealed, and audited.

::: info Review the security model
This page documents the {{version}} security model. Public source makes independent
review possible, but it is not the same as an independent security audit.
:::

## Security goals

See [identity and tokens](/reference/identity#permission-model) for the
permissions available to each role.

- Encrypt secret values before persistence.
- Keep the master encryption key outside the secrets database.
- Use authenticated encryption so modified ciphertext is detected.
- Keep cryptographic code isolated and auditable.
- Reveal plaintext only for authorized operations.
- Never include plaintext secrets in application or audit logs.
- Record sensitive actions without recording their values.

## Envelope encryption

The v{{version}} implementation uses envelope encryption:

```text
Secret value
    ↓ encrypted with a data encryption key
Ciphertext

Data encryption key
    ↓ encrypted with a master key
Encrypted data key
```

Each stored value receives a random 256-bit data key. XChaCha20-Poly1305
encrypts the value and separately wraps its data key with the master key. The
environment ID, secret key, version, and algorithm version are authenticated as
additional data. Unique random nonces are stored with the ciphertext.

The database contains ciphertext, wrapped keys, nonces, versions, and safe
metadata. The 256-bit master key remains in a separate owner-only file and is
verified before the HTTP listener starts.

## Data in transit

Networked clients need an authenticated, encrypted connection to the server. Local development may use localhost, but a production deployment must not send credentials over an unprotected network.

## OIDC assertions

Dopbase can trust external OIDC identity providers. It verifies an assertion
before using any claim: the `iss` must match a configured provider, the
signature must verify against that provider's published JWKS, the algorithm
must be one the provider allows, the `aud` must contain a configured audience,
and `exp`, and `nbf` when present, must be current. Verification fails closed.
It rejects an untrusted issuer, an unknown key, a bad signature, and every
symmetric algorithm, and it reports a rejection with a stable code without
echoing the assertion.

## Single sign-on

An operator can enable browser single sign-on for the admin UI. It uses the
OpenID Connect authorization code flow with PKCE. The sign-in binds a provider
identity to an existing account; it never creates one. On the first sign-in the
provider's verified email is matched against an account, and the pair of issuer
and subject is stored so later sign-ins do not depend on the email. An email
that matches no account, a group that maps to no role, and an unverified email
are all refused. The role comes from the configured group mapping, so a
provider group can grant `admin` or `member` but never `root`. The single-use
`state`, the PKCE verifier, and the nonce are checked on every callback, the
pending sign-in is bound to the browser that started it with a short-lived
cookie so a callback URL cannot be replayed into another browser, and the
provider's tokens are used only to read claims and are never persisted.

## Data exposure

Authorized users may need to reveal or export a value. These operations should be explicit, permission-controlled, and audited. Secret names can also disclose information, so access to metadata still needs authorization.

## Audit integrity

Audit records form a hash chain: each record stores its position, the hash of the record before it, and the hash of its own contents. Editing a record, removing one, or inserting one out of order breaks the chain at that point, which `dopbase audit verify` and `GET /api/v1/audit-events/verify` report.

The chain is tamper evidence, not access control. Someone who can write the database file can rewrite every hash or delete the newest records, and the chain alone still verifies. That is detected only against a copy of the head kept outside the server: verify with `--anchor SEQ:HASH`, ship the `audit checkpoint` lines the server logs, or stream `dopbase audit export` to write-once storage. Records written before the chain existed are reported as legacy and are not protected. A restore archives the current log before replacing it. See [audit events](/reference/audit-events#integrity-and-verification) for the full threat model and what to do when verification fails.

## Terminal editor sessions

`dopbase env edit` checks environment and secret metadata before password
confirmation. Missing targets and declined empty environments do not export
values.

Existing sessions require confirmation on every invocation, a login
completed during the command counts as confirmation. Plaintext export follows
that confirmation.

Its protected editor profiles disable user and project configuration, plugins,
language servers, persistent history, backups, and swap files where applicable.
Custom editor commands require acknowledgement of their exposure before export.
A profile reduces accidental retention; it does not sandbox the executable.

Session directories and files are owner-only. Dopbase rejects unsafe edited
files, excludes credentials from the editor environment, disables core dumps
for the editing command, and removes the session on completion or handled
interruption.

Abandoned sessions are removed on a later edit invocation without
touching active sessions. Forced termination and power loss can leave files
behind.

Deleting a file does not guarantee erasure from storage or snapshots.
Processes running as the same user, privileged processes, compromised editors,
and terminal recording remain outside this protection.

Previews list key names and counts. Revision checks cover both the editing
period and the time between preview and apply, and saving uses one atomic
import. Editor files are removed before submitting a confirmed draft. A dry run validates syntax and Dopbase's limits; it cannot establish
whether credentials work with another service.

Editor comments are unencrypted layout metadata, visible wherever that
metadata is available. Values are stripped before layout persistence, and the
server rejects layout slots containing values. Comments must never contain
credentials.

## Offline runtime cache

Successful `dopbase run` operations cache their runtime payload locally using
XChaCha20-Poly1305 with a fresh nonce. A separate local cache key and the exact
credential that fetched the payload are both required to derive the decryption
key.

Cache files, metadata, key names, and values are encrypted. Logs expose
only safe runtime metadata, cache source, fetch time, age, and key count.

After unlocking the local document, `dopbase cache list` and `dopbase cache
clean --dry-run` parse only the server, environment metadata, fetch time, age,
and aliases.

Their human and JSON output omits secret names, values, cache keys,
nonces, and ciphertext. Cleanup holds the cache lock and replaces a partially
retained document atomically.

An unavailable server cannot confirm whether a credential or secret was later
revoked. Offline fallback therefore deliberately favors availability and may
inject stale values.

It is limited to connection failures, timeouts, and server
5xx responses. An explicit authentication, authorization, or not-found response
always fails closed.

## Reporting vulnerabilities

Dopbase accepts private vulnerability reports through [GitHub's private vulnerability reporting form](https://github.com/dopbase/dopbase/security/advisories/new).

Do not publish an undisclosed vulnerability in an issue, discussion, or pull request. Reports must not contain live credentials, customer data, private service URLs, or secrets from a system the reporter does not own. Read the repository's [security policy](https://github.com/dopbase/dopbase/security/policy) for the complete reporting and disclosure process.
