# 26 — Deploy builds take 20+ min; wasm deps rebuilt every time

Status: resolved

## VPS verification (2026-07-21, team lead)

Steady-state (pure server-code change, deploy #5 / rev 546189d,
measured from deploy.sh timestamps): local pre-flight 69s; full deploy
— push, VPS build, atomic switch, restart, health-green — 3m29s; total
change-to-live ≈4.6 min. Dep-change case (deploy #4): ~10.5 min incl.
the one-time bundleDeps rebuild, by design. Baseline before the fix:
~20+ min every deploy. Acceptance (<8 min server-code deploy) met with
margin.

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

## 2026-07-21 15:11 — b0a5cdb: build duration not derivable from box (run-driver)

Tried to capture the first-deploy build number. Could not derive it cleanly:
`/opt/tender-db/deploy-build.log` has a stale mtime (Jul 19 17:07) — the b0a5cdb
build did not route through it, so no start/end pair to diff. Only firm datum:
service `ExecMainStartTimestamp=15:09:37 UTC`, and the watcher first saw rev
b0a5cdb at 15:11:01 UTC → **startup/migration pause ≈ 84s** (index build +
tenders current_seq backfill + job_queue.progress ALTER), within the 120s grace.
That's the migration cost, not the build cost. Per the plan, the NEXT deploy is
the steady-state build measurement — will capture start/end then.

## 2026-07-21 16:40 — build duration from operator's deploy.sh (team lead + run-driver)

Correcting my "not derivable from box" note: the build output streams to the
OPERATOR's `deploy.sh`, not to `/opt/tender-db/deploy-build.log` (that file's
Jul-19 mtime is stale — it is NOT the measurement source). From the lead's
deploy.sh timestamps for b978c96: start 16:23:42 → deploy complete ~16:34:19
(service restart 16:34:01) → **total ≈ 10.5 min**.

Caveat: b978c96 was a **dep-change** deploy (issue 38 dropped a dependency,
which by design invalidates the dep cache → one-time bundleDeps rebuild). So
this is the dep-change case: **~10.5 min (dep-change) vs ~20 min pre-fix cold**.
The server-code-only **steady-state** number (no dep change → warm dep cache)
is still unmeasured; it will come from the next no-dep-change deploy.
