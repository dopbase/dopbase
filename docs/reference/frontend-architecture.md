---
title: "Frontend architecture"
description: "The internal architecture and conventions of the Dopbase Admin UI: controllers, services, stores, and HTTP communication."
---

# Frontend architecture

The Admin UI follows a layered design:

```mermaid
flowchart LR
    accTitle: Dopbase Admin UI Layered Architecture
    accDescr: Visual flow showing how pages interact with controllers, services, stores, and the HTTP client.

    PAGE["Page or Component"] -->|"User actions & events"| CONTROLLER["Controller"]
    CONTROLLER -->|"Reactive state"| PAGE
    CONTROLLER -->|"API methods"| SERVICE["Service"]
    CONTROLLER <-->|"Cross-screen state"| STORE[("Pinia Store")]
    SERVICE -->|"Transport request"| HTTP["HTTP Client"]
    HTTP -->|"fetch"| BACKEND[("REST API")]

    classDef view fill:#0d0b14,color:#ffffff,stroke:#863bff,stroke-width:2px;
    classDef logic fill:#ede6ff,color:#0d0b14,stroke:#863bff;
    classDef network fill:#e8f7ff,color:#0d0b14,stroke:#47bfff;
    classDef storage fill:#fff7df,color:#0d0b14,stroke:#c98900;
    classDef backend fill:#e6f8ed,color:#0d0b14,stroke:#219653;

    class PAGE view;
    class CONTROLLER logic;
    class SERVICE,HTTP network;
    class STORE storage;
    class BACKEND backend;
```

Pages render values and events returned by their controller. A page does not
import an API module or a Pinia store directly. A controller owns screen state,
validation, navigation, and feedback messages. A service contains endpoint paths,
request and response types, and transport mapping. The HTTP client is the only
place that calls `fetch`.

## State and logic boundaries

- **Stores**: Use a store only when state is shared across multiple screens.
- **Utilities**: Keep pure parsing, formatting, and validation in `src/utils`.
- **Composables**: Put reactive behavior shared by multiple controllers in `src/composable`.

## Action tracing and errors

When tracing an action, start at the template event, find the controller method
returned to that template, then follow its service call.

Error codes come from `ApiError`; controllers map stable error codes to field or
operation messages. Never branch on English server messages.

## Asynchronous requests

Read requests should accept an `AbortSignal` when the screen can change while
they are in flight. The owner cancels an older request before starting a newer
one and ignores completions from a different target.

Mutations snapshot their target before awaiting and refresh state only after the
mutation succeeds.

## Verification

Run these checks before submitting frontend changes:

```bash
bun run typecheck
bunx eslint src tests
bunx vitest run --configLoader runner
```
