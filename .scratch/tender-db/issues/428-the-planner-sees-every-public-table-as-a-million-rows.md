# 428 — the planner sees every public table as a million rows: measure `ANALYZE` on a snapshot before deciding

Status: done — measured 2026-09-26/27 on a reflinked copy of the Sep 20 snapshot; decision taken (below). Implementation is issue 429.
on all user tables, so the engine can choose the best plan for any arbitrary user query?". Mine to decide;
the decision is gated on the measurement below, not on anyone's word.
Kind: query planning (the `/v1/sql` surface first, the API's own read paths second)
Relates to: 421 (a plain JOIN planned backwards — the kind of trap stats may fix), 112 / 114 / 122 (plan gates
that are valid ONLY while `sqlite_stat1` is empty, and say so), 256 (`ANALYZE plan_*`), 425 / 426

## What is true today

- turso 0.7.2's optimizer is cost-based (`translate/optimizer/cost.rs`) and DOES use `sqlite_stat1`: the table
  row count (`RowCountEstimate::AnalyzeStats`) and per-index `avg_rows_per_distinct_prefix` for seek
  selectivity (`constraints.rs`, `cost.rs` ~286). Without stats it assumes **1,000,000 rows for every table**
  (`cost_params.rs`, `rows_per_table_fallback`).
- We run `ANALYZE` only on three internal tables (`org_match_keys`, `plan_prev_edge`, `plan_notice`). Every
  public table — `tenders` 8.6M, `lots` 14.0M, `tender_versions` 14.5M, `changes` 625M, the satellites in the
  tens to hundreds of millions — looks the same size to the planner.
- **That is deliberate, and recorded** (issue 122): every hand-tuned read path of the API and every plan gate
  (112/114/122) was designed and verified against the stats-free planner; a bare `ANALYZE` "would populate
  every read-path table and silently change the planner underneath every measurement". The gates check for
  it and report VOID if stats appear.

## Why it is worth revisiting

`/v1/sql` runs arbitrary queries the app never tuned, and the documented user-facing traps (421's backwards
JOIN, the `kind IN (…)` prefix walk) are exactly the mistakes a planner makes when it cannot tell a 14M-row
table from a 14k-row one. Stats could fix those for every analyst at once — or could move the app's own
tuned plans somewhere worse. Neither is known.

## The experiment (on a SNAPSHOT, never the serving DB)

1. Copy of the newest weekly snapshot (`/data/db/snapshots/tender-db-*.db`, ~672 GB) — run `ANALYZE`, timed
   (the 2026-08 research measured 11.3 s at 10 GB; at 630 GB it is unmeasured — expect tens of minutes).
2. Before/after on that copy: (a) the app's plan gates and hot read plans (`hot_read_plans.sh`, the 112/114/122
   gates — they must be pointed at the copy and will report VOID-with-stats by design; read the plans
   themselves); (b) the known `/v1/sql` traps (421's plain JOIN, the `kind IN` shape, 423's lots shapes); (c)
   timings of the list endpoints' statements through the engine.
3. Decide from the diff: all tables, a subset (only those whose plans improve and none regress), or none. If
   yes, it becomes a scheduled weekly job (it holds the writer for its duration, so it runs in the quiet
   window like the data-quality run), and the stats-absence preconditions in the plan gates change to
   "stats as of <date>" rather than "no stats".

## Verify

    ssh -o BatchMode=yes root@zebreus.click "echo 'SELECT COUNT(*) FROM sqlite_stat1' | /root/sq.sh"

- **done**: the experiment's before/after is recorded here and the decision taken — DONE 2026-09-27; the
  count stays 3 (the internal tables) until issue 429's job has run, then reads the analyzed subset
- **open**: the query is refused — `sqlite_stat1` is not in the public surface, and nothing is recorded here
  yet (2026-09-26)

## Measured (2026-09-26 21:53 → 2026-09-27 00:45 UTC)

