# 260 — superseded test binaries fill the build disk, and the third symptom does not look like a disk problem

Status: FIXED-BUT-RECURRING — **the second cause has a rule now (2026-09-18):** the gate's prune also drops superseded hash variants of dependency archives over 100 MB (keep the newest per stem), which is what the 2026-09-10 recurrence and today's near-miss were made of (see the foot). The first cause (superseded test binaries) has been pruned since 2026-08-20.
runs in one session filled the allowance anyway, in artifacts the prune does not cover. See the
recurrence at the end, including the `df` reading that makes this look like plenty of free space.
Was: FIXED 2026-08-20 (the prune; `ops/check.sh` now runs it before building). Filed anyway because
the SYMPTOM is worth recording — one of the three failures does not mention the disk at all
Kind: operational / build hygiene
Blocked by: —
Relates to: 254 (why `ops/check.sh` exists at all)

## What happens

`cargo` names each test executable `<target>-<16 hex of the build hash>` and **never removes the
previous one**. This workspace has ~12 test targets, each linking the whole dependency tree
(turso, tantivy, reqwest, rustls, …), so each is **200–350 MB**. Every rebuild after a source change
mints a fresh hash and leaves the old file behind.

A day of ordinary iteration is therefore tens of GB of dead executables. Measured on 2026-08-20 while
clearing it: **9.0 GB** of pre-today files in one sweep, then 2.3 GB, 3.28 GB and 3.96 GB more in three
later sweeps as the day's own rebuilds accumulated.

## The three faces of it, and why the third is the reason this is filed

1. `error: failed to build archive at …: No space left on device (os error 28)` — unambiguous.
2. `rustc-LLVM ERROR: IO failure on output stream: No space left on device` — still says it.
3. **`collect2: fatal error: ld terminated with signal 7 [Bus error]`**, under a banner reading
   `PLEASE submit a bug report to https://github.com/llvm/llvm-project/issues/` and a full linker
   command line. Nothing in that names the disk.

A bus error from the linker is what a failing `mmap` write looks like from inside `ld`: the file is
extended, the pages are mapped, and the store has no blocks left when they are touched. It reads
exactly like a toolchain bug, and the invitation to file one against LLVM is actively misleading. The
tell is `df` showing `Avail` at a few hundred MB with `Use% 100`.

Worth knowing in this environment specifically: the note in the system prompt about writable disk being
a fixed per-session allowance means `df` looks odd — "Avail" hits 0 while "Used" is modest — so the
usual "the machine is fine, 37G of 252G used" glance is reassuring and wrong.

## Fix

`ops/check.sh` prunes before it builds: for each test target, keep the newest hash and delete the rest.
That is the one the next build will reuse; anything else is stale, and if a pruned hash IS wanted again
it costs a relink, not a rebuild.

Deliberately narrow — it only touches executable files in `target/debug/deps` matching
`<name>-<16 hex>` **and** larger than 20 MB. It never touches `.rlib`s, build-script outputs, or
anything outside `deps/`, so the dependency tree (the expensive part) is untouched and the worst case
is one extra link.

## Not done

- No cap on total `target/` size and no periodic sweep — the gate runs often enough in practice, and a
  size-based policy would need a number nobody has measured a good value for.
- The `.a`/`.so` pairs for the two turso sdk-kit crates are 1.2 GB together and are current, not
  superseded, so pruning cannot touch them. If headroom gets tight again they are where the next
  GB lives.

## Root-cause fix (2026-08-24, cleanup mandate): debuginfo off in the PROFILE

After the fifth fill in a week (Lennart asked whether new tests caused it — no: three new
binaries are marginal against ~58 × 200-350 MB × N generations), the load-bearing flaw was
that small builds depended on HOW you invoked cargo: only check.sh's env vars turned
debuginfo off, and every raw `cargo test`/`cargo build` paid full price and left the old
binaries behind. Fixed in Cargo.toml: `[profile.dev] debug = "line-tables-only"` — every
build small by default, panic backtraces keep file:line, check.sh's env override is now
belt-and-braces. The pruning in check.sh stays (the graveyard mechanic itself is cargo's,
only its per-binary cost shrank ~5-10x).

## Recurred 2026-09-10 despite the prune — the prune is necessary, not sufficient

