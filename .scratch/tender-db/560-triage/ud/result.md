# 560 U-d: the coverage read for publication day 2026-10-19 (run 2026-10-20, after the daily chain)

**Verdict: PASS, no stop.** No lot is left uncovered by both paths: the residue is **0** in all three
segments, against the pre-deploy baseline of **257** (TED EUR-only 122, TED non-EUR 32, FTS 103; 2026-10-14).
Only 11 lots were silent at their own seq. All 11 sit on 2 TED CZK versions, and the 10-20 rederive's D5 rows
cover all of them. Both versions keep their elected value at fold-time rates (amendment (c)): the
proxy moved only because the next morning's rederive moved the figure. **These are the D5 cases the misfire
analysis predicted, not defects.**

## Window and conditions

- Publication day: 2026-10-19, `published_at` in [1792368000, 1792454400), `seq > 1`. This was folded entirely
  after the 10-17 10:50Z deploy of `900efeb`: TED by the 10-19 daily `project` (its rows are stamped 07:35:55Z),
  FTS by the 10-20 `project` 2669.
- D5 window: 10-20, [1792454400, 1792540800). The 10-20 chain ran fetch-rates 2667, then rederive-eur-recent 2668
  (07:41:18Z to 07:41:26Z, 4,824 D5 correction rows), then project 2669 (07:41:26Z to 07:47:38Z), then
  reveal-recheck 2670. The change cursor went from 958848784 to 958904203. **There was no refold and no drain
  that morning (amendment (d) holds).** project 2669's own compare wrote 4,609 seq-less correction rows inside the same
  window, so coverage is split below by job.
- Query: `q_ud_1019.sql`, byte-identical in shape to the baseline `scratchpad/d56x/adv/q_adv.sql`. Only the three
  timestamps differ (checked with `diff` after normalising them). It ran **once**, in 6.7 s.

## Plan probes (local, turso via the prebuilt store test binary `local_eqp_probe`, on a copy of d56x/adv/plan.db)

All reads seek. Not one has a `SCAN` of a base table.

- `plan_ud.out` (the U-d query): `SEARCH b USING INDEX tender_versions_published (published_at>=? AND published_at<?)`.
  Each correlated subquery after it is a seek: the `t` rowid, `lb`/`la` on `sqlite_autoindex_tender_version_lots_1`,
  `c`/`c2`/`c3` on `changes_entity (entity_kind=? AND entity_id=?)`, and `x`/`y` on
  `tender_version_amounts_version (tender_id=? AND seq=?)`. Then `USE SORTER FOR GROUP BY`.
- `plan_2.out` (`q_seqcov`, `q_silent`): the same seek shape.
- `plan_3.out` (`r_versions`, `r_amounts`, the first `r_changes`): seeks on PK or index for each named tender.
- `plan_4.out` (`r_rates`, `r_changes`): `currency_rates` on its PK range. `r_changes` uses `tender_version_lots lb CROSS JOIN changes c`, so lb drives and c seeks `changes_entity`.

## Result (q_ud_1019.out)

| src | pairs | me_pairs | me_silent_pairs | me_lots (silent) | of which non-EUR | D5-covered (cov15) | covany |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| fts | 191 | 126 | 0 | 0 | 0 | 0 | 0 |
| ted | 2,382 | 760 | 2 | 11 | 11 | 11 (all non-EUR) | 11 |

## Per segment: seq-row coverage, D5 coverage, residue

`q_seqcov.out` gives the seq-row half. It counts lots present on both sides of a version whose proxy moved (`me`)
and that have a lot row at `version_seq = seq`. That is A's path, together with any lot whose own facts moved.

| segment | me pairs | lots on both sides | seq-row covered | silent | D5-covered (10-20) | **residue (neither)** | baseline 10-14 residue |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| TED EUR-only | 137 | 376 | 376 | 0 | 0 | **0** | 122 |
| TED non-EUR | 623 | 3,035 | 3,024 | 11 | 11 | **0** | 32 |
| FTS (all non-EUR, GBP) | 126 | 216 | 216 | 0 | 0 (as expected) | **0** | 103 |
| **total** | 886 | 3,627 | 3,616 | 11 | 11 | **0** | **257** |

- **Bar (amendment (b)): residue 0 against a baseline of about 257. Pass.** Before the deploy, 10-14 had 1,268 silent lots
  behind a moved proxy (TED 1,165, FTS 103). On 10-19 there are 11.
- **FTS:** an FTS notice is folded the day after publication at the correct rate, so the D5 rederive never moves it,
  and D5 covering none of its lots is expected. **A now covers all 216 FTS lots at their own seq.** Before the deploy,
  all 103 were persistently uncovered.
