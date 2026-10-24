# 539 post-drain chains: adversarial check of `result.md` (2026-10-24, after 04:05Z)

**Verdict: CONFIRMED, with discrepancies.** The headline holds. The SQL tests the fold's comparator exactly, on the stored
columns. Both engines reproduce every plan byte for byte. My own re-tabulation of the five bodies gives 0 violations in
26,639 pairs, and every table number in `result.md` §4 matches.

One finding is material to how the evidence reads, though not to the 0-violation verdict. The 93 w1 Tenders
(8,860,949–8,861,041), spot check 8860958 included, are **new Tenders minted by run c**. Run c merged 93 Tenders away
(`tender/removed`), and these 93 replaced them. They are not existing chains that the refold re-sorted below the head. So
only **21** of the "114 reorder-shaped" Tenders are below-head rewrites of existing chains: 124 versions, 103 pairs, 97
of them DÖE. w1 has none. All 21 are in order.

Method: everything was done locally. No box statements, no GETs, no cargo. One slip: early on I ran a single read-only
`git log --oneline -1` before I registered the no-git rule. Nothing else touched git. The turso plans were re-run with the
local `b6u6kit/plan-probe` binary (sha256 50a8094e…), on a scratchpad copy of `plan.db`. Scripts and copies are in
`/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/skeptic539/`.

## (a) The fold's ordering rule, re-derived, against the SQL

Line numbers are the current working tree's. `project.rs` was modified at 04:05:49Z, after the audit, so the auditor's
project.rs numbers are shifted by 10–30 lines; the cited code is the same. The canonical.rs lines (21038, 24247, 24113,
36310, 42710) and the read.rs lines (3390, 3499–3505) match exactly.

