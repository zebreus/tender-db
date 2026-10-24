# 580 unit 1: adversarial check of the sizing and the design

Checked 2026-10-24. I changed no code and wrote only this file. No git, cargo, box statements, ssh or HTTP. The recount
scripts are mine, written fresh rather than reusing `scripts/`. They are at
`/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/skscripts/` (`recount.py`,
`order.py`, `mix.py`), and the days 1–2 tar was extracted into that scratchpad's `sk1023/`.

**Verdict: CORRECTED.**
- **Sizing:** confirmed. Every retirement and mint count re-derives exactly. The corrections below are minor.
- **Design:** confirmed on Q1's core (the successor is not known at retirement), the path ordering, splits and chains,
  notice→Tender uniqueness and the organization 308 template. Corrected on twin repair, on four served sentences, on
  308 caching and on one /v1/sql claim.
- **Recommendation:** (b′) is not the cheapest remedy that closes the Verify "done" condition. (a) closes it on its own.

## (a) Recount

**Method.** I recounted all six all-kind archives plus the b5 tender feed: u6-lchanged, u9-days12 (extracted from
the tar), p0, w539-a, w539-b and w539-c. The script loads every chunk and checks that the cursors are exactly
lo+1..hi with no duplicates. It counts `tender removed` and the removed lot / lot_result / bid / contract rows per
`changed_at` stamp. It also counts minted ids: `tender added` with an id above every Tender id seen at an earlier
cursor.

**Results.**

| archive | rows (expected) | gaps / dups | tender removed per stamp | minted |
|---|---|---|---|---|
| u6-lchanged | 217,029 (217,029) | 0 / 0 | 10-19 07:35:55 **11**, 10-20 07:41:27 **14**, 10-21 07:39:23 **12** | 2,021 / 1,882 / 2,442, with 2 / 1 / 236 `added` on existing ids |
| u9-days12 | 176,762 (176,762) | 0 / 0 | 10-22 07:40:36 **13**, 10-23 07:39:39 **11** | 2,314 / 2,285, with 277 / 306 on existing ids |
| p0, w539-a, w539-b | 11,401 / 18,930 / 4,339 | 0 / 0 | 0 | 0 (w539-b: 54 `added` on existing ids) |
| w539-c | 7,746 (7,746) | 0 / 0 | 10-24 01:38:48 **93** | 93 (8,860,949–8,861,041), with 95 on existing ids |
| b5 (tender feed) | 3,429 tender rows in (958764958, 958787061] | n/a | 0 (all `changed`, one stamp 10-18 22:37:49) | 0 |

- **Totals.** 436,207 rows over (958787061, 959223268], contiguous. The archives do not overlap: each one's lo is
  the previous one's hi. 154 retired ids in total; 0 have any later row and 0 were retired twice. 8852015 was minted
  by 2645 and retired by 2669.
- **fresh568.** The summary's `ops` is `[('changed', None)]` over 46,879 tender events. The tender feed was paged to
  `more=false`, and 46,879 tender rows plus 862,889 lot rows equal the span exactly. So 2617's zero holds, from the
  summary.
- **Cross-checks.** `R.json` `retired` (117) equals the 24 from days 1–2 plus run c's 93. `switchoff.json`
  `retired` (24) equals the days 1–2 set.
- **Attribution.** I checked the attribution against the ledger (`scripts/jobs_2629_2741.json`). Each retirement
  stamp is the start second of project 2645, 2669 (it started at 07:41:26 and stamps 07:41:27), 2688, 2716, 2726 or
  2741. The rederive-eur-recent rows are version-null `changed` rows only, so no retirement can be mis-attributed to
  one.
- **Denominators.** They are the project lines' "tenders written": 4,070, 4,974, 5,887, 5,785 and 5,739. The rates
  check out: 61 / 26,455 = 231 per 100k; 61 / 10,944 = 0.56 %; 93 / 951,573 = 9.8; 93 / 2,476,214 = 3.8. The
  campaign total of 2,328,389 re-sums exactly.

**Corrections to the sizing (all minor; no rate changes):**
- **S1. 2716's per-job row counts include 1,403 of rederive 2715's rows.** 281 version-null tender `changed` and 1,122
  lot `changed` rows share the 07:40:36Z second with the project. The attribution rule gives a mixed stamp to the
  project, so `per-archive-counts.json` 2716 `rows`/`tender_changed` are overstated by 1,403 / 281. Proof: 2715's
  ledger announces 6,651 correction rows and only 5,248 are attributed to it; 6,651 − 5,248 = 1,403. The other three
  rederives match their ledgers exactly (4,824, 5,706, 9,229). Retirements, mints and rates are unaffected.
