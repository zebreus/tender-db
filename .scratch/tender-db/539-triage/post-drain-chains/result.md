# 539 post-drain audit: Tender version chains (2026-10-24 03:56–04:02Z)

**Verdict: 0 invariant violations.** The audit checked 26,639 adjacent version pairs across 23,574 live Tenders in 5
tender-id windows. 12,798 of those pairs have a DÖE notice behind either version, and 13,841 do not. Every chain is
contiguous (seq 1..n), and every `tenders.current_seq` / `current_published_at` matches its last version. The 114
Tenders in the windows whose chains the refold re-announced below the head are all in order. So are 3 of them
spot-checked through the public API (8860958, 248260, 250367).

A second check covers 16,210 versions in 2 of the windows. Every stored version instant (published and dispatched)
equals its causing notice's stored instant, with 0 mismatches for DÖE (5,284 versions) and 0 for non-DÖE (17,345).
Matching on both sides is what you would see if the R3 refolds re-derived every DÖE version that repair-notice-instants
re-stamped.

Budget used: 7 of 10 `/v1/sql` statements (0 errors, 0 408s, queue idle before each, 5–20 s apart) and 3 of 6 public
GETs. Read-only throughout.

## 1. The invariant (derived from code)

Line numbers are for the local tree at `aba602c` (claude/cool-sagan-5bk0rc). The deployed revision is `512b297`, with
its line numbers in brackets. The diff between the two does not touch any line cited here.

**Fold order within a Tender.** The key is `(published_at, source_rank, publication_id, notice_id)`, ascending, with
`source_rank = 1` for `ted` and `0` for every other source (DÖE). `dispatched_at` is **not** a tie-breaker.
- `crates/ingest/src/project.rs:4564-4570` [4487-4490]: `BucketRow::sort_key` is
  `(group_key, published_at, source_rank, publication_id, notice_id)`, documented as "IDENTICAL to `plan_notice_fold` /
  `next_plan_batch`'s `ORDER BY`". The bucketed fold sorts each bucket by it at `project.rs:4042` [3964].
- `crates/store/src/canonical.rs:21038` [21034]: the streaming path is
  `next_plan_batch … ORDER BY group_key, published_at, source_rank, publication_id, notice_id`. `apply_plan_batch`
  folds each group in that order (`group.notice_ids`, `project.rs:3947-3963` [3873]) through `fold()`
  (`project.rs:7015` [6807]).
- `project.rs:6987-6999` [6786]: `source_rank` is ted ⇒ 1, otherwise 0. "On an equal publication instant the higher rank
  folds last", so TED wins (ADR-0003).
- The instant comes from `project.rs:12756-12765` [12324-12331], `notice_stamps`. For DÖE it is `doe_publication`, the
  latest of the publication candidates and the IssueDate's civil day (the 539 rule). For every other source it is the
  best-ranked publication field. Either way it falls back to the dispatch instant with `.or(dispatched)`
  (`:12763` [12331]). That fallback is the "dispatch fallback" in CONTEXT.md:251. Dispatch never breaks a tie.
- When neither date exists, the plan key and the version use the same epoch-0 fallback:
  `notice_instants(..).0.unwrap_or(0)` (`project.rs:6922` [6714] for the plan, `project.rs:6005-6007` [5862] for the
  state).

**seq and the stored columns.**
- `canonical.rs:24246-24247` [24243]: `seq = i + 1` over `p.versions`, which is the fold order. So seq is the fold rank.
- `canonical.rs:42710-42729` [42605]: `write_version` stores `v.published_at` (the instant the fold sorted by), `v.dispatched_at`
  and `v.publication_id` in `tender_versions`.
- Schema, `canonical.rs:212-231`: `published_at INTEGER NOT NULL` is unix seconds, with a date-only value anchored at
  the civil day's UTC midnight (issue 418). The table has `PRIMARY KEY (tender_id, seq)` and
  `UNIQUE (tender_id, caused_by_notice_id)`.
- `notices.source` gives the rank.

**Why a refolded chain re-sorts.** `canonical.rs:24112-24120` [24107]: an epoch-stale Tender keeps nothing (`keep = 0`),
so its whole chain is re-derived. The R3 refold stamps every Tender of the re-dating profiles stale. With the compare in
`On` mode:
- the prefix whose causing notices are unchanged is compared and corrected in place (seq-less D3 rows);
- from the first position where the notice differs (a reorder), versions are re-written and announced with
  version-level rows (`append_version_changes`, `canonical.rs:43100`).

