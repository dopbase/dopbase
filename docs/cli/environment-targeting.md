---
title: "Target projects and environments"
description: "Target a Dopbase project and environment explicitly without saving an active environment or adding configuration to an application repository."
---

# Target projects and environments

Dopbase uses explicit environment targeting. It does not place configuration
in an application repository and does not remember an active project or active
environment.

The server connection is separate machine-global state. With no configured
server, Dopbase uses `http://localhost:8840`; `client connect` can select another
endpoint. See [client configuration](./configuration).

This keeps the selected environment visible at the point where secrets are
read or changed:

```bash
dopbase secret list storefront/production
dopbase import storefront/staging .env.staging
dopbase run env_01ABCDEF -- ./storefront
```

## Why the environment is enough

An environment belongs to exactly one project:

```text
storefront
├── development  env_01DEV...
├── staging      env_01STG...
└── production   env_01PRD...
```

The environment reference therefore identifies both the project and the set
of secrets. Commands do not need a separately selected project.

Humans can use `storefront/staging`. Deployment systems should use the
immutable environment ID so a rename cannot change what the server runs.

## Start from an existing `.env`

Connect and sign in, then bootstrap the project and first environment:

```bash
dopbase client connect https://dopbase.example.com
dopbase login
dopbase init storefront development --from .env
```

`init` validates the file and atomically creates the project, environment, and
individual secrets. It does not write a Dopbase configuration file into the
repository.

Add the other environments explicitly:

```bash
dopbase env create storefront staging
dopbase import storefront/staging .env.staging

dopbase env create storefront production
dopbase import storefront/production .env.production
```

The create and import commands print each environment's immutable ID. Keep the
production and staging IDs in the corresponding deployment configuration, not
in the application repository.

## Run the same project on two servers

Assume both application servers connect to the same Dopbase endpoint. Create a
runner token scoped to each environment:

```bash
dopbase token create storefront/production \
  --name storefront-production-server --role runner

dopbase token create storefront/staging \
  --name storefront-staging-server --role runner
```

Configure the production application server:

```bash
export DOPBASE_URL=https://dopbase.example.com
export DOPBASE_TOKEN=<production-runner-token>
dopbase run env_<production-id> -- ./storefront
```

Configure the staging application server with its own environment ID and
token:

```bash
export DOPBASE_URL=https://dopbase.example.com
export DOPBASE_TOKEN=<staging-runner-token>
dopbase run env_<staging-id> -- ./storefront
```

The application artifact can be identical on both servers. The environment ID
and scoped runner token determine which values reach each process. If the
staging server is accidentally given the production environment ID, its
staging token cannot access that environment and Dopbase stops before starting
the application.

For a service definition that keeps runtime settings in environment variables,
set `DOPBASE_ENV` and omit the positional reference:

```bash
export DOPBASE_ENV=env_<production-id>
dopbase run -- ./storefront
```

This is still explicit deployment configuration. Dopbase does not persist it
as an active environment.

## Different Dopbase servers

Environment IDs are local to the Dopbase server that created them. If two
application servers use different Dopbase instances, each deployment must set
its own `DOPBASE_URL`, token, and environment ID. Dopbase never silently falls
back to another endpoint or maps IDs between servers.
