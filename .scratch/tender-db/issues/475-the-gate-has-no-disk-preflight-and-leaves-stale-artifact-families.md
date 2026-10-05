# 475 — the gate has no disk preflight and leaves stale artifact families

Status: ready-for-agent — unit 1 (the free-space preflight) landed 2026-10-05 and was revised the same day after review, uncommitted in the worktree (`ops/gate-disk.sh`, `ops/test-gate-disk.sh`, `ops/check.sh`; see "Unit 1 — landed" and "Unit 1 — review"). It also carries unit 2's inputs hash and its record (`target/.gate-inputs`), used to credit a reusable family. Next step: unit 2's remainder (`cargo clean` on a re-hash), then 3–5. NOTE: this container reads 11.7 GiB free with no `target/.gate-inputs` yet, so the next `ops/check.sh` here refuses; run it once with `GATE_DISK_NEED_BYTES=3221225472` (target/ holds the current family: 14G, no input changed since) or `cargo clean` first. That green run records the hash, and later gates need ~3 GiB until an input changes.
Kind: operational / build hygiene (the gate's disk)
Relates to: 260 (reads closed; this is its open remainder), 254 (why `ops/check.sh` exists), 425 (the turso
`[patch]` whose re-hash filled the disk on 2026-09-27), 457 (the turso 0.8.1 bump, the next re-hash), 459
(`ops/gate-marker.sh`, the sourced-and-pinned shape this fix copies)

## What is wrong

**260 reads closed.** Its line 3 is `Status: FIXED-BUT-RECURRING`. `ops/board-verify.sh:13` treats only
needs-triage, needs-info, ready-for-agent, REOPENED, open, BACKLOG, PARKED and DORMANT as open, so the board
counts 260 as closed. It has no `## Verify` block. Lines 4–5 are a leftover fragment of an older Status ("runs
in one session filled the allowance anyway, …"). The Status names the 2026-09-18 archive rule but none of the
three 2026-09-29 fixes in its body (the `/tmp` sweep, the one-invocation gate, the `.so` prune).

**The fills on record** (CLAUDE.md:13–20 and :28–30, 260's body):

| date | cause | through the gate |
| --- | --- | --- |
| 2026-08-20 (three times), 08-23 | raw `cargo test` | no |
| 2026-09-10 | four gates in one session, rlibs rebuilt | yes: GATE-EXIT=101, no FAILED line |
| 2026-09-27 | `[patch.crates-io] turso` (425) re-hashed turso and everything above it; ingest died after store's suites passed | yes: GATE-EXIT=101, no FAILED line |
| 2026-09-29 | the one-invocation gate's first try built a new family beside three old ones | yes: 35 MB free, exit 101 |

The 2026-09-29 fixes removed three causes. Four gaps remain in `ops/check.sh` (186 lines, read 2026-10-01).

1. **No free-space check.** No script under `ops/` reads free space except the prod watchdog
   `ops/watchdogs/tender-db-diskwatch.sh`. A run that cannot fit starts anyway and dies mid-link. Its signature
   is GATE-EXIT=101 with no `test result: FAILED` line, or `ld … signal 7 [Bus error]` (260's third face).

2. **A re-hash is invisible until it has happened.** The prune (`prune_stale_test_binaries`, check.sh:32–91)
   runs once, before cargo, and keeps the newest file per stem. When the run itself re-hashes (a change to
   Cargo.lock, `[patch]`, a profile, features or the toolchain), the new family does not exist yet when the
   prune runs. The build writes it beside the old family, so the peak is two families. Nothing compares the hash
   inputs with the last run's. The remedy is a manual line in CLAUDE.md:19–20 ("After a dependency/patch change,
   `cargo clean` first") and step 5 of 457's bump plan. Its "~35 min from clean" predates the one-invocation
   gate: 260 records the first unified gate from clean at 867 s, and 458's incremental gate took 823 s.
   Read 2026-10-01 13:10 UTC in this container: one family is **12.0 GiB** (`target/debug/deps`,
   12,840,753,202 bytes) and **16.3 GiB is free** (`df`). A second family plus a run's `/tmp` scratch (~0.6 GB,
   260) leaves about 3.7 GiB. 457's bump plans "a separate worktree", which means a second `target/` on the same
   free space.

3. **The prune cannot tell a sibling from a superseded variant.** The test-executable half (check.sh:38–58)
   groups `^(.+)-[0-9a-f]{16}$` executables over 20 MB by stem alone and keeps the newest. Several live targets
   of ONE build share a stem: an ingest bin and the ingest integration test of the same name, and the app's
   lib-test, bin-test and bin. Cargo's own unit names in `target/debug/.fingerprint/<pkg>-<hash>/` tell them
   apart, for example `ingest-0ce89eed5b484007/bin-data-quality` and
   `ingest-6b18e854dacd3cf3/test-integration-test-data_quality`. A read-only dry run of that grouping over this
   container's `target/debug/deps` (2026-10-01 13:0x UTC, every file from the single build at 13:03–13:06):

   | stem | the next gate's start deletes | keeps |
   | --- | --- | --- |
   | `data_quality` | bin `src/bin/data-quality.rs` (20 MiB) | test `tests/data_quality.rs` |
   | `fetch` | bin `src/bin/fetch.rs` (82 MiB) | test `tests/fetch.rs` |
   | `process` | test `tests/process.rs` (102 MiB) | bin `src/bin/process.rs` |
   | `project` | bin `src/bin/project.rs` (74 MiB) | test `tests/project.rs` |
   | `tender_db` | lib test (139 MiB) and bin test (77 MiB) | bin |

   That is 493 MiB of live executables. Cargo finds each output missing and rebuilds the unit, on every gate. It
   is the 2026-09-29 kit churn again (260). That fix keyed the archive prune on (stem, extension) but left the
   executable prune keyed on stem, and 260's entry already named the fix: key per (stem, crate type, family),
   not per stem.

4. **17 test binaries run nothing.** 17 integration-test files hold only `#[ignore]`d probes, 27 tests in all:
   15 in `crates/store/tests` (`alter_add_column_cost`, `country_filter_probe`, `exists_shortcircuit_probe`,
   `lot_prod_profile_probe`, `lot_summary_cost`, `lots_drive_probe`, `name_prefix_probe`, `org_cursor_probe`,
   `org_index_bulk_load`, `org_index_read_amplification`, `paginated_index_probe`, `plan_bulk_load`,
   `reveal_cursor_probe`, `scan_budget_probe`, `stripes_probe`) and 2 in `crates/ingest/tests`
   (`project_incremental_timing`, `project_memory`). The owner's note said 16. Counting files whose every
   `#[test]`/`#[tokio::test]` carries `#[ignore]` gives 17. Each binary links the whole dependency tree, 68–76 MiB
   apiece and 1,177 MiB together, about a tenth of the family. Every gate links all of them and runs none.

## Proposed fix

The root cause is that the gate decides what to delete by guessing from file names and mtimes, once, before it
knows what the run will build, and it never asks whether the run will fit. Replace the guess with cargo's own
record and a measured check. Move the disk logic into a sourced `ops/gate-disk.sh` (the `gate-marker.sh` shape)
so that a test can drive the real functions.

1. **Preflight (first unit).** Before cargo starts, read the free bytes where `target/` and `/tmp` live
   (`os.statvfs`). Refuse to start when free space is below the family size the last green run recorded
   (unit 3) plus 1 GiB of `/tmp` scratch. With nothing recorded, the floor is 13 GiB (one family is 12.0 GiB
   today). The refusal prints the free space, the space needed and `du -sh target`, plus the remedy: `cargo clean`,
   then run again (867 s from clean). It exits non-zero before cargo runs, with a line that cannot be mistaken
   for a test failure (`==> GATE REFUSED: disk …`). This one check catches every fill mode, including the ones
   units 2–3 miss: a raw `cargo test` family, or a second worktree's `target/`.
2. **See a re-hash before building.** Hash the inputs that set every crate's metadata hash: Cargo.lock, every
   workspace `Cargo.toml` (features, `[patch]`, `[profile]`), `.cargo/config.toml`, `rustc -vV` and the gate's
   `CARGO_*` environment. Record the hash with each green run (`target/.gate-inputs`). If it differs at the
   start, the old family is superseded: run `cargo clean` before the build and print which input changed. The
   gate then does CLAUDE.md:19–20's manual step itself, and 457's bump inherits it. A false trigger (a
   `Cargo.toml` edit that changes no hash) costs one gate from clean, which is about as long as an incremental
   one (867 s against 823 s).
3. **Prune by cargo's list, not by name.** Build first with `cargo test --no-run
   --message-format=json-render-diagnostics` and the gate's exact arguments, with stdout sent to a file. From its
   `compiler-artifact` messages, which cargo emits for fresh units too, collect the 16-hex hash of every
   `filenames` and `executable` entry. Then delete each `target/debug/deps/*-<hash>*` file whose hash is not in
   that set and whose mtime is older than the gate's start stamp (files from a concurrent build are newer and
   stay). The real `cargo test` with the same arguments then has nothing to compile. Record the live set's size
   for unit 1. This replaces both heuristic prunes (stem/mtime/20 MB, and stem+extension/100 MB/50 MB) and ends
   the 493 MiB sibling churn.
4. **One probe binary per crate.** Move the 17 files under `crates/store/tests/probes/` and
   `crates/ingest/tests/probes/`, each with a `main.rs` that declares them as modules. Cargo discovers
   `tests/probes/main.rs` as one target, `probes`. The probes still compile in every gate, so they cannot rot,
   but they link twice instead of 17 times, about 1 GiB less per family. Rejected: `required-features = ["probes"]`
   per `[[test]]`. It saves the compile as well, but a probe the gate never compiles rots without anyone
   noticing. The six files whose doc comment says `--test <name>` change to `--test probes <name>::`.
5. **Close the books.** Rewrite 260's Status line to point here and drop the fragment on its lines 4–5. Once
   unit 2 lands, remove the manual clean from CLAUDE.md:19–20 and 457 step 5.

**The test that pins it:** `ops/test-gate-disk.sh`. It runs offline, and `check.sh` runs it before cargo, the
way `deploy.sh` runs `test-gate-marker.sh`. Every case is checked both ways:
- the preflight refuses one byte under the threshold, passes at it, and the refusal prints the remedy;
- the inputs hash is stable across two reads and changes when one byte of a Cargo.lock copy changes;
- the prune, run over a fake `deps/`, keeps a live bin/test pair that shares a stem (the `data_quality` shape),
  deletes a hash the list does not name, and keeps an unnamed file newer than the start stamp.

## Unit 1 — landed (2026-10-05)

Uncommitted in the worktree; not gated through cargo (another agent's gate may need the disk), pinned offline.

- `ops/gate-disk.sh` (new, sourced — the `gate-marker.sh` shape): `gate_disk_preflight [target] [tmp]` reads
  `df -Pk` for target/'s filesystem (its parent when target/ is absent, i.e. after `cargo clean`) and /tmp's.
  Threshold: `GATE_DISK_NEED_BYTES` if set; else `target/.gate-family-bytes` (unit 3 will write it) + 1 GiB
  scratch; else 12 GiB family (the 2026-10-01 measurement) + 1 GiB = **13 GiB**. `GATE_DISK_FAMILY_BYTES` and
  `GATE_DISK_SCRATCH_BYTES` override the parts. One filesystem: the whole threshold must fit there; two:
  target/'s needs the family, /tmp's the scratch. Fails closed: df erroring, printing nothing, or a
  non-numeric Available refuses; so does a non-numeric override. A garbage recorded size falls back to the
  floor, never to zero. The refusal's first line is `==> GATE REFUSED: disk — <free> on <mount>, needs <n>;
  cargo was NOT started`, then the threshold's source, `du -sh target`, and the remedy (`cargo clean`, ~867 s
  from clean; a second worktree's target/; stale `/tmp/tender-db-*`; the one-run override).
- `ops/check.sh`: after the prune and the two-hour /tmp sweep, before `gate_begin` and cargo, runs
  `ops/test-gate-disk.sh` (a failing self-test refuses the gate) and then `gate_disk_preflight target /tmp ||
  exit 3`. Exit 3 is not cargo's 101, and no `test result:` line is printed, so GATE-EXIT reads red and the
  cause is the GATE REFUSED line.
- `ops/test-gate-disk.sh` (new, offline, stub `df`/`du` on PATH, planned-count + EXIT-trap harness like
  `test-gate-marker.sh`): 27 cases, each proceed/refuse pair one KiB (df's unit) apart — default floor,
  recorded family, env override (and garbage overrides), two filesystems short on each side, target/
  absent, df failing / garbage / empty, the 2026-10-05 reading, and that check.sh calls the preflight before
  `cargo test` with `|| exit N`. `all 27 cases passed`. Mutants checked: a preflight hard-wired to "fits"
  fails 21 cases; df-failure treated as "fits" fails 3; check.sh's `|| exit 3` turned into `|| true` fails 1.
- Real read, this container, 2026-10-05: `11.7 GiB free on / (target/ and /tmp), needs 13.0 GiB`, target/
  holds 14G — **the next gate here refuses** until `cargo clean`.
- Verify line's first number read `1` (`df -P`) at landing; after the review it reads `2` (see below).
- Deviation from the plan above: `df -Pk`, not `os.statvfs` — the task asked for a stub-able `df`, and the
  Verify grep accepts either. The hash and prune cases of `ops/test-gate-disk.sh` arrive with units 2–3.

## Unit 1 — review (2026-10-05)

Seven findings; outcomes:

1. **high, fixed — the floor refused every gate after a green one.** Allowance here with target/ empty ≈ 24.8 GiB
   (11.7 free + 13.1 in target/); a from-clean gate leaves ~11.7, under 13. The preflight now credits a reusable
   family: `gate_disk_inputs_hash` (sha256 over Cargo.lock, every Cargo.toml outside target/.git/.scratch,
   .cargo/config(.toml), rust-toolchain(.toml), `rustc -vV`, the `CARGO_*`/`RUST*` env; 0.12 s) is computed at
   the start; check.sh records it in `target/.gate-inputs` (temp + rename) only after cargo is green. When it
   matches and `target/debug/deps` exists, the need is `GATE_DISK_RELINK_BYTES` (default 2 GiB, an estimate:
   the prune's ~0.5 GiB of siblings relinked every gate plus parallel link outputs) + 1 GiB scratch = 3 GiB.
   Any mismatch, a missing record, no deps/, or a hash that cannot be computed (no rustc) gives no credit:
   13 GiB as before, and the refusal says `.gate-inputs differs` on a mismatch. This is unit 2's hash and
   record, without its `cargo clean`. The first gate here still refuses (nothing recorded yet) — see Status.
2. **medium, fixed — overflow failed open.** `_gate_disk_int` accepts at most 15 digits (999 TB) after
   stripping leading zeros, so no sum can wrap; a belt-and-braces `need <= 0 || need < family` refuses. A
   recorded family under 1 GiB is not believed and falls back to the floor. A 22-digit record now refuses at
   12.9 GiB ("needs 13.0 GiB"); `GATE_DISK_SCRATCH_BYTES=2^63-1` refuses even with 50 GiB free.
3. **low, fixed — leading zeros.** Zeros are stripped before arithmetic (`08` is 8, then under 1 GiB → floor,
   with the GATE REFUSED line); all-zero values (`00`) are refused; df's Available is read with `10#`.
4. **low, fixed — mount-string comparison.** `_gate_disk_free` also returns df's device column; one pool when
   the device OR the mount matches, and then the whole need must fit in the smaller reading. Two separate
   filesystems df names alike (two tmpfs) are treated as one — the stricter answer.
5. **low, fixed — hard-coded paths.** check.sh calls `gate_disk_preflight "${CARGO_TARGET_DIR:-target}"
   "${TMPDIR:-/tmp}"` and records into the same target; the function's defaults are the same. (The prune at
   check.sh's top still reads `target/debug/deps`; units 2–3 replace it.)
6. **low, fixed — test gaps.** `ops/test-gate-disk.sh` is now 67 cases (`all 67 cases passed`, rc 0): the
   22-digit record, `08`, a zero-padded 12 GiB record, `GATE_DISK_NEED_BYTES` `00`/22 digits/zero-padded,
   scratch 2^63−1, family `00`, a zero-padded df Available, one device under two mount points (both ways),
   the inputs hash (stable; changes on a Cargo.lock byte, a member Cargo.toml, a `CARGO_*` var; fails without
   rustc), the credit both ways (3 GiB proceeds / 1 KiB under refuses; 11.7 GiB proceeds with a match and
   refuses after a Cargo.lock change; no credit without deps/ or rustc), and the record replacing rather than
   appending. The line-order grep is gone: a sandboxed copy of check.sh runs with stub cargo/python3 (and a
   find that answers nothing for /tmp) — short disk exits 3 without cargo and with the shortfall line (not a
   df failure); a failing self-test exits 3 without cargo; room → cargo runs, red keeps cargo's exit and
   records nothing, green records `.gate-inputs`, and the next gate at 11.7 GiB proceeds on the credit;
   `CARGO_TARGET_DIR` / `TMPDIR` on a full filesystem refuse and name it. Mutants: no digit cap 2 FAIL,
   mount-only compare 1, no credit 4, credit without the hash 3, `|| true` 4, hard-coded paths 7, no
   record 2, no `10#` 1, no zero strip 4.
7. **info — no change needed.** GATE-EXIT/marker contract holds (exit 3 before the EXIT trap and gate_begin).

Real read after the fix (read-only, this container): `11.7 GiB free on / (target/ and /tmp, one device:
/dev/vda), needs 13.0 GiB`, threshold "one build family" — no record yet. Verify's first number now reads `2`
(`df -P`, and `Cargo.lock` from the inputs hash); unit 3 adds `message-format`.

## Verify

    cat ops/check.sh ops/gate-*.sh | grep -oE 'statvfs|df -P|Cargo\.lock|message-format' | sort -u | wc -l; grep -l '#\[ignore' crates/*/tests/*.rs | wc -l; grep -c 'cargo clean` first' CLAUDE.md

The line reads units 1–3 (the gate's scripts read free space, hash Cargo.lock and read cargo's artifact list),
unit 4 (the probe files leave the top level of `tests/`) and unit 5, the last (CLAUDE.md:20's manual
`cargo clean` first is gone).

- **open** (2026-10-01): `0`, `18` and `1`. No gate script does any of the three, 18 top-level test files carry
  an `#[ignore]` (the 17 probe files plus `crates/store/tests/lots_filter_fixture.rs`, which has 3 live tests
  beside its 1 ignored), and CLAUDE.md:20 still prescribes the manual clean.
- **done**: `3` or more, `1` (only `lots_filter_fixture.rs`) and `0`.
