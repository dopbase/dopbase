---
title: "Security model"
description: "The Dopbase security model: encryption of secret values, separation of master key material, audit records, and access control."
---

# Security model

Dopbase stores credentials that can grant access to databases, cloud accounts, payment providers, and internal services. Security is part of the product behavior, not a layer added after storage and APIs are complete.

::: info Review the security model
This page documents the 0.0.8 security model. Public source makes independent
review possible, but it is not the same as an independent security audit.
:::

## Security goals

- Encrypt secret values before persistence.
- Keep the master encryption key outside the secrets database.
- Use authenticated encryption so modified ciphertext is detected.
- Keep cryptographic code isolated and auditable.
- Reveal plaintext only for authorized operations.
- Never include plaintext secrets in application or audit logs.
- Record sensitive actions without recording their values.

## Envelope encryption

The v0.0.8 implementation uses envelope encryption:

```text
Secret value
    ↓ encrypted with a data encryption key
Ciphertext

Data encryption key
    ↓ encrypted with a master key
Encrypted data key
```

Each stored value receives a random 256-bit data key. XChaCha20-Poly1305
encrypts the value and separately wraps its data key with the master key. The
environment ID, secret key, version, and algorithm version are authenticated as
additional data. Unique random nonces are stored with the ciphertext.

The database contains ciphertext, wrapped keys, nonces, versions, and safe
metadata. The 256-bit master key remains in a separate owner-only file and is
verified before the HTTP listener starts.

## Data in transit

Networked clients need an authenticated, encrypted connection to the server. Local development may use localhost, but a production deployment must not send credentials over an unprotected network.

## Data exposure

Authorized users may need to reveal or export a value. These operations should be explicit, permission-controlled, and audited. Secret names can also disclose information, so access to metadata still needs authorization.

## Reporting vulnerabilities

Dopbase accepts private vulnerability reports through GitHub's private vulnerability reporting feature. Open the repository's **Security** tab, select **Report a vulnerability**, and complete the private advisory form.

Do not publish an undisclosed vulnerability in an issue, discussion, or pull request. Reports must not contain live credentials, customer data, private service URLs, or secrets from a system the reporter does not own. The full reporting and disclosure policy is in the repository's `SECURITY.md`.
