# Identity and tokens

Dopbase needs identities for people and for software.

## Human users

Human users access the admin interface and CLI. They authenticate with the active server and receive permissions for the organizations, projects, environments, and secret operations they need.

The final login method, session lifetime, recovery process, and multi-factor authentication support are not yet defined.

## Machine identities

CI jobs, servers, containers, deployment systems, and automation cannot depend on an interactive login. Dopbase plans to support machine identities and service tokens for these workloads.

```bash
export DOPBASE_TOKEN=dbs_xxxxxxxxxxxxxxxxx
dopbase run env_01ABCDEF -- npm start
```

The token format is provisional. `DOPBASE_TOKEN` is the planned automation
interface and is preferred over a saved human login when it is present.

Interactive `dopbase login` stores its token in the operating system credential
store under the normalized server URL. The global TOML config contains the
selected server but never the token. A saved credential is used only for its
matching server.

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

`dopbase run` removes Dopbase authentication variables before starting the
child process. Dopbase does not accept tokens as command-line arguments because
they may be exposed through process inspection or shell history.

If an operating system credential store is unavailable, use `DOPBASE_TOKEN`.
Dopbase must not silently store an interactive login token in plaintext.
