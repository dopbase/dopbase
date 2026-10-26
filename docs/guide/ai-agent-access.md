---
title: "Give AI agents scoped access to projects"
description: "Let an AI coding agent read Dopbase projects with a personal token or a shared AI agent account, limited to the projects, environments, and level it needs."
---

# AI agent access

An AI coding agent often needs to know how a project is configured: which
variables exist in each environment, which ones the code references but nobody
has set, what belongs in `.env.example`. Dopbase gives it that with a token it
uses through `DOPBASE_TOKEN`, limited to the projects and level you choose.

There are two ways to do it:

| Use                                                         | When                                                             |
| ----------------------------------------------------------- | ---------------------------------------------------------------- |
| [Your own personal token](#use-your-personal-token)         | An agent on your machine works on your projects, as you          |
| [A shared AI agent account](#use-a-shared-ai-agent-account) | A team bot or a CI job needs its own identity, owned by a person |

Either way, the token never reaches more than its account does, and you can
revoke it at any time.

## What an agent should get

Start with **Names only** (`metadata`). The agent can then list projects,
environments, secret names, versions, timestamps, and the `.env` layout, which
covers tasks such as:

- checking that every variable the code references exists in Dopbase;
- comparing secret names across development, staging, and production;
- finding a missing or renamed variable;
- writing an `.env.example` with empty placeholders.

Give **Read** only when the agent must run something with real values, such as
integration tests, and only on the environment it needs. An AI agent account
never goes past Read. For a deployed application, use an environment-scoped
[runner token](./run-an-application) instead.

::: warning Names are sensitive too
Secret names and project structure say a lot about your infrastructure. Scope
the token to the project or environment the task needs.
:::

## Use your personal token

1. Open **Account** and choose **New token** under **Personal tokens**.
2. Name it after the agent or machine, such as `laptop-agent`.
3. Pick **Names only** and the project it needs, then a short expiry.
4. Copy the token when it appears. Dopbase shows it once.

Or from the CLI, signed in as yourself:

```bash
dopbase token personal create --name laptop-agent --level metadata \
  --project payment-service --expires-in 7d
```

Personal tokens start with `dpp_`. The token acts as you: if you lose access to
the project, so does the token.

## Use a shared AI agent account

An AI agent account has a name instead of an email, no password, and an owner:
the person who created it. It reaches only what it has been granted, and never
more than its owner. Disabling the owner disables the agent.

1. Open **Users** and choose **Add AI agent**. You become its owner.
2. Open the project's **Access** page and give the agent **Names only**, on the
   whole project or one environment.
3. Back on **Users**, choose **Get token** beside the agent, pick what the token
   can do and when it expires, and confirm it's you.
4. Copy the token when it appears.

Agent tokens last at most 90 days. Getting a new token revokes the agent's
previous one. Before deleting yourself, delete your agents or give them another
owner.

## Give the token to the agent

Keep the token in a password manager or a secret store, and load it into the
agent's shell before it starts, outside the conversation:

```bash
export DOPBASE_URL=https://dopbase.example.com
export DOPBASE_TOKEN="$AI_AGENT_TOKEN"

dopbase access
```

`dopbase access` shows the account, its grants, and the token's own limit, so
you can check what the agent will reach. Don't commit the token, put it in a
repository `.env` file, paste it into a prompt, or log it.

## Inspect configuration

The agent uses normal CLI commands. JSON output is easier for tools to parse:

```bash
dopbase --json project list
dopbase --json env list payment-service
dopbase --json secret list payment-service/staging
dopbase --json secret get payment-service/staging DATABASE_URL
```

With Names only, these return identifiers, names, versions, and timestamps,
never a value. An invalid, expired, or revoked token fails without falling back
to a saved human session.

For example, ask a coding agent:

```text
Find every environment variable referenced by this repository. Compare those
names with payment-service/staging in Dopbase and report missing or unused
names. Use metadata commands only. Do not attempt to reveal or change secrets.
```

## What the agent can't do

A Names only token is refused when it tries to read or change values:

```bash
dopbase secret get payment-service/staging DATABASE_URL --reveal
dopbase run payment-service/staging -- npm test
printf '%s' 'replacement' | \
  dopbase secret set payment-service/staging DATABASE_URL --stdin
```

No personal token manages people, grants, projects, or other tokens, whatever
its level. Don't swap in a human session to get around these limits.

## Revoke access

Revoke your own tokens on the **Account** page or with
`dopbase token personal revoke`. Revoke an agent's by getting it a new token or
deleting the agent on **Users**. Removing a grant cuts the token off on its next
request. Revoke a token as soon as it may have been exposed.

See [identity and access](/reference/identity#personal-tokens) for the full
rules.
