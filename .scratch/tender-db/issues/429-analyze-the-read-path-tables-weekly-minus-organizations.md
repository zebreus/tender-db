# 429 — ANALYZE the read-path tables weekly (minus `organizations`), as issue 428 decided

Status: ready-for-agent — steps 1, 2 and 5 BUILT 2026-09-29 (below): `store::ANALYZE_TABLES`, the `analyze` job, and the plan test. Deploys unscheduled. Step 3 (the weekly schedule) and step 6 wait on step 0's capture diff, which needs the 2026-10-04 Sunday jobs and that day's snapshot.
Was status (before 2026-09-29): ready-for-agent — filed 2026-09-27 01:00 UTC from issue 428's measurement. Mine; the decision is taken
(428 § Decision), this is the build. Step 0's capture is LIVE on prod since 2026-09-27 03:12 UTC (`d7657a3`, drop-in
`/etc/systemd/system/tender-db.service.d/plancapture.conf` → `/data/tmp/plan-capture-429.sql`; 83 statements in the
first minute). The first deploy (`bba7804`, 02:44) recorded nothing — dioxus's logger took tracing's one global
subscriber first; fixed by installing before `serve`. The diff waits for a week of capture (a weekday tick and the
2026-10-04 Sunday jobs) and the 2026-10-04 snapshot; remove the drop-in after.
Kind: query planning / operations
Relates to: 428 (the evidence — read it first), 421 (the trap class this fixes), 256 (`ANALYZE plan_*`, the
existing precedent), 112 / 114 / 122 (plan gates whose precondition is "no stats"), 62 / 111 (deferred indexes —
why the resolver cannot take an `INDEXED BY`)

## What to build

0. **FIRST — the statements 428 did not see.** 428's capture was the app suites, which never run the
   supervisor's weekly jobs (data-quality, censuses, org merges). Build `tender_db::plan_capture` (the suites'
   hook, promoted into the server behind `TENDER_PLAN_CAPTURE=<file>`, capped at 20,000 distinct statements),
   deploy it, enable it with a drop-in for one full week so a Sunday tick and a weekday tick both run under it,
   then plan every captured statement on two copies of the next weekly snapshot (base vs the chosen subset) with
   `plan-probe`, time what differs, and only then schedule the job. If a job's statement regresses, it gets the
   same treatment `organizations` got (exclude the table) or a statement fix, decided from the numbers.
1. **The set, as a constant with its reasons.** `ANALYZE_TABLES` in the store: `tenders lots tender_versions
   organization_names organization_mentions tender_version_dates tender_version_classifications
   tender_version_parties tender_version_texts tender_version_amounts tender_version_lots tender_version_bids
   tender_version_bid_parties tender_version_contracts tender_version_lot_results tender_version_result_winners
   tender_version_result_stats tender_version_lot_group_members lot_results bids contracts currency_rates
   quarantine notices` (24). The doc comment names the three exclusions and the measured reason for each
   (`organizations`: the resolver's name lookup 0.001 → 6.19 s cold on a skewed name; the raw `notice_*` layer
   and `changes`: 1 h 44 min for no measured plan benefit; the small operational tables: seeks become scans).
2. **A supervisor job `analyze`**: one `ANALYZE <table>` per table, each its own transaction (so a stop costs at
   most one table, and the writer is released between tables — the longest is `tender_version_parties`, 242 s on
   prod data). Records per-table seconds in the job's counts line. `Box::pin` the arm (CLAUDE.md: `run_spec` is a
   62-arm future). It must never touch a table outside the constant — and it DELETEs any `sqlite_stat1` row for
   `organizations` it finds, so a stray manual `ANALYZE` cannot re-introduce the regression.
