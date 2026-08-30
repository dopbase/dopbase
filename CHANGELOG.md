# Changelog

All notable changes to Dopbase are documented in this file.

## Unreleased

### Added

- `dopbase serve --port <PORT>` and `--host <HOST>` flags (plus `port`/`host`
  keys in `server.toml` and `DOPBASE_PORT`/`DOPBASE_HOST` environment
  variables) as the simple way to configure the network address. With a
  loopback bind and no explicit `public_url`, the public URL is derived from
  the port. Binding beyond loopback without `public_url` fails with guidance
  instead of guessing.
- `dopbase update` checks GitHub for a newer release and prints the current
  version, the latest version, and the release URL. Informational only —
  Dopbase does not self-update.
- `dopbase serve --background` starts the server as a detached daemon with a
  PID file (`dopbase.pid`) and log file (`serve.log`) in the data directory.
  Startup failures are reported to the terminal before the command returns.
- `dopbase stop` gracefully stops the background server, escalating to a
  forced stop after `--timeout` seconds (default 10), and cleans up stale PID
  files.
- `docs` server setting with `--docs`/`--no-docs` flags and `DOPBASE_DOCS`
  environment variable to control the Swagger UI and OpenAPI document.

### Changed

- The default server port is now **8840** (was 8376). Existing `server.toml`
  files with an explicit `bind_address` are unaffected; default installs will
  serve on `http://localhost:8840` after upgrading.
- The Swagger UI (`/api/docs`) and OpenAPI document (`/api/v1/openapi.json`)
  are now disabled by default; pass `--docs` to enable them.

## 0.0.1 - 2026-08-29

### Added

- Testing release of the Dopbase server, CLI, REST API, and embedded Admin UI.
- Self-contained release binaries for Linux and macOS on x64 and ARM64.
