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
team that happens to own it today. Names are unique within one Dopbase server,
and every project also receives an immutable ID.

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
├── development  env_01DEV...
├── staging      env_01STG...
└── production   env_01PRD...
```

Each environment belongs to exactly one project and receives an immutable ID.
The same key can have a different encrypted value in each environment.

Create and inspect environments with:

```bash
dopbase env create payment-service staging
dopbase env list payment-service
dopbase env show payment-service/staging
```

## Explicit targeting

Dopbase does not save an active project or environment and does not add a
configuration file to the repository. Instead, commands accept either a
readable environment reference or immutable ID:

```bash
dopbase secret list payment-service/staging
dopbase secret list env_01STG...
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
permission-controlled, and audited.

## Deletion

Deleting an environment also removes its secrets and scoped tokens. Deleting a
project removes all of its environments. Dopbase shows affected resource counts
and requires confirmation or `--yes` before either operation.

## Environment inheritance

Inheritance is planned for a later release. It may allow environments to share
non-varying values through a base environment while overriding selected keys.

The final rules must stay easy to trace. Dopbase will not add inheritance if a
user cannot clearly determine which value reaches the application.
