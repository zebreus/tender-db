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

**`set_query_timeout` is not for serving connections (issue 438, 2026-09-27).** While a
limit is set, `turso_core` evaluates `io.current_time_monotonic() >= deadline` before
EVERY VDBE instruction (`vdbe/mod.rs` `maybe_request_interrupt`, a `clock_gettime` per
call). Measured locally, changing nothing but the timeout (0 vs 600 s): covering-index
GROUP BY 2.9× slower, scans 1.7–2.7×, sorter group-bys 1.7–2.1× — and issues 425/120 had
set it on `/v1/sql` and both REST pools. The API now stops a read with a timer that calls
`interrupt()` (`crates/app/src/v1/stop.rs`): the interrupt flag is read in that same
per-instruction check whether or not a limit is set, so it costs nothing until it fires.
The one caller left is the offline `plan-probe mem` instrument (issue 426).

Wired in by `[patch.crates-io]` in the workspace `Cargo.toml` and excluded from the
workspace members. It lives under `crates/` because the nix build's source fileset
is `Cargo.toml`, `Cargo.lock` and `crates/` only.

To drop it: upstream exposes `interrupt()` on `turso::Connection` — and, if
`set_query_timeout` is exposed too, has the engine read the clock every N steps rather
than before every instruction (the request is drafted on issue 425, step 4 — not yet
posted; the clock-read point was added by issue 438) — bump `turso`, delete this
directory and the patch entry.