- **S2. The 0-of-2.33M "≤ 0.13 per 100k at 95 %" bound assumes independent trials.** 14,008 of the 2.33M are the same
  3,502 Tenders re-folded four more times (2649, 2658, 2679, plus 2730's 3,537). The zero is structural: no grouping
  input moved. Report it as structural, not as a per-Tender rate.
- **S3. "Never reappears" is near-vacuous for run c's 93.** Only about 7,700 cursors follow their retirement. The
  guarantee is AUTOINCREMENT, within a generation.
- **S4. Run c's one-to-one pairing is an inference.** Equal counts (93 retired, 93 minted) are consistent with "each
  retired Tender re-formed under one new id", but the change rows carry no notice and cannot show the pairing.
- **S5. One retired id is outside the stated bands.** 300875 (2726) is in none of the three id bands in sizing.md's
  observation 3.
- **S6. Small wording points.**
  - 2738's summary says 126,813 Tenders written, against 126,812 stamped.
  - The task summary's "two earlier reads cover tenders only" is loose: fresh568 counted the tender and lot feeds
    (sizing.md itself says so).
  - Change rows span 10-19 03:34:07Z to 10-24 01:38:48Z; 02:34Z is when 2741 finished.

## (b) Design claims in code

**Confirmed:**
- `retire_chunk_tx(conn, ids, now)` (canonical.rs:43320) takes only ids and writes the change rows per Tender in id
  order. All four callers decide by "key missing from the plan": 43244, 43555 (seek at 43577), 43609, and
  `twin_repair_tx` 43971.
- **Full path:** Phase 2, then retirement at project.rs:2225/2232, then `clear_plan` at 2234; a stopped fold skips
  retirement (2214–2222). **Incremental path:** retirement at 3796, *before* Phase 2. Run c (2741) is the incremental
  path too: a refold stamps Tenders epoch-stale "for the incremental fold".
- The feed data confirms the order. In every retirement stamp the `tender removed` block comes before the first
  `added`/`changed` of the project's fold. On 10-22 the only earlier rows are the rederive's version-null rows and the
  project's own 1,000 `organization added` from mention resolution. "In either order" is the right served phrase,
  because the full path reverses it.
- **Splits and merges:** `a_uuid_hub_splits_per_buyer_cluster_on_full_and_daily_folds` (project_incremental.rs:3771;
  doc at 3755–3758). `a_matched_row_merges_on_the_next_daily_and_its_undo_splits` (2403) asserts one removal at the
  merge and "a split retires nothing" at the undo. **Chains:** issue 386:599–612.
- `tenders.id` is AUTOINCREMENT (110), and so are lots, lot_results, bids, contracts, organizations and notices. A
  rebuild resets the sequence (18139–18194; `clear_canonical` bumps the generation at 16127 first).
- `tender removed` is written only at 43322; every other tender `append_change` is `added`/`changed`. The change log
  is never trimmed except by `clear_changes`, which bumps the generation (18235).
- **Notice→Tender uniqueness:** `plan_notice.notice_id` is the primary key (18489), and the prod check found at most
  one row per notice (43675–43676). The schema allows several (225–226).
- **Organization template:** mod.rs:2067–2089 (308, `Location`, `merged_into`); openapi.json:247–251.
- `/v1/notices?tender=<retired>` answers 404 (mod.rs:2117–2128). This corrects the issue text.
- `tender_versions` is a leaf in CONTENT_TABLES (1426), so `retire_chunk_tx` deletes it. The golden tests count the
  14 tables matching `tender_versions` / `tender_version_*`. `IN_CHUNK` is 512 (45245).
- The public feed is tender, lot and organization only (mod.rs:2158–2167), so "a `removed` for each of its lots" is
  the complete served statement.
- **Line drift (harmless):** `tender_versions_notice` is at 232, not 231. The `RETIRE_TABLES` DELETE loop starts at
  43345, so the INSERT goes after 43344 and before 43345, not "between 43341 and 43343".

**Corrected:**
- **D1. Twin repair is described wrongly, and (b′) as specified mis-answers it.**
  - `twin_repair_tx` (43900–43975) deletes organization_mentions, the 9 parsed tables, quarantine and `notices`
    (43908–43930). It does **not** delete `tender_versions`. The emptied Tender's versions are still there when
    `retire_chunk_tx` runs at 43972, pointing at notice ids that no longer exist.
  - With the proposed INSERT…SELECT, ledger rows *are* written, keyed on those deleted notice ids. They never join
    to a live version, so the id answers **410 with `successors: []` permanently**. The design says "no rows (falls
    to the change-log 410)" with `null`.
  - **The successor is knowable here.** The planner computes `keeper_tender = tender_of(reader, repair.keep)`
    (around 43784); the surviving twin's notice is `plan.plan[].keep`.
  - **Fix:** for twin repair, write `(emptied tender, keep notice id)` instead of the dropped ids. Or leave twin-repair
    retirements out of the ledger, so they fall to `null`. Either way, add a test.
  - The production rate is low (issue 404: three retirements in 1625/1626), but the behaviour is served.