Two reflinked copies of `/data/db/snapshots/tender-db-1789874587.db` (672 GB; no extra disk — `df` did not
move), `base` (as prod: no stats) and `stats` (ANALYZEd). All work fenced in its own cgroup (MemoryHigh 6G,
IO class idle, nice 19) so the serving DB's page cache was not evicted; the queue stayed idle and `/health`
green throughout. Both copies deleted afterwards. The instrument is `plan-probe` (ef6723a + this commit)
and the suites' statement capture (`TENDER_PLAN_CAPTURE`, ef6723a). Artifacts: `.scratch/tender-db/428/`.

### 1. What ANALYZE costs

**No write cost at all.** `sqlite_stat1` is a snapshot the planner reads; nothing maintains it on INSERT/UPDATE
and index maintenance is unchanged. The whole cost is the ANALYZE run (it full-scans every index of the table,
and holds the writer for that table's duration) plus keeping it fresh.

| set | tables | time on the 672 GB DB |
| --- | --- | --- |
| read path (canonical layer + notices + quarantine + orgs; `notice_withheld_fields` is a view) | 25 | **1,609 s (27 min)** — longest `tender_version_parties` 242 s, `…_classifications` 226 s, `…_result_winners` 190 s |
| `changes` | 1 | 1,152 s |
| raw `notice_*` satellites | 8 | 5,012 s — `notice_texts` alone 2,850 s |
| everything | 118 | **8,797 s (2 h 27 min)** |

Per-table: `analyze-timings.log`.

### 2. What it changes — every statement the app prepares

`captured-statements.sql`: the 760 distinct statements turso prepared while the api/sql/admin/webhooks suites
ran (521 SELECT/DML; 461 of them produce a plan). Plans diffed base vs stats (`plan-deltas.txt`):

| stats on | plans unchanged | changed |
| --- | --- | --- |
| all 118 tables | 387 | 74 |
| read-path subset | 411 | 50 |
| **read-path subset minus `organizations`** | **412** | **49** |

With **all** tables the 74 split into:

- **better, 42** — 33 statements seek `tender_versions` on `(tender_id, seq)` instead of `(tender_id)` alone (the
  tenders/lots list heads, summaries, detail); `s0239` (`notices WHERE parse_state='parsed' AND id BETWEEN …` —
  the issue-323 shape) walks the id range instead of every parsed row: **9.403 s cold / 1.119 s warm → 0.001 s**;
  the `v_*` views and a cpv count drive from `tenders` instead of scanning a satellite.
- **neutral, 7** — a different index with the same leading column (`organization_mentions`, `tender_version_parties_org*`).
- **worse, ~20: small operational tables flip from a PK/index seek to `SCAN`** (`projection_state`, `feed_generation`,
  `legacy_adjacency` — one row each; `reports` 53 rows of large JSON bodies; `fetches` 607; `package_rates` 73;
  `webhook_*`). Measured: `reports WHERE kind=?` 0.002 → **0.227 s cold**; `fetches` 0.002 → 0.012 s. Cheap, but
  pure loss — and they come ONLY from analyzing those small tables, which the subset does not.
- **worse, 1, and it is the real one: the org resolver** (`canonical.rs:9203` and `:20005`, `SELECT id FROM
  organizations WHERE name_norm = ? AND country = ? AND identifier IS NULL LIMIT 1`). Stats say both
  `organizations_name_country` and `organizations_name_norm_id` average **2 rows per name**, so the planner takes
  the narrower one. The real distribution is wildly skewed: `name_norm = ''` carries **1,415,306** organizations
  across 135 countries, `tribunal administratif` 25,660 across 19, `avenue-web systèmes` 68,239. A miss walks the
  whole name group: **0.001 s → 1.580 s cold** (`tribunal administratif`, ZZ) and **0.001 s → 6.190 s cold / 0.756 s
  warm** (`''`, ZZ) — on the fold's per-mention hot path. `INDEXED BY organizations_name_country` fixes it
  (0.001 s on both copies; turso 0.7.2 enforces the hint) but is NOT usable: that index is deferred (built by
  `reindex`, issues 62/111), and a hint naming a missing index fails at prepare. The siblings `:9299` (country
  IS NULL) and `:9950` keep `name_country` either way (measured 0.001 s).

None of the hand-tuned list/seed plans (issues 223, 388, 408, 423, 424 — the lots/tenders pages, seeds, windowed
walks) changed under any stats set.

### 3. What it changes for `/v1/sql` (`traps.sql`, `timed.sql`)

| shape | no stats | with stats |
| --- | --- | --- |
| 421: `tenders t JOIN tender_version_classifications c ON … c.scheme='cpv' WHERE t.id BETWEEN` (2,000 ids) | drives from `scheme=?` over every CPV row — 10 s cap on prod | **tenders PK range → (tender_id, seq) seek: 0.032 s cold / 0.013 s warm** |
| 421: same with `tender_version_texts` `x.field='title'` | `SCAN tender_version_texts` — cap | **0.185 s cold / 0.027 s warm** |
| 421: same with `tender_version_parties` `p.role LIKE '%uyer%'` | `SCAN tender_version_parties` — cap | **0.023 s cold / 0.011 s warm** |
| 421: the documented `CROSS JOIN` form | 0.036 s cold | 0.036 s cold (unchanged — the guidance stays correct) |
| `SELECT * FROM v_tenders WHERE id BETWEEN …` | `SCAN tender_versions` (14.5M) | `SCAN tenders` (8.5M) + seek — better, still a full scan (the view gets no pushdown) |
| 417: `… JOIN notice_codes c ON … AND c.field_id='TXT-NC'` | `SCAN notice_codes` | **unchanged — still a trap** (the `+c.field_id` fix stays necessary) |
| lots → lot_results → result_winners by tender range | `SCAN tender_version_result_winners` | **unchanged — still a trap** |
| 329 `IN` list, 323 `parse_state`, `fetch_id … GROUP BY profile` | as documented | unchanged (323's app statement is fixed by the notices stats, the `/v1/sql` shape is not) |

## Why stats can make a query slower (the answer to "more info should help, right?")

Measured above, not argued. turso 0.7.2 reads only `sqlite_stat1` — a row count and, per index prefix, the
**average** rows per distinct value (`stats.rs`; there is no `sqlite_stat4`, no histogram). An average is exactly
wrong for a skewed column: 6.85M organizations over ~3.4M names averages 2 per name, while one name has 1.4M.
Without stats the planner assumed nothing and happened to prefer the wider index; with stats it "knows" both are
equally selective and picks the one that loses on every generic name. Plans are also compiled before parameters
are bound, so even a histogram could not tell `''` from a rare name. The second effect is small tables: real row
counts make a scan of a 53-row table look cheaper than a seek, but the cost model counts rows, not the megabytes
of JSON in them. Info helps on average — that is what fixed every 421 trap — and hurts precisely where the data
is far from its average.

## Decision

**ANALYZE a fixed subset weekly: the read-path set minus `organizations`** — 24 tables (`tenders lots
tender_versions organization_names organization_mentions tender_version_{dates,classifications,parties,texts,
amounts,lots,bids,bid_parties,contracts,lot_results,result_winners,result_stats,lot_group_members} lot_results
bids contracts currency_rates quarantine notices`) plus the three already analyzed (`org_match_keys`,
`plan_prev_edge`, `plan_notice`). About 26 min of writer time a week; 49 plan changes, every one an improvement
or neutral; the 421 trap class fixed for every analyst.

- NOT `organizations` — the one measured regression, on the fold's hot path, with no safe hint available.
- NOT the raw `notice_*` layer or `changes` — 1 h 44 min of ANALYZE for zero measured plan benefit.
- NOT the small operational tables — their stats only turn seeks into scans.
- Reversible in one statement: `DELETE FROM sqlite_stat1 WHERE tbl IN (…)` and a restart.

Built as issue 429 (the job, the list as a constant with this issue's evidence, a plan test that pins the
exclusions, stats pickup by running readers, and the 112/114/122 gate preconditions).
