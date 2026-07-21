# 26 — Deploy builds take 20+ min; wasm deps rebuilt every time

Status: ready-for-agent

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
