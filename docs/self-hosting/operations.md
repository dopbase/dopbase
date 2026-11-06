---
title: "Production operations: TLS, monitoring, upgrades"
description: "Operate a Dopbase server safely: network and TLS setup, monitoring, upgrades, backups, and recovery for a secrets manager."
---

# Operations

Operating a secrets manager requires protecting both service availability and stored credentials.

## Network and TLS

Localhost is the expected development default. Any server reachable over a network needs authenticated encryption in transit, a deliberate bind address, and firewall rules that expose only the required interface.

Terminate TLS with a reviewed reverse proxy and restrict the Dopbase listener to the network interfaces that need it.

Behind a proxy, every connection Dopbase sees comes from the proxy, so the audit log would record the proxy's address for every action. List the proxy in `trusted_proxies` in `server.toml`, or in `DOPBASE_TRUSTED_PROXIES`, and Dopbase reads the client address from `X-Forwarded-For` for connections from that proxy only:

```toml
trusted_proxies = ["127.0.0.1", "172.16.0.0/12"]
```

Dopbase reads the header from the right and stops at the first address that is not a trusted proxy, so a client cannot choose the address it is recorded under. Configure the proxy to set or append `X-Forwarded-For`, and list only proxies you run: any address on the list can claim to be any client.

## Secret history

Dopbase keeps the value of the newest 20 versions of every secret so they can
be [rolled back](/guide/projects-environments-secrets#versions-and-rollback).
Set `secret_history_versions` in `server.toml`, or
`DOPBASE_SECRET_HISTORY_VERSIONS`, to keep between 2 and 1000:

```toml
secret_history_versions = 50
```

The setting applies to a secret the next time it is saved. Retained versions
are part of the database, so backups and restores carry them, and they grow
the database by roughly the size of each stored value. Expired versions keep
only metadata.

## Optional features

Three features are off until you turn them on in `server.toml` or with an
environment variable. Restart the server after changing them.

| Setting      | Environment variable | What it turns on                                                        |
| ------------ | -------------------- | ----------------------------------------------------------------------- |
| `sso`        | `DOPBASE_SSO`        | [Single sign-on](/guide/single-sign-on) with an OpenID Connect provider |
| `rbac`       | `DOPBASE_RBAC`       | Adding and editing people, groups, and the access they're given         |
| `rbac_roles` | `DOPBASE_RBAC_ROLES` | Custom roles. Needs `rbac` as well                                      |

```toml
sso = true
rbac = true
rbac_roles = true
```

Turning a feature off deletes nothing, and turning it back on brings
everything back as it was. While a feature is off, its pages leave the
console and its API routes answer `403` `FEATURE_DISABLED` for every account,
root included.

With `sso` off, the login page shows only the password form, and a provider
set to "required" no longer blocks passwords. Accounts that have no password
can't sign in until it's back on. Root can always use its password.

With `rbac` off, everyone keeps their role and their access, but nobody adds
people, changes their email or role, changes groups, or gives people and
groups access. AI agents work as usual. Admins can still set someone a new
password, and disable, enable, and delete accounts.

With `rbac_roles` off, nobody creates, edits, or deletes custom roles, or gives
one to anyone. People who already hold a custom role keep it. Setting
`rbac_roles` without `rbac` leaves custom roles off, and the server prints a
warning when it starts.

### People limit

`rbac_max_users`, or `DOPBASE_RBAC_MAX_USERS`, caps how many active people the
instance holds while `rbac` is on. Root and AI agents don't count, and neither
do disabled accounts. Leave it out or set it to `0` for no limit, the same way
`audit_retention_days = 0` keeps the audit log forever. Any other value has to
be a whole number, or the server won't start.

```toml
rbac = true
rbac_max_users = 5
```

Once the limit is full, adding someone or turning a disabled account back on
answers `409` `USER_LIMIT_REACHED`. The Users page shows how many spots are
taken and turns off **Add user**. Disabling or deleting someone frees a spot.
If you lower the limit below the number of active people, everyone keeps
their access, and nobody new can be added until enough people leave. The
Status page shows how full the limit is. Setting it without `rbac` does
nothing, and the server prints a warning when it starts.

## Monitoring

Monitor service availability, database health, disk space, backup completion, authentication failures, and abnormal audit activity. Monitoring output must not contain request bodies or secret values.

## Logging

Application logs may include timestamps, operation names, request identifiers, status codes, and safe resource identifiers. They must never include plaintext secrets, credentials, decrypted request bodies, or authentication tokens.

## Upgrades

Back up the database and master key before an upgrade. Review the [release notes](https://github.com/dopbase/dopbase/releases) for migration and compatibility information before replacing the executable.

Since 0.2.0, single sign-on and people management are off by default. If an
older server used them, set `DOPBASE_SSO=true` and `DOPBASE_RBAC=true` (or
`sso = true` and `rbac = true` in `server.toml`) before you upgrade, or people
who sign in only through single sign-on are locked out. See
[optional features](#optional-features).

The server runs database migrations when it starts, and there is no automatic downgrade. A database opened by 0.1.10 cannot be used by 0.1.9, so to go back you restore the backup you made before the upgrade.

## Incident response

An incident plan should cover a stolen database, exposed master key, leaked service token, unavailable server, failed migration, and corrupted backup. Each case has different recovery and rotation requirements.

## Production readiness checklist

- A supported Dopbase release is installed.
- TLS and network exposure are reviewed.
- The database and master key use separate protected storage.
- Backups are automated and restoration has been tested.
- Logs and monitoring have been checked for secret leakage.
- Upgrade and incident procedures have named owners.
