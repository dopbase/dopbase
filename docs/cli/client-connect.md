# `dopbase client connect`

`dopbase client connect` selects the server used by later client commands.

```bash
dopbase client connect http://localhost:8376
```

Connecting does not authenticate. Sign in separately after selecting an endpoint:

```bash
dopbase login
```

## Cloud uses the same command

```bash
dopbase client connect <dopbase-cloud-url>
dopbase login
```

The Cloud URL has not been published. Dopbase Cloud follows the same client and REST API model as a self-hosted server.

## Local client state

The client is expected to store the active endpoint and authentication material in local user configuration. It must not store a plaintext copy of every project secret.

The location, file format, token-storage mechanism, and support for named connection profiles are not final. The first release only requires one active endpoint.

## Safety behavior

The command should make the selected endpoint visible. Later operations must fail clearly if that endpoint cannot be reached. Dopbase must not silently switch servers or fall back from a self-hosted endpoint to Cloud.
