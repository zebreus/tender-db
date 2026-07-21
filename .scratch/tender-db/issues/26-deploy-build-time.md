# 26 — Deploy builds take 20+ min; wasm deps rebuilt every time

Status: needs-verification

## Resolution (measured, 2026-07-21) — diagnosis corrected

Measured on a 16-core NixOS machine (local; VPS ratio noted below). The
original diagnosis was wrong on two counts, and the real cause was bigger:

- **wasm-opt is NOT a factor.** dx 0.7.9 does its wasm size optimization at
  compile time via the `wasm-release` profile (`opt-level=s`); no separate
  wasm-opt pass runs on the real build. Fix direction 2 is moot.
- **wasm deps alone are not the bottleneck.** The wasm client path finishes at
  ~64s and runs *in parallel* with the ~258s native server path, so caching
  only wasm deps would save ~0 wall-clock.
- **Real root cause: crane's dep cache was 100% unused by dx.** `dx` builds the
  server under `--target x86_64-unknown-linux-gnu --profile server-release` and
  the client under `--target wasm32 --profile wasm-release`, landing in
  `target/<triple>/{server,wasm}-release/`. crane's `buildDepsOnly` cached deps
  under the plain `release` profile in `target/release/` — a directory dx never
  reads. So every deploy cold-built the ENTIRE ~1300-crate native + wasm
  dependency graph. That is the 20 min.

**Fix (nix/package.nix):** a new `bundleDeps` deps-only artifact primes the
cache with `dx` itself against crane's dummy sources — two separate `dx build`s
(server, then web) so fingerprints match by construction (matching dx's ad-hoc
profiles with plain cargo does NOT reproduce them — verified). The dummy
workspace-crate artifacts are dropped from the cache so the real bundle
recompiles the four workspace crates cleanly (dx passes the app's own lib to its
bin as `--extern`; a stale stub lib otherwise ships an empty client). The bundle
step reuses everything else. LTO left as-is: it only costs on cold builds, not
incrementally.

**Measured before/after (local, one-line server-code change → `nix build`):**

| | before | after |
|---|---|---|
| one-line server change | 258 s (4m18s) | **62 s** |
| cold build (clean store) | 261 s | 260 s (unchanged) |

Output verified real, not a stale dummy: the shipped wasm is 1,404,630 bytes
(byte-identical cold vs incremental; an empty dummy client would be absent).
Cold build stays reproducible; `nix flake check` (clippy) unaffected (it keeps
its own `release`-profile dep cache).

VPS ratio: local→VPS on full builds was ~4.6× (258 s ↔ 20 min), so 62 s local
projects to ~5 min on the VPS — under the 8-min target, but **final gate is a
real VPS deploy** (scheduled by the lead).

## Original report (diagnosis since corrected)

Every deploy costs ~20 min of nix build on the VPS even for a one-line
server change. Structural cause found in nix/package.nix: crane's
`buildDepsOnly` caches only the NATIVE dependency artifacts; the wasm
client deps are compiled by `dx bundle` inside the main derivation
(package.nix:54 says so explicitly) — so every deploy cold-builds the
entire dioxus/wasm dependency graph, then runs wasm-opt on top, plus a
thin-LTO native workspace build.

Fix directions, in expected-win order:
1. Cache wasm deps like native deps: a second deps-only artifact for the
   wasm target that `dx bundle` reuses (crane supports per-target
   artifacts; may need dx to accept a warm target dir — investigate how
   dx 0.7 caches and whether keeping its target dir across derivations
   is feasible in pure nix).
2. Measure and, if significant, skip `wasm-opt` for deploys (larger
   one-time dashboard download; fine for an ops dashboard) — keep it for
   tagged releases if wanted.
3. `lto = "thin"` → consider `lto = false` for the wasm side only if
   profiles can be split; keep server LTO (ingestion throughput).

Requirement: stay hermetic/nix — no dev-server-in-prod, no unpinned
toolchains (decided 2026-07-21; debug-profile runtime is catastrophic
for ingestion, and hot reload wouldn't cover the server-side changes
that actually drive deploys).

Acceptance: a one-line server-code deploy builds in < 8 min on the VPS;
build remains reproducible from a clean store.
