# Domain context

Architecture decisions for the **an1by/dopbase** fork (upstream merges, workspaces,
Docker, Windows) are recorded in [docs/adr/](./docs/adr/index.md).

## Project

A project is one application or service. It contains one or more environments.

## Environment

An environment is a named deployment context within a project, such as development, staging, or production. It owns its secrets and environment-scoped runner tokens.

## Role

Every account has one fixed role: root, admin, member, or ai_agent. Role-based access control (RBAC) will replace the fixed roles. Code that hard-codes a role carries a `TODO(rbac):` comment, which CONTRIBUTING.md describes under "Mark code that RBAC will replace". Add the marker whenever new code depends on a fixed role.

## Workspace

A workspace groups projects that share a filesystem layout on a machine. Each linked project has a relative path inside the workspace. Workspace-scoped runner tokens can read runtime secret values for every environment whose project is linked to that workspace through `project_locations`. Projects without a workspace link are outside that token scope.

