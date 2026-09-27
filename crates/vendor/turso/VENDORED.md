# Vendored: `turso` 0.7.2 (the Rust SDK crate only)

Byte-for-byte the crates.io `turso` 0.7.2 package, except:

- `src/connection.rs`: two pass-through methods at the end of `impl Connection` —
  `set_query_timeout(Duration)` and `interrupt()` — marked "tender-db issue 425". The
  engine (`turso_core` 0.7.2) and the SDK kit (`turso_sdk_kit` 0.7.2,
  `TursoConnection::{set_query_timeout, interrupt}`) already implement both; the
  published SDK keeps the connection they live on private, so `/v1/sql` could not
  stop a query (issue 425).
- `Cargo.toml`: the examples, tests and dev-dependencies are dropped (none are
  shipped here); nothing else changes, and every dependency stays the registry
  0.7.2 (`turso_core`, `turso_sdk_kit` are NOT vendored).

Wired in by `[patch.crates-io]` in the workspace `Cargo.toml` and excluded from the
workspace members. It lives under `crates/` because the nix build's source fileset
is `Cargo.toml`, `Cargo.lock` and `crates/` only.

To drop it: upstream exposes both methods on `turso::Connection` (ask filed on
issue 425), bump `turso`, delete this directory and the patch entry.
