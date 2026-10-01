# 467 — `run_spec` is one future the size of every arm, so each stack overflow gets fixed one arm at a time and nothing measures it

Status: ready-for-agent — filed 2026-10-01 from the owner's board survey (workflow wf_4eac8781-4d0, verified by an adversarial pass). The first unit is the tripwire: a test that checks the `size_of_val` of `run_spec`'s future, and of each future it boxes, against a named budget. Growth then fails in the gate under its own name, not as a SIGABRT in an unrelated test.
Kind: risk (test-stack overflow, gate reliability)
Relates to: 464 (owns CLAUDE.md's stale "62-arm" note and the doc comment that `run_spec` lacks), 404 (the stack lesson,
404:815–829), 432 and 388 (both hit the overflow again), 342 (`run_fetch_fts`), 434 (`run_project`)

## What is wrong

Read 2026-10-01 at `3994975`:

- `crates/app/src/supervisor.rs:3893–10636`: `run_spec` is one `match &job.spec`, 6,744 lines long, with one arm per
  `Spec` variant. That is 84 arms for the 84 variants of `enum Spec` (:224–687). An async fn compiles to one state
  machine. An unboxed arm's locals and the future it awaits belong to that one type, so the future is as large as its
  largest arm.
- 51 of the 84 arms contain no `Box::pin`. The largest are `OrgMergeHealth` (406 lines, :4547), `ScanOrgMatchKeys`
  (275, :9591) and `R2Census` (202, :4953). The file has 45 `Box::pin` call sites, 40 of them inside `run_spec`.
- Nothing measures the size. No test calls `size_of_val` on a future. Neither `ops/check.sh` nor `.cargo/config.toml`
  sets `RUST_MIN_STACK`. The only figures are two estimates from 2026-09-07, never re-taken: a "debug-build poll frame
  ~490 KB, a quarter of a test thread's stack" (:3234), and "24 KB from the stack limit" (342:133).

**Where it fails: in tests.** `#[tokio::test]` (tokio-macros 2.7.1, `entry.rs:526–547`) pins the test body on the test
thread's stack, and 17 tests await `sup.run_spec(...)` inline, at 49 call sites. The test that fails is always
`an_execute_without_an_expected_count_is_refused` (:15361), a short test of the `MarkSkippedSiblings` refusal. It aborts
with `stack overflow` (SIGABRT) and does not name the arm that grew.

**The history: six overflows since 2026-09-01, each fixed where it surfaced.** The commits were read through the GitHub
API on 2026-10-01, because the local clone is shallow (61 commits):

| date | commit | what grew | local fix |
|---|---|---|---|
| 09-01 | `27386b5` | issue 334's census arm | boxed the five census arms and wrote CLAUDE.md's note |
| 09-07 | `3560157` | 342's FTS fetch, boxed inline in its arm | moved it into `run_fetch_fts` and sibling methods that box inside (:3233–3239) |
| 09-17 | `b6750d5` | 404's wet `repair-member-twins` | the arm was already boxed and still overflowed; boxed the deep awaits in the supervisor and the store, after "four wrong guesses" (404:815–829) |
| 09-18 | `79c1bef` | the `Reindex` builders (388's exact-count cap) | boxed the arm's body (388:315–319) |
| 09-27 | `0797820` | 432's `repair-provisional-name-norm` | the arm's own `Box::pin` was not enough; boxed the store future inside it (:8730–8736, 432:113–114) |
| 09-27 | `94d2346` | the fold (434's mention refresh) | moved the arm into `run_project` behind `Box::pin` (:3655, :4060–4064) |

Three more commits box their arm in advance: `75db3bf` (355) and `b434845` (448), whose messages cite the note, and
`af8d192` (395).
In all, 31 commits to `supervisor.rs` since 2026-09-01 add a `Box::pin` line.

CLAUDE.md's remedy, "wrap a big arm's body in `Box::pin`", is not enough, and `b6750d5` and `0797820` show why.
`Box::pin` builds the future on the stack before it moves it to the heap, so a large boxed future still overflows.
That means the measure has to cover every future `run_spec` builds, not only `run_spec`'s own. Fixing the note's
wording belongs to 464.

**Production: no overflow seen.** Jobs run through `Handle::spawn` on the `job-exec` runtime (:3161), which puts the
task on the heap. That runtime's threads keep tokio's default 2 MiB stack, because :76–93 sets none. The server is an
optimized `server-release` build (`nix/package.nix:65–66`). The prod journal since 2026-09-01 00:20 has no
`stack overflow` line (read 2026-10-01). The margin there is also unmeasured.

## Proposed fix

The root cause is that one future spans 84 arms, so any arm's growth spends every test's stack. The fix has two parts:
a gauge, so that growth is caught where it happens, and a split that removes the coupling.

1. **Tripwire (first unit).** Add `run_spec_futures_stay_inside_their_size_budgets` to `supervisor.rs`'s tests.
   - It builds `sup.run_spec(&job(..))` without polling it and asserts `std::mem::size_of_val` against a named const
     budget. One async fn has one future type, so any `Spec` gives the same number.
   - It checks each future that `run_spec` boxes at its call site in the same way. Today there are ten:
     `run_fetch`, `run_fetch_fts`, `run_rehash_probe`, `run_probe_fts`, `run_project`, `run_sweep_orphan_orgs`,
     `run_analyze`, `run_repair_member_twins`, `refuse_without_org_fk_indexes` (seven sites, :5309–8717) and the
     store's `repair_provisional_name_norm` (:8736). Each of these is also built on the stack before `Box::pin` moves it.
   - The assertion message names the future, its size, its budget and the remedy.
   - Take the first measurement in the gate's profile (`ops/check.sh`) and record it here. Set each budget at that
     number plus a small headroom, so that a growing arm fails, by name, at the commit that grew it.
   - Keep `an_execute_without_an_expected_count_is_refused` as the backstop.
2. **Split the match.** Move the arm bodies out of `run_spec` into async fns grouped by family, each boxed at its call
   site. `run_project` already has this shape (:4064).
   - Group by what the arms share, for example: ingest and fetch, backfills, censuses and packets, org identity and
     merges, repairs, and projection and maintenance. An arm that shares nothing gets its own fn.
   - `run_spec` becomes a dispatch with one short arm per variant. Each family's future is then only as large as that
     family's largest arm. The tripwire lists every family fn with its own budget.
   - Keep `enum Spec` and its serde shape exactly as they are. Job rows store it externally tagged (:1182), and on
     recovery a row the running build cannot parse is dropped (:2885–2900). Nesting the variants into per-family enums
     would silently drop queued jobs at a deploy.
3. **The rule for the next arm.** Once the split lands, the rule becomes: add the arm as its own fn behind `Box::pin`,
   and the tripwire names any budget it breaks. Hand that wording to 464, which owns CLAUDE.md's note.

## Verify

    awk '/async fn run_spec\(/{s=NR} s&&/^    }$/{print NR-s+1; exit}' crates/app/src/supervisor.rs; grep -rl 'fn run_spec_futures_stay_inside_their_size_budgets' crates/app/src | wc -l

- **open** (2026-10-01, at `3994975`): `6744` and `0`. `run_spec` is the whole match, and no tripwire exists.
- **after the first unit:** `6744` and `1`.
- **done**: a number under `400` (84 dispatch arms of one to three lines each) and `1`. If the split moves `run_spec`
  to another file, point the awk at that file.