**Chain.** One plan group is one Tender.
- Streaming path: `group.notice_ids` arrive in `next_plan_batch`'s `ORDER BY group_key, published_at, source_rank,
  publication_id, notice_id` (canonical.rs:21038). `apply_plan_batch` (project.rs ~3947) only filters ids that have no
  state, which never reorders.
- Bucketed path: `rows.sort_by(sort_key)` (project.rs:4028). `BucketRow::sort_key` (:4554) is the same 5-tuple.
- `fold` (:6985–7156) pushes exactly one `TenderVersion` per chain entry, in chain order. Its loop has no `continue`,
  `break` or `return`.
- `apply_tender_tx` writes `seq = i + 1` (canonical.rs:24247). A stale Tender keeps 0 versions (:24113). On-mode
  compare corrects the compared prefix in place: same seq, same notice, so the new values come in new-fold order.

**Key fields, and what is stored for each.**
- `published_at`: `notice_instants(&notice.source, parsed).0.unwrap_or(0)`. The plan side computes it at :6892, the
  state side at :5982, both on the `normalise_de1`'d parse. `notice_stamps` (:12727–12734) runs every stamp through
  `civil_day`: a timed value is its exact UTC second, and a date-only value is its civil day's 00:00Z. `.or(dispatched)`
  is the instant fallback only. `dispatched_at` is never in the key.
- `write_version` (canonical.rs:42710) stores `v.published_at`, which is the sort value itself.
- `source_rank` (:6964): `"ted"` ⇒ 1, everything else ⇒ 0, read from `notices.source`. DÖE is `"doe"`
  (`DOE_SOURCE`).
- `publication_id` is `notice.publication_id` on both sides. In the plan table it is TEXT, and in `tender_versions` it
  is `TEXT NOT NULL` in a STRICT table with no collation, so BINARY. Rust `&str` Ord is the same bytewise order.
- `notice_id` is `caused_by_notice_id`.

**SQL against the rule.**
- `first_viol_seq` and `vd_*`/`vt_*` together are the exact lexicographic `key_a > key_b` over
  `(a.published_at, na.source='ted', a.publication_id, a.caused_by_notice_id)`.
- An equal key is impossible because of `UNIQUE (tender_id, caused_by_notice_id)`, so "not strictly ascending" and
  "greater" are the same test.
- The date/tie split and the DÖE/non-DÖE split partition the same predicate.

| point | code | SQL | misclassified pairs |
|---|---|---|---|
| tie-breaker | rank, then pubid, then notice id | same, same order | none |
| NULL / no instant | `unwrap_or(0)`, stored 0 (`published_at INTEGER NOT NULL`, STRICT) | compares the stored 0 | none |
| date-only vs timed | the key is the anchored integer (date-only = civil day 00:00Z) | compares the same stored integer | none |
| offsets | not in the key | not used | none |
| NULL `na.source` | `notices.source NOT NULL` | would make the predicate NULL, so not counted, and the pair would drop out of both DÖE classes | 0 here: `n_nonotice = 0` in all 23,574 rows |
| text order | Rust `&str` Ord | BINARY memcmp (STRICT TEXT) | none |

So, measured against the code and on stored columns, the SQL is neither looser nor stricter.

**Kept prefix.** A non-stale Tender's kept prefix can carry pre-539 instants. Rule (a) only moves a DÖE instant later
(the latest candidate, raised to the IssueDate's day). For the non-negative offsets DÖE publishes in, a stale prefix
value is therefore never above its re-derived value. That prefix cannot make the stored order fail the SQL. Every
Tender with a re-dating DÖE version was stale-stamped by R3 in any case.

**Limits of what the chain check shows.** None of these is a misclassification.
1. It compares stored values with stored values. A chain left in pre-539 order **with** pre-539 instants would also
   pass. Only the instant check (version = notice, which the notice-side audit `../post-drain/` shows carries the new
   rule) closes that gap, and it covers w2 and w3 only. In w1, w4 and w5, "re-sorted by the new DÖE dates" rests on the
   chain check, plus the 8860958 spot check's DÖE dates landing on their IssueDate day.
2. A seq gap would hide the pair across it. There are 0 gaps.
3. Served shape: `tender_detail` renders each version in its notice's offset. A date-only `2025-08-14` (00:00Z) is
   therefore served before a timed `2025-08-13T23:30-05:00` (04:30Z). That is the code's rule, and the check rightly
   doesn't flag it. All 42 served values in the three spot chains are date-only, so the spot checks never exercised
   mixed precision.

## (b) Plans

- The sha256 of all 7 local `.sql` files matches `requests.log`: w1 17e3e314…, w2 e719b01b…, w3 5a817cd8…,
  w4 dba77788…, w5 57748428…, i3 29c72211…, i2 f58487b9…. Each `.sql` mtime is earlier than its send, and each plan
  file is newer than its `.sql`.
- w1, w2 and w4 differ from w3 only in the `BETWEEN` line. w5 is w3 plus the two sdk-0.1 columns and new bounds. i2
  differs from i3 only in its bounds.
- `plan.db` is byte-identical to `539-triage/plan.db` (sha256 a821689b…). It holds no rows and no `sqlite_stat*`, and
  its index set matches the code's `CREATE INDEX` list. `notices_unprojected` is absent, as the auditor noted.
- Re-running python sqlite3 3.45.1 gives **identical** plan lines for all 7 statements. Re-running turso plan-probe gives
  **identical** `PLAN` lines for all 7.
  - Every step is a SEARCH: t by rowid range; a by `autoindex_1 (tender_id=?)`; b by `autoindex_1 (tender_id=? AND
    seq=?)`; na and nb by PK.
  - There is no SCAN, no TEMP B-TREE and no IN-list.
- `PRAGMA index_info` confirms `sqlite_autoindex_tender_versions_1` is `(tender_id, seq)` and `_2` is
  `(tender_id, caused_by_notice_id)`.
- The auditor's turso claims check out.
  - `BETWEEN` prints as `(rowid=?)`, the same as `=`.
  - Without `INDEXED BY`, turso seeks b on `_2 (tender_id=?)`, which reads nv² rows. That justifies the pin.
  - The pin changes cost only, never results.
- The read is bounded by ≤ 5,000 tender rowids per statement. The largest chain read was 201 versions (8813517).

## (c) Re-tabulation (my script, from the raw bodies)

Every body has `truncated: false` and `row_count` equal to its rows. tids are strictly ascending and inside the window.
The body sha256 prefixes and sizes match `result.md` §7.

| | w1 | w2 | w3 | w4 | w5 | total |
|---|---|---|---|---|---|---|
| Tenders | 4,041 | 4,851 | 4,891 | 4,897 | 4,894 | 23,574 |
| versions | 4,660 | 10,926 | 11,703 | 11,471 | 11,453 | 50,213 |
| 2+ versions | 347 | 1,786 | 3,169 | 3,193 | 3,109 | 11,604 |
| with a DÖE version | 1,094 | 1,553 | 1,195 | 1,180 | 1,181 | 6,203 |
| 2+ and DÖE | 280 | 1,098 | 1,043 | 1,033 | 1,018 | 4,472 |
| DÖE and TED | 278 | 1,097 | 1,008 | 1,003 | 970 | 4,356 |
| DÖE-only, 2+ versions (pairs) | 2 (2) | 1 (1) | 35 (55) | 30 (47) | 48 (56) | 116 (161) |
| pairs | 619 | 6,075 | 6,812 | 6,574 | 6,559 | 26,639 |
| DÖE pairs / non-DÖE pairs | 512 / 107 | 4,019 / 2,056 | 2,897 / 3,915 | 2,651 / 3,923 | 2,719 / 3,840 | 12,798 / 13,841 |
| equal-instant pairs (in DÖE Tenders) | 51 (44) | 926 (412) | 411 (173) | 352 (145) | 335 (193) | 2,075 (967) |
| cross-source pairs | 490 | 3,855 | 2,796 | 2,573 | 2,612 | 12,326 |
| versions neither DÖE nor TED | 611 | 607 | 0 | 0 | 0 | 1,218 |
| vd_doe / vt_doe / vd_non / vt_non | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | **0/0/0/0** |
| non-null `first_viol_seq` | 0 | 0 | 0 | 0 | 0 | **0** |
| seq gaps / pairs ≠ nv−1 / head mismatch / no-notice | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0 |
| longest chain | 16 (8860958) | 201 (8813517) | 114 (252684) | 39 (179176) | 37 (1019458) | |

w5 sdk-0.1: 194 Tenders and 246 versions. 48 of those Tenders have 2+ versions, and all 48 have sdk-0.1 pairs (56 pairs).

Instant bodies:
- i3: 11,703 versions, 2,190 DÖE and 9,513 non-DÖE, 0 mismatches.
- i2: 10,926 versions, 3,094 DÖE and 7,832 non-DÖE, 0 mismatches.
- Both totals equal the w3 and w2 bodies' `nv` and `n_doe` sums.

All of `result.md` §4's example ids hold their stated property. The logic test DB, re-run, returns the expected cell for
each of its 8 synthetic Tenders.

## (d) Windows against the change feed (`551-lot-sum/b6/u9/segments/w539-{a,b,c}`)

The chunk sha256s match their manifests: a 4/4, b 1/1, c 2/2, all `complete`.

| window | changed-live ids a/b/c | in body, with DÖE | seq-less (D3) Tenders | version-level Tenders | of which minted in run c |
|---|---|---|---|---|---|
| w1 | 0 / 0 / 93 | 93 / 93 | 0 | 93 (c 93) | **93** |
| w2 | 1 / 27 / 36 | 64 / 64 | 64 | 8 (b1, c7) | 0 |
| w3 | 44 / 15 / 11 | 70 / 70 | 70 | 7 (a1, b2, c4) | 0 |
| w4 | 28 / 15 / 7 | 50 / 50 | 50 | 5 (a3, b1, c1) | 0 |
| w5 | 62 / 11 / 10 | 83 / 83 | 83 | 1 (c1) | 0 |

The 114 / 562 versions / 448 pairs / 422 DÖE pairs reproduce. Every changed-live id is in its body with a DÖE version,
and no removed id is in any body.

**Why the w1 93 are new mints and not re-sorts:**
- Run c has exactly 93 `tender/removed` rows. Their ids are all older, 8,680,277–8,848,708, and issue 539 records them
  as "Tenders merged away by re-dated links".
- The 93 ids are contiguous and are the highest in `tenders`: w1's max tid is 8,861,041 and nothing above it exists.
  None of them appears in run a or run b.
- Their first change cursors rise monotonically with id. They are interleaved with 2,089 older Tenders' rows, which is
  the pattern ids allocated in fold order produce.
- Each one's first row is seq 1 `added`, and **none has a seq-less D3 correction row**. Every existing corrected Tender
  in w2–w5 has one; 250367 and 248260 do as well.
- Run c's changed-live count is 2,289. That is the 2,196 corrected Tenders plus these 93.

So `result.md`'s "re-announced from seq 1", "its compared prefix ended before the head", and spot check 8860958 "run c
re-wrote it from seq 1" are wrong. These chains had no stored chain and no compared prefix. Their order still shows that
the fold sorts the new dates and interleaves DÖE with TED correctly, but only for a fresh write.

The genuine below-head rewrites are 21 Tenders. Two of them are seq-1 reorders of existing chains, 250367 (run b) and
252837 (run c). 250367 fits the sdk-0.1 inherited-request pattern: the old date of the later DÖE notice equalled
version 1's date, and publication_id ordered it first.

The coverage claim is otherwise honest. It is stated as a sample, not a census, with no older non-DÖE range and thin
sdk-0.1 coverage. Within the windows, DÖE Tenders are mostly multi-version (4,472 of 6,203). The windows are not
representative of the global island-heavy population, and `result.md` §8 says so.

## (e) Violations

There are none to explain. All eight violation counters are 0 in every row, `first_viol_seq` is null in all 23,574 rows,
and the instant mismatches are 0/0/0/0. Re-checked independently, the three served chains have 0 violations,
contiguous seq, top-level date equal to the last version's, and counts that match their body rows:
- 8860958: 16 versions, 8 DÖE / 8 TED, 6 equal-instant pairs;
- 248260: 20 versions, 10/10, 7;
- 250367: 6 versions, 3/3, 0.

21 of 21 DÖE versions fall on their IssueDate's local day. Every equal-instant pair in those chains is decided by
publication_id within one source. A notice-id-first rule would reverse 3 pairs in 8860958 and 1 in 248260.

## Discrepancies (none changes the 0-violation verdict)

1. **Material characterization.** The w1 93, spot check 8860958 included, are run-c mints, not below-head re-sorts.
   - Below-head evidence is 21 Tenders / 103 pairs (97 DÖE), not 114 / 448.
   - w1 contains no below-head rewrite.
   - Of the 3 spot checks, only 248260 and 250367 exercise a re-sort.
   - The same correction applies to `result.md` §8's 470: run b's 54 and run c's 188 Tenders have seq-1 rows, meaning no
     compared prefix, and 93 of run c's 188 are mints.
2. **Arithmetic.** `result.md`'s opening says the instant check "covers 16,210 versions in 2 of the windows". The correct
   figure is **22,629** (11,703 + 10,926, which also equals 5,284 + 17,345). 16,210 is 5,284 + 10,926.
3. "2,075 pairs share an instant, so the tie-break decided them" is set next to the DÖE counts. Only **967** of those
   pairs are in Tenders with a DÖE version, and the body cannot say how many touch a DÖE version.
   - The rank tie-break (DÖE against TED at an equal instant) is never observed in served data and cannot be counted
     from the bodies. Only the synthetic logic test covers it.
4. The "cross-source pairs (DÖE↔TED)" row also counts pairs involving the 611 (w1) and 607 (w2) versions that are
   neither DÖE nor TED.
5. Window rationale counts:
   - w2's "65 changed heads" is 64 live plus the removed 8,811,451.
   - w5's "63 run-a changed heads": I count 62.
   - w4's "DÖE-only sdk-0.1 chains": only 30 of its 1,033 multi-version DÖE Tenders are DÖE-only, and 1,003 of 1,180
     carry TED.
6. In the spot checks, "the TED version … comes 1–3 days later" does not hold for 248260: seq 19 → 20 is 4 days
   (2025-08-14 → 2025-08-18).
7. The project.rs line numbers no longer match the working tree, which another agent modified at 04:05:49Z, after the
   audit. The code content is as cited.
8. Scope, not error: the chain check is stored against stored. The new-rule instants are verified only in w2 and w3
   through i2/i3.
