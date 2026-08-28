# Identity and tokens

Dopbase needs identities for people and for software.

## Human users

Human users access the admin interface and CLI. They authenticate with the active server and receive permissions for the organizations, projects, environments, and secret operations they need.

The final login method, session lifetime, recovery process, and multi-factor authentication support are not yet defined.

## Machine identities

CI jobs, servers, containers, deployment systems, and automation cannot depend on an interactive login. Dopbase plans to support machine identities and service tokens for these workloads.

```bash
export DOPBASE_TOKEN=dbs_xxxxxxxxxxxxxxxxx
dopbase run -- npm start
```

The variable name and token format are provisional.

## Permission model

The planned permission model may distinguish these operations:

- View secret names and metadata
- Create or update secrets
- Reveal plaintext values
- Export an environment
- Manage projects and environments
- Manage users and service tokens
- Read audit history

Basic role-based access control is planned for v0.1. More advanced policy features should wait until the basic model is clear and tested.

## Token handling

Tokens must be scoped, revocable, and hidden from logs. Operators should use the narrowest permissions available and rotate a token immediately if it may have been exposed.
