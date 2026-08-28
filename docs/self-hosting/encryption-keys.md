# Encryption keys

Dopbase must keep the master encryption key outside the database that stores encrypted secrets.

```text
dopbase.db
    +
master encryption key stored separately
```

This separation means that stealing the database alone should not reveal plaintext secret values.

## Planned key sources

The architecture may support:

- A protected environment variable
- A local file with restrictive permissions
- An operating-system secret store
- AWS KMS
- Google Cloud KMS
- Azure Key Vault
- A hardware security module

Only implemented and reviewed providers will appear in the production configuration reference.

## Operator responsibilities

- Restrict access to key material to the Dopbase process and authorized operators.
- Keep key backups separate from database backups.
- Prevent keys from appearing in shell history, process arguments, logs, or support bundles.
- Plan rotation before the original key is compromised or retired.
- Test recovery with the same provider and access policy used in production.

The exact key format, provider configuration, and rotation procedure are not final.
