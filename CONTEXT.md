# Domain context

## Project

A project is one application or service. It contains one or more environments.

## Environment

An environment is a named deployment context within a project, such as development, staging, or production. It owns its secrets and runner tokens.

## Principal

Who makes a request: root, an account (a person or an AI agent), or a runner token. A personal token is not a principal of its own; it acts for its account.

## Root

The account created during first-run setup. It is a flag on the account, not a role, holds every capability, and is the way back in when everything else fails.

## Role

What an account may do on the instance (its capabilities) and how deep into project data its grants can reach (its level ceiling), plus how holders sign in and what their personal tokens may do. Roles are rows in the `roles` table. Admin, Member, Auditor, and AI agent are built in; root creates custom roles. Code never decides by a role's name.

## Capability

A power over the instance, such as `users.manage` or `audit.read_all`. The list is fixed in `app/src/services/authx.rs`; which roles hold each one is data. Some capabilities belong to root alone and no role can hold them.

## Level

How far into a project or environment an account reaches: none, metadata (names only), read, write, manage, or project manager. Each includes the ones before it.

## Grant

A level on all projects, one project, or one environment, given to a user or a group. The most specific grant wins, capped by the role's ceiling.

## Group

A named set of accounts that receives grants together. Groups are flat and never hold root.

## Protected environment

An environment that grants on its project, below project manager, don't reach.

## Personal token

A token that acts as one account, within its own scope and level, and always expires. AI agents only ever use personal tokens.

## AI agent

An account whose role signs in with tokens only. It has an owner and never reaches more than its owner does.

## Workspace

Workspace is not a Dopbase domain term. Use it only for development concepts such as an editor or filesystem workspace.
