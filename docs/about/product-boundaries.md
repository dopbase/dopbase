---
title: "Product boundaries"
description: "What Dopbase is not: no certificate authority, PKI, SSH platform, database proxy, or identity provider — a focused secrets manager."
---

# Product boundaries

Dopbase manages application secrets. It should not become a general infrastructure security suite by accumulating every adjacent feature.

The project does not currently aim to become:

- A certificate authority or general PKI system
- An SSH or privileged-access platform
- A database proxy
- An identity provider
- A Kubernetes management layer
- A general certificate manager

These are legitimate problems, but solving them would broaden Dopbase beyond its core model.

## How features are evaluated

Before adding a feature, the project should ask:

1. Does it directly improve application secrets management?
2. Can a new developer still understand Dopbase quickly?
3. Does it make the single-executable experience harder?
4. Are users asking for it?
5. Does it solve a user problem rather than copy a competitor?

Dopbase should first deepen version history, CLI behavior, permissions, auditing, encryption, reliability, and developer workflows.
