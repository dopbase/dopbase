---
title: "Admin UI: AI agents"
description: "Add token-only accounts for bots and CI in Dopbase, give them access, create and replace their token, and change their owner in the Admin UI."
---

# AI agents

An AI agent is an account for a bot or a CI job. It has no email and no
password, only a token, and it always has an owner: the person who answers for
it. An agent reaches only what it is granted, never past its role's level
(Read for the built-in AI agent role), and never more than its owner does. If
the owner is disabled, so is the agent.

An agent running on your own machine doesn't need an account. Create a
personal token with **Names only** from your account menu instead.

Open **AI agents** from the sidebar. Anyone with `users.view` can see the list.
The table shows each agent's owner, the deepest level it can reach, its token
(`active`, `expires in 3 days` once it is a week from expiring, or
`no token`), when the token expires, and when it was last used. Add an
**Owner** chip to see one person's agents.

## Add an agent

Choose **Add AI agent**, give it a name such as `review-bot`, and choose
**Add agent**. Adding agents needs `users.manage`. You become its owner and
land on its page. It starts with no access.

## An agent's page

The page header names the owner and the deepest level the agent can reach.
**Activity** opens the audit log filtered to the agent, and **Transfer owner**
gives it to someone else.

- **Access** lists its grants, as on a person's page. Choose **Give access**
  to give it a project or one environment. Levels above its role are shown
  but can't be picked.
- **Tokens** lists its current token and the ones before it.
- **Danger zone** disables or deletes the agent.

Its owner, or anyone with `users.manage`, can change it.

## Create or replace its token

An agent holds one token at a time. Choose **New token**, on the Tokens tab or
in the list's **⋯** menu, and set:

- **Name**: lowercase letters, numbers, and hyphens, such as `ci-deploy`. Name
  it after where it's used.
- **Can**: Names only, or Read. The role sets the most a token can do.
- **Where**: every project the agent reaches, or one of its grants.
- **Expires**: 7, 30, or 90 days, or a custom number of days, within the
  role's limit.

If the agent already has a token, a note says the new one replaces it.
Creating the new token revokes the old one right away, so update wherever it
is used. Dopbase shows the token once, with an `export DOPBASE_TOKEN=...` line
and a `dopbase run` line for where it reaches. Copy it straight into a secret
store, then choose **I've stored it**.

To stop a token without making a new one, choose **Revoke** on the Tokens tab
or **Revoke token** in the list's **⋯** menu.

## Transfer, disable, or delete

**Transfer owner** lists the people who can own it. The agent keeps its grants
and token, but never reaches more than its new owner does.

**Disable agent** stops its token; its access and history stay. **Delete
agent** asks you to type its name, then removes it with its grants and tokens.
Audit events stay.

The [AI agent access guide](/guide/ai-agent-access) shows how to pass the
token to an agent and what it can do with it.
