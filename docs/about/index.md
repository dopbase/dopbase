---
title: "Project principles"
description: "The principles behind Dopbase: keep secrets management understandable, make self-hosting real, and avoid becoming another platform."
---

# Project principles

Dopbase exists to make application secrets easier to manage without turning secrets management into another infrastructure platform.

## Keep the product understandable

The core model remains projects, environments, and secrets. A new user should be able to understand that structure and start locally without reading an operations manual first.

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