3. **Schedule**: weekly in the quiet window, after the Sunday data-quality run and clear of the snapshot job
   (issue 420's overlap rule) — the same mechanism the data-quality run uses. About 26 min of writer time.
4. **Stats reach the running readers — MEASURED 2026-09-27, they do not on their own.**
   `crates/store/tests/analyze_stats_pickup.rs`: on one `turso::Database`, a reader opened before an ANALYZE
   keeps planning with the statistics it loaded at open (`SEARCH t USING INDEX t_k`), while the analyzing
   connection and any new connection plan with the fresh ones (`SCAN u` + PK seek) — the server would run two
   plans for one query until its next restart (and a revert, `DELETE FROM sqlite_stat1`, would likewise wait for
   one). A throwaway schema change (`CREATE TABLE …; DROP TABLE …`) makes the same reader reparse at its next
   statement and pick the new statistics up; the test pins both halves. So the job ENDS with that bump.
5. **A plan test that pins the decision** (store): a scratch DB with a skewed `name_norm` distribution, the
   job's ANALYZE run on it, then assert (a) no `sqlite_stat1` row for `organizations`; (b) the resolver
   statement still seeks `organizations_name_country`; (c) the 421 plain-JOIN shape drives from `tenders`.
   Assert on the artifact (`EXPLAIN QUERY PLAN` of the builder's own statement text), issue 114's rule.
6. **The gates**: 112/114/122's "stats must be absent" precondition becomes "stats only for `ANALYZE_TABLES`";
   the prod-box-reads trap table's 421 row gains "planned correctly once 429's job has run — CROSS JOIN stays the
   robust form"; the 417 row stays as is (stats do not fix it, 428 §3).

## Verify

    ssh -o BatchMode=yes root@zebreus.click "/root/aj.sh '/admin/jobs?limit=200'" | python3 -c "import sys,json; r=[j for j in json.load(sys.stdin)['recent'] if j['kind']=='analyze']; print(*([r[0]['job_id'], r[0]['outcome'], (r[0]['counts'] or '')[:200]] if r else ['no analyze job yet']))"

- **done**: `<job> ok analyze (issue 429): 24 of 24 tables in …; organizations statistics rows removed: …` — the
  job ran on prod, every table, no stop (step 3's first scheduled run)
- **open**: `no analyze job yet` (read 2026-09-29 13:5x UTC: built, deployed, not scheduled). Changed 2026-09-29: the
  old verify read `sqlite_stat1` through `/v1/sql`, which refuses it (not in the public surface), so it could never
  print its done line.

## Steps 1, 2 and 5 — BUILT 2026-09-29 (unscheduled)

- **Step 1**: `crates/store/src/analyze.rs` holds `ANALYZE_TABLES` (the 24), and its doc comment gives the three
  exclusions with 428's numbers. `Db::analyze_table` refuses any table outside the list. `Db::finish_analyze` deletes
  every `sqlite_stat1` row for `organizations`, one per index (a stray `ANALYZE organizations` on the scratch schema
  wrote 6, not 1). It then runs the `CREATE`/`DROP TABLE` bump, so pooled readers re-plan.
- **Step 2**: the job kind `analyze` (`Spec::Analyze`, boxed arm `run_analyze`) runs one statement per table, with
  the writer released between tables and a phase line naming the table in flight. It is stoppable between tables
  (`STOPPABLE_KINDS`), and it runs `finish_analyze` even when stopped. The summary lists every table's seconds and
  the number of organizations rows removed. Test: `the_analyze_job_covers_its_tables_and_strips_organizations_statistics`.
- **Step 5**: `crates/store/tests/analyze_job.rs` seeds 3,000 orgs (half named `stadt`) and 10,000 Tenders with
  three classifications each, plants stray organizations statistics, runs the job's two calls and asserts:
  (a) no `organizations` row survives and every analyzed table is in the list;
  (b) the resolver's reuse lookup still seeks `organizations_name_country`;
  (c) 421's plain JOIN drives from `tenders` and seeks the version table by `(tender_id, seq)`.
  (c) discriminates: before the ANALYZE, the same statement drove from
  `tender_version_classifications_code (scheme=?)`, 421's trap. Refusal of `organizations` and `notice_texts` is
  asserted too.
- `docs/operations.md` carries the job's row and marks it unscheduled.

Next: deploy it with the next bundle (running it by hand is allowed but not planned). After 2026-10-04, diff step 0's
capture on two copies of that day's snapshot, then schedule it (step 3) and update the gates (step 6).