- **TED EUR-only:** all 376 lots are seq-row covered. Before the deploy, 122 lots were persistently uncovered, because only 2 of 124 got D5.
- **D5 split by job** (`q_silent.out`, by `changed_at`): all 11 silent lots have a seq-less lot `changed` from the
  **rederive 2668** (rows at 07:41:23Z and 07:41:25Z, cursors 958850962 to 958851655, before project 2669 started). The
  same 11 lots *also* got a second seq-less `changed` from **project 2669's compare** (07:41:27Z, cursors
  958886424 to 958900458). The rederive alone covers them all.

## Residue versions: there are none. The 2 silent versions, read instead

With the residue at 0 there is no residue version to read. These are the only two versions with any silent lot,
`q_silent.out` sorted by count. Both are TED with CZK amounts, and both are max-election heads with no derivation:
`tenders.current_value_derivation` is NULL and the epoch is 4.

CZK rates (`r_rates.out`, units per EUR): 10-16 **24.323**, 10-19 **24.343**. The 10-19 ECB fixing did not exist
when the 10-19 morning fold ran (07:35:55Z), so the fold converted every 10-19 version at the 10-16 fixing.

### 1. Tender 878700, seq 360: 10 silent lots, all covered by D5

- seq 359 was published 10-16 and seq 360 on 10-19. The head is seq 361, also 10-19.
- **The elected figure is lot 2394899's `estimated_value` of 6,701,675,175 CZK at both versions.** It is the largest
  figure. The procedure figures at seq 360 (554,354,808 est. and 645,231,768 result, CZK) are smaller.
- **At fold-time rates nothing moved:** 6,701,675,175 / 24.323 = 275,528,314 on both sides.
- **The stored seq 360 value changed only through the 10-20 rederive:** 6,701,675,175 / 24.343 = 275,301,942, which
  equals `tenders.current_value_eur_cents`. That is the only reason the proxy MAX moved (275,528,314 → 275,301,942).
- Lot 2394897's own figure moved natively (188,278,272 → 554,354,808 CZK). It got its own rows `lot@360` and
  `lot@361 changed` at 07:35:55Z, so it is not silent. The other 10 lots kept their facts and got no seq row,
  which is correct under A. Each has a D5 row (rederive) and a project-compare row on 10-20.
- **Class: a D5 case under amendment (c). Not a stop.**

### 2. Tender 1139127, seq 3: 1 silent lot, covered by D5

- seq 2 was published 10-16 and seq 3 on 10-19. The head is seq 3.
- **The elected figure is the procedure `estimated_value` of 3,140,495,800 CZK at both versions.** It is the largest.
- **At fold-time rates nothing moved:** 3,140,495,800 / 24.323 = 129,116,301 on both sides.
- **After the rederive, seq 3 reads 3,140,495,800 / 24.343 = 129,010,221**, which equals `tenders.current_value_eur_cents`.
- The two lots present on both sides:
  - lot 3107130's own figure moved (15,264,000 → 98,196,459 CZK) and it got `lot@3 changed`;
  - lot 3107131 (242,388,000 CZK on both sides) is the silent one, and it has D5 and project-compare rows on 10-20.
  - lots 14691574 to 14691578 are new (`added` at seq 3).
- **Class: a D5 case under amendment (c). Not a stop.**

This is exactly the misfire the adversarial check predicted (objection 3). A TED non-EUR version whose previous
version is dated earlier is folded with the latest fixing on both sides, so the fold sees no move. The rederive
moves it the next morning, and D5 covers its lots with seq-less rows.

## Files (this directory)

- SQL:
  - `q_ud_1019.sql`, the prepared query, run once;
  - `q_seqcov.sql`, the seq-row coverage;
  - `q_silent.sql`, the silent versions with D5 coverage split by job;
  - `r_versions.sql`, `r_amounts.sql`, `r_rates.sql`, `r_changes.sql`, the bounded reads of the two silent versions.
- Outputs: the matching `.out` files. Each one ends with `EXIT=` and `wall=` lines.
- Plans: `plan_in_ud.sql` with `plan_ud.out`, and `plan_in_{2,3,4}.sql` with `plan_{2,3,4}.out`.

The first form of `r_changes.sql` used `group_concat`. The endpoint refused it with HTTP 400 because the function
is not allowed there, so nothing ran. It was replaced by the CROSS JOIN form, re-planned in `plan_4.out`, and that
version's output is the one in `r_changes.out`.

Budget: 8 `/v1/sql` requests, one at a time: 7 answered and 1 refused with a 400. There was no 408 and no retry.
