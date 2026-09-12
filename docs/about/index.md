---
title: "Project principles"
description: "The principles behind Dopbase: keep secrets management understandable, make self-hosting real, and avoid becoming another platform."
---

# Project principles

Dopbase exists to make application secrets easier to manage without turning secrets management into another infrastructure platform.

## Keep the product understandable

The core model remains projects, environments, and secrets. A new user should be able to understand that structure and start locally without reading an operations manual first.

## Prefer one clear path

Each task should have one obvious command and one documented workflow. When
several commands, aliases, or configuration paths produce the same result, a
developer must choose before acting and remember which path behaves differently.
That flexibility becomes a burden.

Dopbase adds another path only when it addresses a distinct use case that the
existing workflow cannot handle clearly. Developers should be able to learn the
Dopbase way once and repeat it without comparing equivalent options.

## Use constraints to build discipline

Dopbase takes inspiration from Rust's approach to safety. Rust encodes ownership
and borrowing rules in its type system, allowing the compiler to reject code that
violates those rules before it runs. Dopbase should also make important decisions
explicit instead of hiding them behind shortcuts or implicit behavior.

This does not mean making the product difficult for its own sake. Good tooling
removes accidental friction while preserving the steps that communicate intent.
For a secrets manager, the active server, identity, and environment should remain
clear to the developer.

The project deliberately limits its feature set so each supported workflow can
be documented, tested, secured, and maintained well. Quality in the core workflow
takes priority over adding more ways to perform the same task.

## Make self-hosting real

The open-source product should be useful on its own. A developer should be able to run the server, use the admin interface and API, manage secrets, and operate an installation without an artificial expiration or required Cloud account.

## Charge complexity to features

New capabilities must justify the complexity they add to setup, operation, and the mental model. Dopbase should improve its core secrets workflows before expanding into unrelated security products.

## Treat security as behavior

Encryption, authorization, redaction, auditability, key separation, recovery, and clear failure modes belong in the product design from the beginning.

## Keep the client portable

The same client should work with a local server, a production self-hosted deployment, or Dopbase Cloud. Changing the endpoint should not require learning a different secrets model.

The guiding question is:

> Does this make secrets simpler to manage without weakening security?

## Further reading

- [Background: Why Dopbase exists](./background): Why `.env` files are broken under AI-assisted development, and the real-world constraints of regulated financial systems.
- [Open source](./open-source): Licensing, distribution, and freedom from artificial paywalls.
- [Roadmap](./roadmap): Upcoming releases and operational milestones.
- [Product boundaries](./product-boundaries): Explicit capabilities and non-goals.