Four `ops/check.sh` runs in one session (three green, each after edits to `canonical.rs` and
`data_quality.rs`) filled the container's writable allowance. The fourth died mid-link:

```
error: failed to build archive at .../libingest-*.rlib: No space left on device (os error 28)
error: linking with `cc` failed: ... ld terminated with signal 7 [Bus error]
```

**GATE-EXIT=101 with ZERO `FAILED` lines** — the fourth face of this issue, and it wears the same
disguise as the third: a compile error prints no `test result:` line, so a grep for failures reports
nothing wrong. Reading the echoed `GATE-EXIT=` value is what caught it, exactly as CLAUDE.md says.

| | |
| --- | --- |
| `target/` at failure | **24 GiB** |
| of which `target/debug/deps` | **18 GiB** |
| recovered by `cargo clean` | **24.5 GiB, 26,459 files** |
| free after | 24 GiB |

**`df` lies about the headroom, and this is the part worth remembering.** It reported
`252G size, 38G used, 31M avail, 100%` — a reader would see 214 GB of slack. The writable allowance
is per-session and is what "avail" tracks; the size column is not a budget. **Low "Used" with zero
"Avail" means the allowance is spent, not that the disk is broken.**

**Why the prune did not save it.** The prune removes superseded test EXECUTABLES; most of today's 18
GiB in `deps` was rlibs and fresh artifacts from rebuilding two large crates four times. So the fix
this issue landed still works and is still worth having — it just addresses one of the two things
that grow.

**Not fixed here, and deliberately.** The cheap mitigations (a `cargo clean` when free space drops
below a threshold; `--profile` sharing so gate runs reuse artifacts) each have a cost measured in gate
minutes, and one session hitting it is not enough evidence to pick one. What this entry buys is that
the next `GATE-EXIT=101` with no failing test is diagnosed in a minute rather than debugged as a code
error. Reopen with a rule if it happens twice more.

## 2026-09-18 — the near-miss priced, and the rule the 09-10 entry asked for

Five gates in one afternoon (issues 386/388/416) took the allowance from 5.3 GB free to **3.5 GB**
with `target/` at 20 GB — the 09-10 shape, one or two gates from the link error. Sized before it
bit: `target/debug/deps` held **nine hash variants each of `libturso_sync_sdk_kit` and
`libturso_sdk_kit`** (200–300 MB apiece, mtimes 09-10 to 09-12, one profile/feature change per
variant) and three of `libturso_core` — ~4.5 GB of archives the current build could not reference.
Deleting all but the newest per stem freed 3.1 GB (3.5 → 6.6 GB) and the next gate did NOT rebuild
turso: the newest was the live one.

That is the rule the "reopen with a rule" line was waiting for, and it is the same shape as the
test-binary prune: **`ops/check.sh` now keeps the newest hash variant of every dependency archive
over 100 MB and removes the rest**, in the same prune step, before the build. The cost of being
wrong (a stale variant cargo still wanted) is one crate's rebuild, never a wrong build; the cost of
not having it is the 09-10 failure at the next profile change. `cargo clean` stays the recovery when
the allowance is already gone.

## 2026-09-29 — the third cause: every gate run leaves ~0.6 GB of test databases in /tmp

The container had 3.3 GB free after a deploy's gate. `/tmp` held **6,573 `tender-db-*` files, 4.2 GB**, from the
seven gates run since 11:16 UTC. The two-hour rule above never saw them: each file was younger than two hours at the
next gate's start.

- It is not only panicking tests. About 230 tests leave their `.db` (4 KB) and its `-wal` (2.6 MB, up to 11 MB) on
  EVERY run. Examples: every `orphan-sweep-*`, `test-typo-*`, `dupid-*` and `cluster-*` scratch, and `sup-*` (644
  files, 473 MB).
- **Fix** (`ops/check.sh`, `sweep_run_scratch`): the gate sweeps its own run on the way out, pass or fail, via an
  EXIT trap. It removes every `/tmp/tender-db-*` file written since the gate started that no process holds open,
  read from `/proc/*/fd`. A test still running elsewhere holds its database open, so it is left alone. The two-hour
  rule stays for runs that were killed before their trap.