A non-stale Tender keeps its prefix by `caused_by_notice_id` (`:24116-24119`). The one writer that can break the order
is `repair-version-instants` (`canonical.rs:36310` [36306]): it updates `published_at` in place and never re-sorts. The
drain did not run it.

**Served shape.** `crates/store/src/read.rs:3390` (`tender_detail`), versions query at `:3499-3505`:
`… FROM tender_versions v LEFT JOIN notices n ON n.id = v.caused_by_notice_id WHERE v.tender_id = ? ORDER BY v.seq`.
- The detail lists versions in seq order.
- It renders each version's own `published_at` instant in the causing notice's stored offset / has_time. A date-only
  value is served as `YYYY-MM-DD`.
- The served order is the stored order, so checking `tender_versions` checks what `/v1/tenders/{id}` serves.

**Checkable form.** For every adjacent pair a = seq k, b = seq k+1 of a Tender, both of these must hold:
- `a.published_at < b.published_at`, or
- on an equal instant, `(rank_a, a.publication_id, a.caused_by_notice_id) < (rank_b, b.publication_id, b.caused_by_notice_id)`.
  Text compares bytewise (BINARY, the same as Rust `&str` Ord). Notice ids are unique per Tender.

The audit splits violations two ways:
- **date** (`a.published_at > b.published_at`) vs **tie** (equal instant, tie-break out of order);
- **DÖE** (`na.source = 'doe' OR nb.source = 'doe'`) vs **non-DÖE**.

## 2. Statements

### 2a. Chain statement (w1–w4; w5 adds a sdk-0.1 split). One row per live Tender in the window.

`w3.sql` (sha256 5a817cd8…) verbatim. w1, w2 and w4 differ only in the `BETWEEN` bounds:

```sql
SELECT t.id AS tid,
       t.current_seq AS cur_seq,
       COUNT(*) AS nv,
       MIN(a.seq) AS min_seq,
       MAX(a.seq) AS max_seq,
       SUM(CASE WHEN na.id IS NULL THEN 1 ELSE 0 END) AS n_nonotice,
       SUM(CASE WHEN na.source = 'doe' THEN 1 ELSE 0 END) AS n_doe,
       SUM(CASE WHEN na.source = 'ted' THEN 1 ELSE 0 END) AS n_ted,
       MAX(CASE WHEN a.seq = t.current_seq THEN a.published_at END) - t.current_published_at AS head_pub_delta,
       COUNT(b.seq) AS pairs,
       SUM(CASE WHEN b.seq IS NOT NULL AND (na.source = 'doe' OR nb.source = 'doe') THEN 1 ELSE 0 END) AS pairs_doe,
       SUM(CASE WHEN b.seq IS NOT NULL AND a.published_at = b.published_at THEN 1 ELSE 0 END) AS pairs_same_instant,
       SUM(CASE WHEN b.seq IS NOT NULL AND na.source <> nb.source THEN 1 ELSE 0 END) AS pairs_cross_source,
       SUM(CASE WHEN b.seq IS NOT NULL AND (na.source = 'doe' OR nb.source = 'doe')
                 AND a.published_at > b.published_at THEN 1 ELSE 0 END) AS vd_doe,
       SUM(CASE WHEN b.seq IS NOT NULL AND (na.source = 'doe' OR nb.source = 'doe')
                 AND a.published_at = b.published_at
                 AND ((na.source = 'ted') > (nb.source = 'ted')
                      OR ((na.source = 'ted') = (nb.source = 'ted')
                          AND (a.publication_id > b.publication_id
                               OR (a.publication_id = b.publication_id AND a.caused_by_notice_id > b.caused_by_notice_id))))
                THEN 1 ELSE 0 END) AS vt_doe,
       SUM(CASE WHEN b.seq IS NOT NULL AND NOT (na.source = 'doe' OR nb.source = 'doe')
                 AND a.published_at > b.published_at THEN 1 ELSE 0 END) AS vd_non,
       SUM(CASE WHEN b.seq IS NOT NULL AND NOT (na.source = 'doe' OR nb.source = 'doe')
                 AND a.published_at = b.published_at
                 AND ((na.source = 'ted') > (nb.source = 'ted')
                      OR ((na.source = 'ted') = (nb.source = 'ted')
                          AND (a.publication_id > b.publication_id
                               OR (a.publication_id = b.publication_id AND a.caused_by_notice_id > b.caused_by_notice_id))))
                THEN 1 ELSE 0 END) AS vt_non,
       MIN(CASE WHEN b.seq IS NOT NULL
                 AND (a.published_at > b.published_at
                      OR (a.published_at = b.published_at
                          AND ((na.source = 'ted') > (nb.source = 'ted')
                               OR ((na.source = 'ted') = (nb.source = 'ted')
                                   AND (a.publication_id > b.publication_id
                                        OR (a.publication_id = b.publication_id AND a.caused_by_notice_id > b.caused_by_notice_id))))))
                THEN a.seq END) AS first_viol_seq
  FROM tenders t
 CROSS JOIN tender_versions a INDEXED BY sqlite_autoindex_tender_versions_1 ON a.tender_id = t.id
  LEFT JOIN notices na ON na.id = a.caused_by_notice_id
  LEFT JOIN tender_versions b INDEXED BY sqlite_autoindex_tender_versions_1 ON b.tender_id = a.tender_id AND b.seq = a.seq + 1
  LEFT JOIN notices nb ON nb.id = b.caused_by_notice_id
 WHERE t.id BETWEEN 248001 AND 253000
 GROUP BY t.id
 LIMIT 6000
```

