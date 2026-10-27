---
title: "Projects, environments, and secrets"
description: "How Dopbase organizes application configuration: projects, environments, and individually managed secret records."
---

# Projects, environments, and secrets

Dopbase keeps its data model deliberately small. Most work happens within a
project, an environment, and a set of secrets.

## Projects

A project represents one application or service, such as `payment-service`,
`customer-portal`, or `worker`.

Projects provide the scope for environments, membership, service tokens, and
audit history. A project name should identify the application rather than the
team that happens to own it today. Names are unique within a **workspace**
(see [Workspace paths (design)](./workspace-paths)), and every project also
receives an immutable ID.

Manage projects explicitly:

```bash
dopbase project create payment-service
dopbase project list
dopbase project show payment-service
```

## Environments

An environment contains the values a project needs in one context. Common
examples are `development`, `staging`, and `production`.

```text
payment-service
├── development  env_482731
├── staging      env_190482
└── production   env_735104
```

Each environment belongs to exactly one project and receives an immutable,
six-digit ID. The number is random, so IDs do not reveal creation order. The
same key can have a different encrypted value in each environment.

Create and inspect environments with:

```bash
dopbase env create payment-service/staging
dopbase env list payment-service
dopbase env show payment-service/staging
```

## Explicit targeting

Dopbase does not save an active project or environment and does not add a
configuration file to the repository. Instead, commands accept either a
readable environment reference or immutable ID:

```bash
dopbase secret list payment-service/staging
dopbase secret list env_190482
```

The environment identifies its project, so callers do not select both. Use the
readable form during interactive work and immutable IDs in deployments.

Read [target projects and environments](/cli/environment-targeting) for the
production and staging server workflow.

## Secrets

A secret is an individually managed key and encrypted value:

```text
DATABASE_URL
STRIPE_SECRET_KEY
REDIS_URL
```

Dopbase does not treat an entire `.env` file as one database value. Individual
records allow one secret to be updated, audited, versioned, revealed, or rolled
back without replacing everything else.

```bash
dopbase secret set payment-service/staging DATABASE_URL
dopbase secret get payment-service/staging DATABASE_URL
dopbase secret get payment-service/staging DATABASE_URL --reveal
```

Reading metadata does not reveal a value. Reveal operations are explicit,
permission-controlled, and audited. The official CLI requires interactive
password confirmation every time `--reveal` is used.

### Versions and rollback

Every save gives a secret a new version number: setting it, importing it, or
rolling it back. Dopbase keeps each version encrypted with the same envelope
encryption as the current value, so you can see who changed a secret and when,
and restore an earlier value:

```bash
dopbase secret history payment-service/staging DATABASE_URL
dopbase secret rollback payment-service/staging DATABASE_URL 3
```

A rollback saves the earlier value as a new version. If `DATABASE_URL` is at
v5 and you roll back to v3, it becomes v6 with the value of v3, and v1 to v5
stay as they were. The server does this in one transaction, so runtime clients
see either the old value or the restored one, never a partial change.

Retention keeps the value of the newest 20 versions of each secret. Older
versions keep only their metadata (the change, who made it, and when) and
show as `expired`. Change the count with `secret_history_versions` in
`server.toml`, or `DOPBASE_SECRET_HISTORY_VERSIONS`, to anything from 2 to 1000. Dopbase applies it when a secret is next saved, so lowering the count
does not remove old values from secrets that stop changing. Each retained
version takes about the size of its value plus a few hundred bytes.

Secrets that existed before version history was added start with one version
(their current value), shown as "before history". Deleting a secret, its
environment, or its project deletes the history too. Rolling back a whole
environment at once is not supported yet.

## Deletion

Deleting an environment also removes its secrets, their version history, and
its scoped tokens. Deleting a
project removes all of its environments. Dopbase shows affected resource counts
and requires confirmation or `--yes` before either operation.

## Environment inheritance

Inheritance is planned for a later release. It may allow environments to share
non-varying values through a base environment while overriding selected keys.

The final rules must stay easy to trace. Dopbase will not add inheritance if a
user cannot clearly determine which value reaches the application.
