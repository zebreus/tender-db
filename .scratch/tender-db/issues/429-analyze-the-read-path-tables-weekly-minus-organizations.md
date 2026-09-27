# 429 — ANALYZE the read-path tables weekly (minus `organizations`), as issue 428 decided

Status: ready-for-agent — filed 2026-09-27 01:00 UTC from issue 428's measurement. Mine; the decision is taken
(428 § Decision), this is the build.
Kind: query planning / operations
Relates to: 428 (the evidence — read it first), 421 (the trap class this fixes), 256 (`ANALYZE plan_*`, the
existing precedent), 112 / 114 / 122 (plan gates whose precondition is "no stats"), 62 / 111 (deferred indexes —
why the resolver cannot take an `INDEXED BY`)

## What to build

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
4. **Stats reach the running readers.** turso loads `sqlite_stat1` at schema (re)parse (`connection.rs`
   `ReparsePhase::RefreshStats`; `stats.rs` `refresh_analyze_stats`). `sqlite_stat1` already exists (the three
   internal tables), so a later ANALYZE changes no schema and the pooled reader connections may keep the old
   stats until restart. MEASURE first (a two-connection test: ANALYZE on one, EXPLAIN on the other); then either
   rely on it, reopen the readers after the job, or state "effective at the next restart" in the job summary.
5. **A plan test that pins the decision** (store): a scratch DB with a skewed `name_norm` distribution, the
   job's ANALYZE run on it, then assert (a) no `sqlite_stat1` row for `organizations`; (b) the resolver
   statement still seeks `organizations_name_country`; (c) the 421 plain-JOIN shape drives from `tenders`.
   Assert on the artifact (`EXPLAIN QUERY PLAN` of the builder's own statement text), issue 114's rule.
6. **The gates**: 112/114/122's "stats must be absent" precondition becomes "stats only for `ANALYZE_TABLES`";
   the prod-box-reads trap table's 421 row gains "planned correctly once 429's job has run — CROSS JOIN stays the
   robust form"; the 417 row stays as is (stats do not fix it, 428 §3).

## Verify

    ssh -o BatchMode=yes root@zebreus.click "echo 'SELECT COUNT(DISTINCT tbl) FROM sqlite_stat1' | /root/sq.sh"

- **done**: 27 (the 24 + `org_match_keys`, `plan_prev_edge`, `plan_notice`) and no `organizations` row
- **open**: 3 (only the internal tables) — or the query is refused (`sqlite_stat1` is not in the public surface;
  read it through the admin job's summary instead) (2026-09-27)
