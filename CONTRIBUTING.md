# Contributing to Dopbase

Thanks for taking the time to work on Dopbase. The project is still pre-release, and much of the public documentation describes the product we are building rather than finished behavior. Check the code before assuming a documented command already exists.

## Before you start

Search the existing issues before opening a new one. For a substantial feature, architecture change, or new dependency, open an issue first and describe the problem you want to solve. Early discussion can prevent a large pull request from heading in a direction the project cannot use.

Security vulnerabilities do not belong in public issues. Follow [SECURITY.md](./SECURITY.md) instead.

## Repository layout

| Path     | Purpose                                   |
| -------- | ----------------------------------------- |
| `app/`   | Rust service and command-line application |
| `src/`   | Vue application                           |
| `docs/`  | Public VitePress documentation            |
| `tests/` | Frontend tests and shared test setup      |

The Rust service and Vue application are still scaffolding. Product behavior should stay consistent with the public documentation, but implementation findings may require the documentation to change.

## Set up the project

You need:

- [Bun](https://bun.sh/) for the frontend and documentation
- A Rust toolchain with Rust 2024 edition support

Install the JavaScript dependencies:

```bash
bun install
```

Start the Vue development server:

```bash
bun run dev
```

Run the Rust application:

```bash
cargo run --manifest-path app/Cargo.toml
```

Start the documentation site:

```bash
bun run docs:dev
```

## Make a change

Keep each pull request focused on one problem. Match the surrounding code, add tests where behavior changes, and update public documentation when a user-facing interface changes.

Do not commit generated output from `dist/`, `docs/.vitepress/dist/`, coverage reports, editor settings, local databases, credentials, or `.env` files.

Use commit messages that explain the change in plain language. A pull request should explain:

- What problem it solves
- How the solution works
- How it was tested
- Any compatibility, security, or migration concerns

Screenshots are useful for visible interface changes. Logs and screenshots must not contain credentials or private endpoints.

## Check your work

Run the checks that apply to your change.

Frontend:

```bash
bunx vitest run --passWithNoTests
bun run build
bunx prettier --check .
```

The repository does not contain a discoverable frontend test file yet, so `--passWithNoTests` keeps the scaffold check explicit without treating the absence of tests as a failure. Vitest will run normally as soon as `*.test.*` or `*.spec.*` files are added.

Rust:

```bash
cargo fmt --manifest-path app/Cargo.toml -- --check
cargo clippy --manifest-path app/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path app/Cargo.toml
```

Documentation:

```bash
bun run docs:build
```

If an existing unrelated failure prevents a check from passing, describe the failure and the command output in the pull request. Do not hide or silently skip it.

## Review expectations

Maintainers may ask for a smaller change, clearer tests, documentation, or a different interface. Review focuses on correctness, security, maintainability, and keeping Dopbase understandable for someone running it themselves.

Be patient and respectful. Reviews may take time, especially while the project is pre-release.

## Contribution license

Dopbase is licensed under the [Apache License 2.0](./LICENSE). Unless you state otherwise, any contribution you intentionally submit for inclusion in Dopbase is provided under that license, as described in section 5.

The project does not currently require a Contributor License Agreement or Developer Certificate of Origin sign-off.
