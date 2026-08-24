# 260 — superseded test binaries fill the build disk, and the third symptom does not look like a disk problem

Status: FIXED 2026-08-20 (the prune; `ops/check.sh` now runs it before building). Filed anyway because
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
