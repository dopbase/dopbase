# Projects, environments, and secrets

Dopbase keeps its data model deliberately small. Most work happens within a project, an environment, and a set of secrets.

## Projects

A project represents one application or service, such as `payment-service`, `customer-portal`, or `worker`.

Projects provide the boundary for environments, membership, service tokens, and audit history. A project name should identify the application rather than the team that happens to own it today.

## Environments

An environment contains the values a project needs in a particular context. Common examples are `development`, `staging`, and `production`.

```text
payment-service
├── development
├── staging
└── production
```

The same key can have a different encrypted value in each environment. This lets an application keep a consistent configuration interface while using the right database, API account, or service endpoint at runtime.

## Secrets

A secret is an individually managed key and encrypted value:

```text
DATABASE_URL
STRIPE_SECRET_KEY
REDIS_URL
```

Dopbase does not treat an entire `.env` file as one database value. Individual records allow one secret to be updated, audited, versioned, revealed, or rolled back without replacing everything else.

## Environment inheritance

Inheritance is planned for a later release. It may allow environments to share non-varying values through a base environment while overriding selected keys.

The final rules must stay easy to trace. Dopbase will not add inheritance if a user cannot clearly determine which value reaches the application.