`w5.sql` is w3 with bounds `1016501 AND 1021500` and two more columns:
- `SUM(CASE WHEN na.profile = 'eforms:eforms-sdk-0.1' THEN 1 ELSE 0 END) AS n_sdk01`
- `SUM(CASE WHEN b.seq IS NOT NULL AND (na.profile = 'eforms:eforms-sdk-0.1' OR nb.profile = 'eforms:eforms-sdk-0.1') THEN 1 ELSE 0 END) AS pairs_sdk01`

Notes on the statement:
- **Why `INDEXED BY`.** Without it, turso 0.7.2 seeks `b` by `tender_id` alone on the UNIQUE
  `(tender_id, caused_by_notice_id)` autoindex as soon as any `b` column is selected. That reads nv² rows per Tender.
  This was measured in the local plan probes (variants `f_*` / `g_*`, not kept). The pin restores
  `(tender_id=? AND seq=?)`. `/v1/sql` has accepted `INDEXED BY` before (551 u9 `box_confirm/raw/0001_w12a_e1_hint`).
- **Why per-Tender rows.** `group_concat` is refused by `/v1/sql` (issue 426; refusal precedent in
  `546-triage/a_w19_groupconcat_refused.out.json`). The statement therefore returns one row per Tender (≤ 5,000, under
  the 10,000-row cap), and the window tallies, example ids, min/max and violating ids are computed locally by `summ.py`.

### 2b. Instant agreement statement (i2, i3). One aggregate row.

`i3.sql` (sha256 29c72211…). i2 differs only in the bounds (8811001–8816000):

```sql
SELECT MIN(t.id) AS min_tid,
       MAX(t.id) AS max_tid,
       COUNT(*) AS versions,
       SUM(CASE WHEN na.id IS NULL THEN 1 ELSE 0 END) AS no_notice,
       SUM(CASE WHEN na.source = 'doe' THEN 1 ELSE 0 END) AS doe_versions,
       SUM(CASE WHEN na.source = 'doe' AND a.published_at <> COALESCE(na.published_at, 0) THEN 1 ELSE 0 END) AS doe_pub_mismatch,
       SUM(CASE WHEN na.source = 'doe' AND a.dispatched_at IS NOT na.dispatched_at THEN 1 ELSE 0 END) AS doe_disp_mismatch,
       SUM(CASE WHEN na.source = 'doe' AND na.published_at IS NULL THEN 1 ELSE 0 END) AS doe_notice_pub_null,
       MIN(CASE WHEN na.source = 'doe' AND a.published_at <> COALESCE(na.published_at, 0) THEN t.id END) AS doe_mm_min_tid,
       MAX(CASE WHEN na.source = 'doe' AND a.published_at <> COALESCE(na.published_at, 0) THEN t.id END) AS doe_mm_max_tid,
       SUM(CASE WHEN na.source <> 'doe' THEN 1 ELSE 0 END) AS non_versions,
       SUM(CASE WHEN na.source <> 'doe' AND a.published_at <> COALESCE(na.published_at, 0) THEN 1 ELSE 0 END) AS non_pub_mismatch,
       SUM(CASE WHEN na.source <> 'doe' AND a.dispatched_at IS NOT na.dispatched_at THEN 1 ELSE 0 END) AS non_disp_mismatch,
       MIN(CASE WHEN na.source <> 'doe' AND a.published_at <> COALESCE(na.published_at, 0) THEN t.id END) AS non_mm_min_tid,
       MAX(CASE WHEN na.source <> 'doe' AND a.published_at <> COALESCE(na.published_at, 0) THEN t.id END) AS non_mm_max_tid
  FROM tenders t
 CROSS JOIN tender_versions a INDEXED BY sqlite_autoindex_tender_versions_1 ON a.tender_id = t.id
  LEFT JOIN notices na ON na.id = a.caused_by_notice_id
 WHERE t.id BETWEEN 248001 AND 253000
```

