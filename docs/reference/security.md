# Security model

Dopbase stores credentials that can grant access to databases, cloud accounts, payment providers, and internal services. Security is part of the product behavior, not a layer added after storage and APIs are complete.

::: warning Design-stage security model
This page describes the intended architecture. It is not a completed threat model, security audit, or claim of production readiness.
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

The planned design uses envelope encryption:

```text
Secret value
    ↓ encrypted with a data encryption key
Ciphertext

Data encryption key
    ↓ encrypted with a master key
Encrypted data key
```

The database can store ciphertext, a nonce, the encrypted data key, an encryption version, and safe metadata. The master key remains in a separate key source.

AES-256-GCM and ChaCha20-Poly1305 are possible authenticated encryption schemes. The final algorithm, nonce strategy, key hierarchy, and library choices require implementation review and independent assessment.

## Data in transit

Networked clients need an authenticated, encrypted connection to the server. Local development may use localhost, but a production deployment must not send credentials over an unprotected network.

## Data exposure

Authorized users may need to reveal or export a value. These operations should be explicit, permission-controlled, and audited. Secret names can also disclose information, so access to metadata still needs authorization.

## Reporting vulnerabilities

Dopbase accepts private vulnerability reports through GitHub's private vulnerability reporting feature. Open the repository's **Security** tab, select **Report a vulnerability**, and complete the private advisory form.

Do not publish an undisclosed vulnerability in an issue, discussion, or pull request. Reports must not contain live credentials, customer data, private service URLs, or secrets from a system the reporter does not own. The full reporting and disclosure policy is in the repository's `SECURITY.md`.
