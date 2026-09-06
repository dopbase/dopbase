# Frontend architecture

The Admin UI keeps a simple flow:

```text
page or component -> controller -> service -> HTTP client
                         |
                       store
```

Pages render values and events returned by their controller. A page does not
import an API module or a Pinia store. A controller owns screen state,
validation, navigation, and the messages shown after an operation. A service
contains endpoint paths, request and response types, and transport mapping.
The HTTP client is the only place that calls `fetch`.

Use a store only when state is shared by more than one screen. Keep pure
parsing, formatting, and validation in `src/utils`. Reactive behavior that is
shared by several controllers belongs in `src/composable`.

When tracing an action, start at the template event, find the controller
method returned to that template, then follow its service call. Error codes
come from `ApiError`; controllers map stable codes to field or operation
messages. Do not branch on English server messages.

Read requests should accept an `AbortSignal` when the screen can change while
they are running. The owner cancels an older request before starting a newer
one and ignores completions from a different target. Mutations snapshot their
target before awaiting and refresh state only after the mutation succeeds.

Run `bun run typecheck`, `bunx eslint src tests`, and
`bunx vitest run --configLoader runner` before submitting frontend changes.
