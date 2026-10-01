# 475 — the gate has no disk preflight and leaves stale artifact families

Status: ready-for-agent — filed 2026-10-01 from the owner's board survey (workflow wf_4eac8781-4d0, verified by an adversarial pass). The first unit is a free-space preflight in `ops/check.sh` that refuses to start cargo when one more build family would not fit, prints the remedy, and is pinned both ways by an offline shell test.
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

## Verify

    cat ops/check.sh ops/gate-*.sh | grep -oE 'statvfs|df -P|Cargo\.lock|message-format' | sort -u | wc -l; grep -l '#\[ignore' crates/*/tests/*.rs | wc -l; grep -c 'cargo clean` first' CLAUDE.md

The line reads units 1–3 (the gate's scripts read free space, hash Cargo.lock and read cargo's artifact list),
unit 4 (the probe files leave the top level of `tests/`) and unit 5, the last (CLAUDE.md:20's manual
`cargo clean` first is gone).

- **open** (2026-10-01): `0`, `18` and `1`. No gate script does any of the three, 18 top-level test files carry
  an `#[ignore]` (the 17 probe files plus `crates/store/tests/lots_filter_fixture.rs`, which has 3 live tests
  beside its 1 ignored), and CLAUDE.md:20 still prescribes the manual clean.
- **done**: `3` or more, `1` (only `lots_filter_fixture.rs`) and `0`.
