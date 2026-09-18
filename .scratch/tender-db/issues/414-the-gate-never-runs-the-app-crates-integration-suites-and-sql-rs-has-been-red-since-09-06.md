# 414 — `ops/check.sh` never runs the app crate's five integration suites, and `tests/sql.rs` has been red since 2026-09-06 without a single gate noticing

Status: ready-for-agent — found 2026-09-18 06:4xZ while adding a `/v1/sql/schema` assertion (issue 50): two tests in `crates/app/tests/sql.rs` fail on the committed tree, and the last green gate's log contains no `Running tests/sql.rs` line — nor `api.rs`, `admin.rs`, `accounts.rs`, `webhooks.rs`.
Kind: operational (the test gate — `ops/check.sh`, `.cargo/config.toml`'s `test-app` alias) plus two test-side defects in `crates/app/tests/sql.rs`
Relates to: 260 (the gate script and its pruning — the script this is about), 254 (the pipeline-exit-code trap the script was written against; this is its sibling: a gate that reports green over suites it never opened), 239 (the `v_tender_current` refusal that turned the sql suite red on 2026-09-06), 412 (a status nobody re-reads is a photograph — a suite nobody runs is the same thing), 117/390 (their API tests live in `api.rs`, which the gate does not run)
Blocked by: nothing

## Observed

`ops/check.sh` runs, in order, `test -p model`, `test -p store`, `test -p ingest`, `test-app`. The
alias (`.cargo/config.toml`):

    test-app = "test -p tender-db --features server --lib"

`--lib` builds and runs the unit tests inside `crates/app/src` and nothing else. The app crate's
integration tests — `crates/app/tests/{accounts,admin,api,sql,webhooks}.rs` — are never built by the
gate. The last green gate log (2026-09-18, 117 suites, for the 343 change) lists forty-odd
`Running tests/….rs` binaries, every one of them from `store` or `ingest`; none of the five.

Run by hand on the committed tree (`2df1a49`), `cargo test -p tender-db --features server --test sql`:

    test a_real_analytical_query_answers ... FAILED
    test the_analyst_views_answer ... FAILED
    test result: FAILED. 12 passed; 2 failed

The first fails with a `400` whose body is issue 239's refusal, verbatim: `v_tender_current is NOT
FILTERABLE and this query filters it … read tenders.current_seq directly`. The test's acceptance
query — `FROM v_tender_current c JOIN tender_version_parties p ON p.tender_id = c.tender_id AND
p.seq = c.seq …` — is exactly the shape 239 made illegal on 2026-09-06 (`26581c1`, `12bffae`,
`e1f270e`). The test was last touched on `b4a18a2`, before that. Twelve days red, three deploys a
day, and no gate opened the file.

## Why it matters

Every "gate green" verdict on an app-layer change since the alias was written has been a verdict
on the unit tests only. Issues 117 and 390's API contract tests (`api.rs`), the `/v1/sql`
allow-list and refusal tests (`sql.rs`), the account and webhook flows — none of it runs unless
someone runs it by hand, and the CLAUDE.md note that motivated the alias ("`cargo test -p
tender-db --lib` prints ok. 0 passed") was fixed by making the lib tests run, not by making the
rest run. It is the 254/260 class: a green that is not measuring what it says.

## Repro

    grep -c 'Running tests/sql.rs' <last gate log>        # 0
    cargo test -p tender-db --features server --test sql   # 2 failed

## Verify

    grep -n 'test-app\|--test' ops/check.sh .cargo/config.toml | grep -c -- '--tests\|--test '

- **done**: `1` or more — the gate's app step builds the integration tests (`--tests`, or each `--test <name>`), and `cargo test -p tender-db --features server --test sql` passes
- **open**: `0` — the app step is `--lib` alone (read 2026-09-18)

## Done when

- The two red tests are fixed on their own merits: the analytical query uses the documented join
  (`JOIN tender_versions v ON v.tender_id = t.id AND v.seq = t.current_seq`) rather than the
  refused pointer view, and `the_analyst_views_answer`'s failure is diagnosed (its body is in the
  audit file) and fixed.
- The remaining three suites are run by hand once and their state recorded here — how much else
  is red outside the gate is the number that decides whether this is a script fix or a campaign.
- `ops/check.sh` runs the app crate's integration tests. Disk is the cost (issue 260): five more
  test binaries per gate, which the prune step already handles by name.
- `.cargo/config.toml`'s comment stops calling `test-app` "all server-side tests".