What it tests:
- The fold stamps a version with the same resolver the notice row carries. The two agree by construction wherever the
  resolver found a date (the invariant `notices_and_their_versions_carry_the_same_instants`).
- Step 1 (repair-notice-instants) re-stamped the notice rows and the R3 refolds re-derived the version rows. A DÖE
  version that disagrees with its notice would be one the refold missed, or a notice the repair missed.

### 2c. Plans

Plans come from `tplan.sh`, run on `post-drain-chains/plan.db`. That file is a byte copy of `539-triage/plan.db`:
schema only, no rows, no stats, nothing added. Its indexes on the three tables match the code's `CREATE INDEX` list:
- tender_versions: PK and UNIQUE autoindexes, `_published`, `_notice`;
- notices: `_profile`, `_parse_state`, `_fetch_id`, `_member_identity`;
- tenders: `_current_published`, `_current_value_eur`.

Prod's partial `notices_unprojected` index is absent, and it cannot serve a rowid seek.

Each plan was produced by both python sqlite3 3.45.1 and turso (b6u6kit plan-probe, sha256 50a8094e…). After
normalising the window name, all five chain plans are identical, and so are both instant plans. The full text is in
`<name>.plan.txt`. `w3.plan.txt` verbatim:

```
(13, 0, 0, 'SEARCH t USING INTEGER PRIMARY KEY (rowid>? AND rowid<?)')
(18, 0, 0, 'SEARCH a USING INDEX sqlite_autoindex_tender_versions_1 (tender_id=?)')
(23, 0, 0, 'SEARCH na USING INTEGER PRIMARY KEY (rowid=?) LEFT-JOIN')
(28, 0, 0, 'SEARCH b USING INDEX sqlite_autoindex_tender_versions_1 (tender_id=? AND seq=?) LEFT-JOIN')
(40, 0, 0, 'SEARCH nb USING INTEGER PRIMARY KEY (rowid=?) LEFT-JOIN')
-- turso plan-probe (rc=0) 50a8094ed7de7ed9
PLAN	w3	SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
PLAN	w3	SEARCH a USING INDEX sqlite_autoindex_tender_versions_1 (tender_id=?)
PLAN	w3	SEARCH na USING INTEGER PRIMARY KEY (rowid=?) LEFT-JOIN
PLAN	w3	SEARCH b USING INDEX sqlite_autoindex_tender_versions_1 (tender_id=? AND seq=?) LEFT-JOIN
PLAN	w3	SEARCH nb USING INTEGER PRIMARY KEY (rowid=?) LEFT-JOIN
```

`i3.plan.txt` verbatim:

```
(6, 0, 0, 'SEARCH t USING INTEGER PRIMARY KEY (rowid>? AND rowid<?)')
(11, 0, 0, 'SEARCH a USING INDEX sqlite_autoindex_tender_versions_1 (tender_id=?)')
(16, 0, 0, 'SEARCH na USING INTEGER PRIMARY KEY (rowid=?) LEFT-JOIN')
-- turso plan-probe (rc=0) 50a8094ed7de7ed9
PLAN	i3	SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
PLAN	i3	SEARCH a USING INDEX sqlite_autoindex_tender_versions_1 (tender_id=?)
PLAN	i3	SEARCH na USING INTEGER PRIMARY KEY (rowid=?) LEFT-JOIN
```

What the plans guarantee:
- No step is a SCAN, and every big table is searched by PK or a PK-prefix index.
- The row bound is the `tenders` rowid range of ≤ 5,000 ids. turso prints every rowid seek as `(rowid=?)`, ranges
  included; this is the CROSS-JOIN-from-tenders shape that issue 421 measured at 0.04–0.08 s.