- **D2. The resolver's `MAX(id)` guard should read `sqlite_sequence`.** `MAX(id)` of live Tenders is not the
  high-water mark. An id above every live id, because the newest ids were retired, would answer 404 although it was
  retired. Rare, and only on the pre-ledger path.
- **D3. Q4: `MAX(tender_id)` does not always pick the successor.** In the transient full-path window it picks the
  successor only when the successor was newly minted. A merge into an existing, older Tender (the DÖE island into the
  TED Tender) leaves the ghost holding the higher id. Moot unless (c) is built.
- **D4. Q2: `v_tender_notices` does not "also work" on /v1/sql.** It is NOT FILTERABLE (sql.rs:1014–1016), and a
  filtered read is refused (test at sql.rs:3049). Only the `tender_versions` form works, and it must never be written
  into a served doc.

## (c) Is (b′) the cheapest remedy that fully closes Verify, and is every proposed served sentence true?

**Cheapest: no.**
- The done line is: "the second number is ≥ 1: /docs and OpenAPI say a Tender id can be retired … and how a client
  follows its notices. *If unit 1 also adds a successor pointer*, the first line answers `308`".
- (a) alone, size S, fully closes it. The 308 clause applies only if (b) is chosen.
- (b′) is size M. It touches crates/store, so it needs a B4 W-on probe. The design itself puts it after the 551 U9
  watch (11-04).
- (b′) also **cannot** satisfy the done line as written. 8811451 will answer 410 with `null`, because there is no
  backfill. The design says so, and unit 3's Verify would have to be rewritten to use an id retired after the deploy.
- **Recommendation:** close 580 with (a), which is the design's own fallback "2a", using the [a] text plus the fixes
  below. File (b′) as its own issue with its own Verify ("an id retired after the deploy answers 308 or 410 with its
  successors; 8811451 answers 410 with `null`"). That keeps a docs fix from waiting on a store change and on the 551
  U9 window. Parity with issue 455 is a fair reason to build (b′), but it is not needed to close 580.

**Served sentences that would be false or misleading as drafted:**
- **F1. [b′] "or `[]` while none is folded (as can happen briefly during a fold)".** False for twin-repair
  retirements (permanent `[]`, see D1). Also false for a retired Tender whose notices are later held, since a held
  notice has no Tender (json.rs:202–204). Say "`[]` when none of its notices is in a Tender now (briefly during a
  fold, or for good if they were withdrawn)", or fix D1 and name the held case.
- **F2. [b′] "an id retired twice answers its latest successor directly".** An id is never retired twice, and the same
  paragraph says it is never issued again. Write "an id whose successor was itself later retired answers the latest
  successor directly".
- **F3. [b′] 308 against "where the notices are when you ask".**
  - 308 is a permanent redirect, and RFC 9111 lets a cache store it heuristically. The org handler sets no
    Cache-Control (no CACHE_CONTROL anywhere in crates/app except SSE).
  - The Tender successor is computed at request time and can change. In the merge-then-undo case the design cites as
    (b′)'s advantage, a cached 308 would land a client on the TED Tender after the undo.
  - Send `Cache-Control: no-store` on the 308 and 410, or use 307, and test the header.
- **F4. [a]/[b′] "On /v1/sql: `SELECT tender_id FROM tender_versions WHERE caused_by_notice_id = ?`".** /v1/sql takes
  the raw SQL as the request body (docs.rs, `#sql`), with no bind parameters, so a literal `?` fails. Write
  `= <notice id>` or a concrete literal.
- **F5. [a] "the id is never issued again".** A rebuild reissues ids, and the same page says so. Write "never issued
  again in this generation".
- **F6. Minor scope wording.**
  - "retired by a fold", in the [a] inline 404, the [b′] table row and "A Tender id a fold retired answers 308": the
    twin repair (`repair-member-twins`) also retires, and it is not a fold. Write "retired by a fold or a repair".
  - "Its notices fold into another Tender, new or existing, or into several": add "or, if they are withdrawn, none".

Everything else in the [a] text is true against the code: the generation does not move, the feed carries `removed`
with `version: null` plus the lots' `removed` in either order, `/v1/notices?tender=` answers 404, and
`?publication_id=&source=` answers the Tender holding the notice now (read.rs:2050–2097).