- Exercised on a held and an unheld pair: the held pair survived, the unheld pair went, and the held pair went once
  closed. One manual sweep with the same guard removed 6,523 files, 4.14 GB; the disk went from 3.3 GB to 7.5 GB free.
- **Not fixed**: the tests themselves still leak. Each would need a drop guard that removes the `-wal`/`-shm` too.
  The sweep makes that a tidiness question rather than a disk one.

## 2026-09-29 — the archive prune churned the turso kits on every gate (fixed: one gate invocation; `.so` cdylibs pruned too)

Nearly every gate log of the last weeks reads `pruned 0.8 GB of superseded dependency archives` and then
`Compiling turso_sdk_kit … turso_sync_sdk_kit … turso`, even with no dependency change. `target/debug/deps` shows
the reason. Each gate builds TWO families of each kit, one about 60 MB and one about 110 MB as `.so`, one to two
minutes apart. The kits are built as several crate types (`.a`, `.so` and `.rlib`), and the prune keeps only the
newest `.a`/`.rlib` over 100 MB per stem. So a live family is deleted at every gate start and rebuilt. Separately,
the `.so` cdylibs (58–116 MB each) match no prune pattern, so every variant stays: ten were on disk on 09-29.

- **What the fingerprints say** (`target/debug/.fingerprint/turso_sdk_kit-*/lib-turso_sdk_kit.json`, read
  2026-09-29): every variant has the same features (`default, encryption, fts, pure-rust-crypto`). Two profile hashes
  appear:
  - `7635…`, which is `[profile.dev] debug = "line-tables-only"`, the RAW `cargo test` build;
  - `8378…`, the gate's `CARGO_PROFILE_*_DEBUG=0`.
  Gate-built variants of one profile still carry different metadata hashes from step to step, which points at
  feature unification. `cargo test -p store` resolves the dependency graph on its own, while `cargo test -p
  tender-db --features server` unifies a larger one, so the kit's dependencies differ and so does its hash.
- **Confirmed** with `cargo tree -e normal --format '{p} {f}'` per gate step (no build needed): 41 of the 291
  crates under `turso_sdk_kit` resolve DIFFERENT features for `-p store`, `-p ingest` and `-p tender-db --features
  server`. Examples: `futures-core` `alloc,std` versus `alloc,default,std`; `futures-util` gains
  `channel,io,sink` in the app step; `getrandom` 0.3 gains `wasm_js` in the app step. So each step builds its own kit
  family, and so do store and ingest themselves, since the app step rebuilds them under its own resolution.
  `resolver.feature-unification = "workspace"` would fix it at the source, but cargo 1.94 ignores it without
  `-Zfeature-unification`.
- **Fix (2026-09-29): the gate runs ONE invocation**, `cargo test -p model -p store -p ingest -p tender-db --features
  tender-db/server`, which resolves once. The app's integration suites still run: plain `cargo test` covers lib,
  bins, `tests/` and doctests. There are no live doctests anywhere: model, store and ingest run 0, and the app has 1,
  ignored.
- **The transition needs `cargo clean`.** A first try built the unified family BESIDE the three old ones and filled the
  disk: 35 MB free, exit 101. The 830 files (7.65 GB) it had written were removed, and the tree was cleaned (18.1 GiB)
  before the first unified gate, as this file already prescribes after a re-hash.
- **First unified gate, from clean (2026-09-29 ~16:0x UTC): GATE-EXIT=0, 140 suites (139 plus the app's doctest
  suite), 867 s.** The four-invocation gate took about 35 minutes from clean and 757–1,019 s incrementally today. The
  scratch sweep took 946 files (0.61 GB), and the build directory holds one family. Then either unify the two families, so one
  build serves both steps, or keep the newest per (stem, crate type, family) instead of per stem.
- A fix is worth one to two minutes of every gate, plus about 1 GB of `.so` that nothing removes today.

- **`.so` half fixed (2026-09-29 evening).** The prune also takes `.so` cdylibs from 50 MB up, keeping the newest per
  (stem, extension). A kit's `.a` and `.so` from ONE build share a hash, so a stem-only key would have deleted one
  of a live pair. A dry run over the unified family lists each kit's `.a` and `.so` as their own singleton groups,
  with 0 to prune. The next gate is green (GATE-EXIT=0) and pruned no dependency archive.