- There are no IN-lists.
- Measured wall time on prod was 0.36–1.30 s per statement.

### 2d. Logic test, before anything was sent

`logic_test.db` is a copy of the plan DB with 8 synthetic Tenders:
- a valid DÖE/TED same-instant chain;
- one DÖE date inversion;
- a TED publication_id tie inversion;
- a TED-before-DÖE rank inversion at an equal instant;
- a notice-id tie inversion;
- a seq gap with a head mismatch;
- one Tender outside the window.

The chain query (`logic_test_q.sql`) returned the expected value in every cell in python sqlite. turso returned
identical values for all 7 in-window Tenders: each row was asserted with an `IS`-equality wrapper, and each check
returned rows=1. The instant query (`logic_test_qi.sql`) was checked the same way: 2 DÖE mismatches, 1 non-DÖE
mismatch and 1 NULL notice stamp, with python and turso agreeing. The w5 sdk-0.1 columns were checked in python.

## 3. Windows (chosen from local evidence, no scan)

The windows were chosen from the change rows the three runs wrote. The source was
`551-lot-sum/b6/u9/segments/w539-{a,b,c}/read/changes/all/chunk_*.json` together with the `changed_live_ids.json`
beside them (13,618 changed live Tenders in all).

| window | tender ids | why |
|---|---|---|
| w1 | 8,857,001–8,862,000 | All 93 run-c Tenders 8,860,949–8,861,041 that were re-announced from seq 1 (`1 added, 2..n changed`), which are recent DÖE+TED chains |
| w2 | 8,811,001–8,816,000 | 65 changed heads and 8 version-level Tenders (b1, c7), recent de-2.x |
| w3 | 248,001–253,000 | 70 changed heads and 7 version-level Tenders (a1, b2, c4) |
| w4 | 178,001–183,000 | 50 changed heads and 5 version-level Tenders (a3, b1, c1), DÖE-only sdk-0.1 chains |
| w5 | 1,016,501–1,021,500 | Densest window of run-a changed heads (63), with the sdk-0.1 split |

## 4. Per-window tallies (`summ.py` → `tallies.json`)

| | w1 | w2 | w3 | w4 | w5 | total |
|---|---|---|---|---|---|---|
| min / max tid returned | 8857001 / 8861041 | 8811001 / 8816000 | 248001 / 253000 | 178001 / 183000 | 1016501 / 1021500 | |
| Tenders (live, with versions) | 4,041 | 4,851 | 4,891 | 4,897 | 4,894 | **23,574** |
| versions | 4,660 | 10,926 | 11,703 | 11,471 | 11,453 | 50,213 |
| Tenders with 2+ versions | 347 | 1,786 | 3,169 | 3,193 | 3,109 | 11,604 |
| Tenders with a DÖE version | 1,094 | 1,553 | 1,195 | 1,180 | 1,181 | **6,203** |
| 2+-version Tenders with a DÖE version | 280 | 1,098 | 1,043 | 1,033 | 1,018 | 4,472 |
| Tenders with DÖE and TED versions | 278 | 1,097 | 1,008 | 1,003 | 970 | 4,356 |
| adjacent pairs checked | 619 | 6,075 | 6,812 | 6,574 | 6,559 | **26,639** |
| pairs with a DÖE version | 512 | 4,019 | 2,897 | 2,651 | 2,719 | **12,798** |
| pairs without a DÖE version | 107 | 2,056 | 3,915 | 3,923 | 3,840 | 13,841 |
| pairs at an equal instant (tie-break decided) | 51 | 926 | 411 | 352 | 335 | 2,075 |
| cross-source pairs (DÖE↔TED) | 490 | 3,855 | 2,796 | 2,573 | 2,612 | 12,326 |
| **violations DÖE, date** | 0 | 0 | 0 | 0 | 0 | **0** |
| **violations DÖE, tie** | 0 | 0 | 0 | 0 | 0 | **0** |
| **violations non-DÖE, date** | 0 | 0 | 0 | 0 | 0 | **0** |
| **violations non-DÖE, tie** | 0 | 0 | 0 | 0 | 0 | **0** |
| seq gaps (min ≠ 1 or max ≠ nv) | 0 | 0 | 0 | 0 | 0 | 0 |
| head mismatch (current_seq ≠ max, or current_published_at ≠ head's) | 0 | 0 | 0 | 0 | 0 | 0 |
| versions with no notice row | 0 | 0 | 0 | 0 | 0 | 0 |
| longest chain (tid) | 16 (8860958) | 201 (8813517) | 114 (252684) | 39 (179176) | 37 (1019458) | |

The w5 sdk-0.1 split: 194 Tenders carry an sdk-0.1 version (246 versions); 48 of them have 2+ versions, giving 56
pairs that touch sdk-0.1, all in order. Examples are 1016549, 1016835, 1017302, 1017370 and 1017608.

Example ids:
- 2+-version Tenders with a DÖE version: w1 8857005, 8857008, 8857015; w2 8811228, 8811234, 8811236; w3 248002, 248008,
  248011; w4 178002, 178005, 178010.
- DÖE+TED Tenders with an equal-instant pair: w1 8857540, 8860957, 8860958; w2 8811247, 8811253; w3 248028, 248038;
  w4 178137, 178142.

**Instant agreement (2b):**

| | i3 (248,001–253,000) | i2 (8,811,001–8,816,000) |
|---|---|---|
| versions | 11,703 | 10,926 |
| DÖE versions | 2,190 | 3,094 |
| DÖE `published_at` ≠ notice | **0** | **0** |
| DÖE `dispatched_at` ≠ notice | **0** | **0** |
| DÖE notice `published_at` NULL | 0 | 0 |
| non-DÖE versions | 9,513 | 7,832 |
| non-DÖE `published_at` / `dispatched_at` ≠ notice | **0 / 0** | **0 / 0** |

## 5. Violations

**None, in either class, in any window: DÖE 0 (0 date, 0 tie) and non-DÖE 0 (0 date, 0 tie).** No `first_viol_seq` is
non-null in any of the 23,574 rows, so nothing is left unexplained.

The Tenders the refold re-announced below the head are listed in full in `raw_rows_reordered.txt`, with their raw
rows. There are 114 inside the windows: w1 93, w2 8, w3 7, w4 5, w5 1. Each one:
- carries version-level change rows from runs a/b/c, meaning its compared prefix ended before the head, which is the
  shape a reorder takes;
- is present in the body and has a DÖE version.

Together they hold 562 versions, 448 pairs and 422 DÖE pairs, with 0 violations.

## 6. Spot checks through the public API (3 GETs, logged)

`spot.py` reads each served chain and checks the invariant on it:
- **Instant:** a date-only `published_at` is its civil day at 00:00Z (issue 418); a timed one is converted to UTC.
- **Source:** read from the publication_id shape. `NNNNNNNN-YYYY` is TED; a UUID with a `-NN` suffix is DÖE.
- **Cross-check:** the per-Tender counts from the chain statement (n_doe, nv, pairs_same_instant) match the served
  chains: 8/16/6, 10/20/7 and 3/6/0.

| tender | refold evidence | served chain | verdict |
|---|---|---|---|
| 8860958 (w1) | run c: seq 1 added, 2..16 changed (re-written from the front) | 16 versions, 8 DÖE + 8 TED, 6 equal-day pairs | in order, seq 1..16, head = last |
| 248260 (w3) | run c: seq 2..20 changed and a head correction | 20 versions, 10 DÖE + 10 TED, 7 equal-day pairs | in order, seq 1..20, head = last |
| 250367 (w3) | run b: seq 1 added, 2..6 changed and a head correction | 6 versions, 3 DÖE + 3 TED alternating | in order, seq 1..6, head = last |

The served chains confirm that the tie-breaker is **publication_id before notice id**:
- 8860958 seq 3–5 are DÖE on 2025-08-13. The publication_ids run 36391aa8… < 77e457ac… < 9aae70ae…, while the notice ids
  run 26731937, 26731797, 26731804, which is not ascending.
- 8860958 seq 6–8 are TED on 2025-08-14. The publication_ids run 00533712 < 00534123 < 00534259, while the notice ids
  fall: 24908303 > 24908065 > 24907344.

Each DÖE version is ordered before the TED version of the same procedure step, which comes 1–3 days later. In all 21
DÖE versions of the three chains (8 + 10 + 3), the served `published_at` is the same local day as the version's
`dispatched_at`, the IssueDate. That is what rule (a) gives when the IssueDate day is the latest candidate. The full
output is in `spot.txt`; the raw bodies are `t8860958.json`, `t248260.json` and `t250367.json`.

## 7. Raw bodies and request log

Files (sha256 prefix, bytes). Each chain body is one row per Tender; nothing was truncated.

| file | sha256 | size | rows |
|---|---|---|---|
| w1.out.json | b54694de75100b92 | 190,211 B | 4,041 |
| w2.out.json | c01da10d0297ff82 | 228,611 B | 4,851 |
| w3.out.json | fde10264789edc30 | 225,506 B | 4,891 |
| w4.out.json | 3ba8858bd6bf9914 | 225,743 B | 4,897 |
| w5.out.json | 7d2d43d176a78624 | 250,121 B | 4,894 |
| i3.out.json | 184e80d5c41655b4 | 359 B | 1 |
| i2.out.json | 452eb822f3e97415 | 361 B | 1 |
| t8860958.json | a6b6aa51fb972048 | 12,349 B | GET 200 |
| t248260.json | 07ab99097dde0fb9 | 15,744 B | GET 200 |
| t250367.json | ccf98d1ac1c01e93 | 7,693 B | GET 200 |

`i3.out.json`, verbatim:
`{"columns":[…],"row_count":1,"rows":[[248001,253000,11703,0,2190,0,0,0,null,null,9513,0,0,null,null]],"truncated":false}`

`i2.out.json`, verbatim:
`{"columns":[…],"row_count":1,"rows":[[8811001,8816000,10926,0,3094,0,0,0,null,null,7832,0,0,null,null]],"truncated":false}`

`requests.log`: every `/v1/sql` statement was sent once, through `/root/sq.sh` via `run.sh`. Each was preceded by an
idle-queue check (`aj.sh`: current null, queued []), and none was retried.

```
03:56:03Z sql w3 rc=0 0.95 s   03:56:26Z sql w4 rc=0 0.96 s   03:56:33Z sql w2 rc=0 1.30 s   03:56:42Z sql w1 rc=0 0.71 s
03:57:28Z get 8860958 200      03:57:31Z get 248260 200       03:57:34Z get 250367 200
03:59:32Z sql w5 rc=0 0.92 s   04:01:24Z sql i3 rc=0 0.36 s   04:01:30Z sql i2 rc=0 0.37 s
```

## 8. Context and what is left unmeasured

**Coverage.** The audit covers 5 windows: 23,574 of the live Tenders and 6,203 of the DÖE ones. The R3 runs stale-stamped
676,936, 126,812 and 147,825 Tenders. Most DÖE Tenders are single-version islands (run a: 636,635 islands), and an island
has no pair to check. sdk-0.1 multi-version chains are thin everywhere: w5 holds 48 of them (56 pairs). w3 and w4 hold a
few more that were not split out by profile (for example 249576, 178116 and 180331, all DÖE-only and in order). The
windows are a sample, not a census, and nothing here speaks for Tenders outside them.

**The ~3,800 reorder estimate is not measured here.** This audit shows that stored chains are in fold order. It does not
show how many versions moved. The change feed gives a related number that is not the same quantity. Across the three
runs, 470 Tenders carry version-level rows: 83 in a, 119 in b and 268 in c, with 2,465 rows in all (477, 548 and 1,440).
That count is the set of Tenders whose compared prefix ended before the head: a reorder, or a chain whose membership
changed (run c's 93 `tender/removed` merges and its 188 `tender/added` version rows).

Unit 1 predicted about 3,800 re-dated *versions* moving past at least one other. The two counts are in different units:
- versions moved, against Tenders re-announced;
- silent versions (facts identical to the previous version) get no row;
- folds of DÖE Tenders between the 10-19 deploy and the drain already took the new dates.

So they are not reconciled. On the 470 figure alone, the drain announced fewer below-head rewrites than the prediction
suggests. If that matters, a dry `refold` / compare tally by Tender would answer it. It is not a defect in the chains.

**Not checked:**
- served date *values* beyond the 3 spot checks. Values were checked only as version = notice agreement (2b) in w2/w3;
  the notice-side values were audited in `../post-drain/`;
- lot and fact re-election on reordered chains (579's six were read in run b);
- Tenders created by fetches after 02:35Z, which the daily fold orders by the same code.

Scripts in this directory: `gen.py`, `gen_i.py`, `tplan.sh`, `run.sh`, `get.sh`, `summ.py`, `spot.py`.
