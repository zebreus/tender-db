## Ground rules held

- **Budget and errors:** 25 requests, all through `ssh root@zebreus.click /root/sq.sh`, run one at a time. There were 0 error bodies and 0 HTTP 408s, and my helper refused any body without a `rows` key.
- **Shape:** no joins, no IN-lists, no repo edits, no cargo.
- **Tables:** every table I read is on the public allow-list: tenders, tender_versions, notices, tender_version_{texts,classifications,dates,amounts,parties,lots}, organizations, notice_codes, notice_ids. The reads need no table outside the surface.
- **Indexes:** `sqlite_master` is not queryable through /v1/sql. So index presence (`tender_version_parties_org_role`, `organizations_name_norm_id`, `tender_versions_published`) is inferred from timings only.
- **Wall times:** measured end to end. The floor is 1.66–1.72 s (ssh, wstunnel and HTTP on trivial reads 17, 20 and 23), and the response body carries no server timing. Server time is roughly wall minus 1.7 s. The slowest read was 3.15 s wall (texts, about 1.5 s on the server); everything else was 2.4 s or less.

## Tender-id layout (reads 1 and 2)

Max id is 8,802,504. The 2026-08-15 rebuild assigned ids in group_key order, so blocks are ordered as follows:

| Ids (approx.) | Contents | Order inside the block |
|---|---|---|
| 2 to ~1.1M | UUID procedure keys (TED eForms, DÖE UUID, merged) | Lexical by UUID, so random with respect to time |
| ~1.1M to ~1.95M | `island:<notice_id>` | Notice-id order. DÖE islands are time-clustered: ids 1,500,000–1,502,499 were all published 2024-09-09..25 |
| ~1.95M to ~7.93M | `ojs:` legacy keys | Chronological |
| ~7.93M and up | Created after the rebuild (2026-09-13 reparse, 2026-09-30 FTS, daily ticks) | Ingestion order |

- The boundaries are approximate. Probe points: id 1,000,000 is UUID `db0f…`, 1,500,000 is `island:26448906`, 2,000,000 is `ojs:1995`.
- Read 2 is the layout probe: an 18-arm UNION ALL of `SELECT id,source,substr(procedure_key,1,40),island_notice_id,kind,created_at,current_seq,current_published_at FROM tenders WHERE id BETWEEN p AND p+1`. It returned 34 rows in 1.72 s.

## (1) Tenders with versions from both sources

**Read 3:** `SELECT tender_id, seq, caused_by_notice_id, publication_id, published_at, dispatched_at, notice_subtype, original_lang FROM tender_versions WHERE tender_id BETWEEN 600000 AND 602499`
- 5,861 rows, 2.36 s. Seeks the primary key.

**Read 13:** `SELECT id, source, procedure_key, island_notice_id, kind, current_seq, current_published_at, current_deadline, current_value_eur_cents, current_title FROM tenders WHERE id BETWEEN 600000 AND 602499`
- 2,452 rows, 2.32 s.

**Telling the source from publication_id:**
- DÖE ids look like `<uuid>-<n>` or `<digits>-<n>` (for example `22690444-1`). TED ids look like `\d+-\d{4}`.
- Read 4 checked 25 TED-shaped and 25 DÖE-shaped ids against `notices.source`: 50 of 50 matched.
- `tenders.source` agrees with version composition on 2,452 of 2,452: source='doe' means DÖE-only, and every merged tender is labelled 'ted'.
- Result for this range: 477 both-source (19.5%), 1,883 TED-only, 92 DÖE-only.
- **Caveat:** FTS release ids (`083685-2026`) have the TED shape. They never appear in a UUID-block range, but they do in date windows, so confirm TED candidates there with `tenders.source`.

**Read 4:** 50 primary-key probes as a UNION ALL of `SELECT id, source, profile, publication_id FROM notices WHERE id = X` (4.3 KB). 50 rows in 1.69 s.

## (2) Per-version satellites for the same 2,500-tender range

| Read | SQL | Rows | Wall |
|---|---|---|---|
| 5 | `SELECT tender_id, seq, lang, value FROM tender_version_texts WHERE tender_id BETWEEN 600000 AND 602499 AND field = 'title' AND lot_id IS NULL` | 5,896 | 3.15 s |
| 6 (don't use) | CPV main without `lot_id IS NULL` | 10,000, truncated (lot rows) | 2.35 s |
| 7 | `SELECT tender_id, seq, code FROM tender_version_classifications WHERE tender_id BETWEEN 600000 AND 602499 AND +scheme = 'cpv' AND field = 'main' AND lot_id IS NULL` | 5,833 | 2.16 s |
| 8 (don't use) | Raw deadline rows | 10,000, truncated (eForms BT-131 is per lot: only 27 procedure-level rows) | 2.23 s |
| 9 | `SELECT tender_id, seq, count(*) AS n, min(utc_seconds) AS dl_min, max(utc_seconds) AS dl_max, sum(lot_id IS NULL) AS n_proc FROM tender_version_dates WHERE tender_id BETWEEN 600000 AND 602499 AND field = 'submission_deadline' GROUP BY tender_id, seq` | 4,164 | 2.07 s |
| 10 | `SELECT tender_id, seq, count(*) AS n, sum(lot_id IS NULL) AS n_proc, max(CASE WHEN lot_id IS NULL THEN cents END) AS proc_cents, max(CASE WHEN lot_id IS NULL THEN currency END) AS proc_cur, max(CASE WHEN lot_id IS NULL THEN eur_cents END) AS proc_eur, sum(CASE WHEN lot_id IS NOT NULL THEN eur_cents END) AS lots_eur, sum(quality IS NOT NULL) AS withheld FROM tender_version_amounts WHERE tender_id BETWEEN 600000 AND 602499 AND field = 'estimated_value' GROUP BY tender_id, seq` | 2,361 | 2.11 s |
| 11 | `SELECT tender_id, seq, lot_id, role, organization_id, mention_notice_id FROM tender_version_parties WHERE tender_id BETWEEN 600000 AND 602499 AND role LIKE '%uyer%'` | 6,757 | 2.36 s |
| 12 | `SELECT tender_id, seq, count(*) AS lots FROM tender_version_lots WHERE tender_id BETWEEN 600000 AND 602499 AND kind = 'Lot' GROUP BY tender_id, seq` | 5,861 | 2.09 s |

- **The `+scheme` in read 7 is required.** Without it the `(scheme, code)` index can drive the plan and walk every CPV row in the corpus (the issue-421 trap).
- **Buyer roles in read 11:** 6,636 'Procedure-Buyer' and 121 'buyer'.
- **Range size:** the satellite tables run about 2.4–2.8 rows per tender, so 2,500 tenders is a safe range size; above about 3,500 the parties read hits the 10,000-row cap. If a response comes back `truncated`, split the range.

**Signals on the 477 positives** (computed client-side, `pairs.py`):

| Signal | Agree | Other |
|---|---|---|
| Title | 477/477 | — |
| Main CPV | 477/477 | — |
| Lot count | 477/477 | — |
| Buyer org id | 476 | 1 differ |
| Deadline | 290 | 187 missing |
| Estimated value | 96 | 381 missing |
| Published within 7 days | 474 | 3 outside |

## (3) Unmerged DÖE Tenders and their buyers

**Read 14:** `SELECT id, procedure_key, island_notice_id, kind, current_seq, current_published_at, current_deadline, current_value_eur_cents, current_title FROM tenders WHERE id BETWEEN 1500000 AND 1502499 AND +source = 'doe'`
- 2,500 rows, 2.15 s. All are islands: procedure_key NULL, kind 'procedure', current_seq 1.
- The `+source` keeps `tenders_source_id` / `tenders_island` out of the plan, which would otherwise risk walking the whole source slice (the parse_state trap).

**Read 15:** buyers for the same range, same SQL shape as read 11.
- 2,504 rows, 2.16 s. Roles: 2,465 'buyer' (sdk-0.1) and 39 'Procedure-Buyer'. 1,351 distinct orgs.

**DÖE-only UUID tenders** need no extra read: filter read 13 on `source = 'doe'` (92 in that range).

**Above-threshold filter for islands:**
- Read 23 discovered the fields for one island notice: a UNION ALL across notice_codes, notice_ids and notices by PK; 12 rows, 1.66 s. It showed `SDK01-RegulatoryDomain`.
- Read 24: `SELECT notice_id, code FROM notice_codes WHERE notice_id BETWEEN 26448906 AND 26454565 AND field_id = 'SDK01-RegulatoryDomain'` returned 2,058 rows in 2.44 s.
- Of the 2,500 islands, 1,851 carry the field: de-vob 1,342, de-vol 367, de-uvgo 74, and `32014L0024` (EU directive) 68, which is 3.7%.
- 649 islands have no RegulatoryDomain row. They need the eForms-DE BT-01 field id, which I did not discover.

**Read 17** (8 organization PK probes, 1.68 s) shows the top island buyers are provisional with country NULL and no identifier. Example: 16006 "Vermögens- und Hochbauverwaltung Baden-Württemberg".

## (4) TED tenders of a buyer org near a date

There is a seekable index: `tender_version_parties_org_role (organization_id, role, tender_id)` serves the lookup from the index alone.

| Read | SQL | Rows | Wall |
|---|---|---|---|
| 16 | `… WHERE organization_id = 16006 AND role = 'Procedure-Buyer' UNION ALL … role = 'buyer'` | 10,000, truncated | 2.04 s |
| 21 | Same shape for org 254, `role = 'Procedure-Buyer'` | 10,000, truncated (all UUID block) | 2.28 s |
| 25 (use this) | `SELECT DISTINCT tender_id FROM tender_version_parties WHERE organization_id = 254 AND role = 'Procedure-Buyer'` | 4,448, complete | 1.84 s |

- In read 16 the 'Procedure-Buyer' arm appears to return 0 rows, so org 16006 has no TED eForms tenders under that id.
- To page a large buyer, add `AND tender_id > cursor`; the index keeps that a seek.

**The island-side org is a different org from the TED-side one.** Reads 19 and 20 show it:
- Island org 23996442 'db infrago ag ? geschäftsbereich fahrweg (bukr 16)' is provisional.
- TED-side org 254 'db infrago ag – geschäftsbereich fahrweg (bukr 16)' is DE with a national id.
- So blocking goes through names. Read 19: `SELECT … FROM organizations WHERE name_norm >= 'db infrago ag' AND name_norm < 'db infrago ah'` (2 prefix arms) returned 24 rows in 1.71 s, including both orgs.

**Restricting to dates:** ids in the UUID block carry no date, so date filtering needs its own read.
- Read 18: `SELECT tender_id, seq, caused_by_notice_id, publication_id, published_at FROM tender_versions WHERE published_at >= 1725926400 AND published_at < 1726012800` returned 4,155 rows (one day) in 2.27 s.
- One day split as: 2,949 with the TED/FTS id shape, 644 DÖE UUID, 562 DÖE numeric. One request per day fits under the cap.
- Intersect that client-side with the org's tender list.

**Scattered candidates:**
- Read 22: a 300-arm UNION ALL of `SELECT id, source, current_seq, current_published_at, current_deadline, current_value_eur_cents, current_title FROM tenders WHERE id = X`. 45.5 KB, 300 rows in 2.03 s.
- The request body cap is 64 KiB, so about 400 arms fit per request.
- This is a new shape: one equality per arm, not an IN-list. It ran fine at 50 and 300 arms, but you may want to confirm it before the sampler relies on it.

## Sampler plan and sample sizes for 450 requests

| Stage | What it reads | Requests | Yield |
|---|---|---|---|
| A. Positives | 4 random 2,500-id UUID-block ranges × 8 reads (3, 13, 5, 7, 9, 10, 11, 12), plus notices PK probes on the DÖE versions to label each with its DÖE profile (about 3 per range) | ~44 | ~1,900 merged pairs, ~370 DÖE-only UUID tenders |
| B. Negatives | Packed DISTINCT reverse lookups for about 300 DÖE-side buyer orgs (~12); tenders PK probes to keep ±30 days (~10); satellites of about 3,000 kept negatives as `(tender_id = X AND seq = Y)` arms (~40) | ~65 | ~3,000 same-buyer different-procedure pairs |
| C. Target: one recent month | Day windows for the month ±7 days (44); notice_codes range reads for above-threshold (~8); DÖE satellites and buyer names (~10); name-prefix blocking (~5); TED-side reverse lookups (~30); TED candidate satellites (~30); `tenders.source` checks to separate TED from FTS (~5) | ~130 | ~600–1,000 above-threshold candidates |
| D. Hand reads | 30 hand reads plus boundary review | ~10 | — |
| E. Reserve | Truncation splits and re-reads | ~60 | — |
| **Total** | | **~310 + 60 = ~370** | |

Pacing: run serially at 300 per hour or less, because the 960/hour token limit is shared with the owner. Stop on the first 408. Treat a body without `rows` as a failure and print the failure count.

## Calibration warning

The UUID-merged positives look like the same eForms-DE notice forwarded to TED. The unmerged target is mostly sdk-0.1 islands, which publish under different buyer orgs and possibly different text. So "no false merge on the positives" does not establish precision on the target. Stage A has to stratify positives by DÖE profile, and an sdk-0.1 positive set will probably need hand labelling. This is the "does the sample contain the phenomenon" check from prod-box-reads.

Files are in .scratch/tender-db/481-dedup/ (data/, scripts/; working copy was the session scratchpad):
- raw/01–25-*.json — the raw response bodies
- reqlog.jsonl — exact SQL, wall time and rows per request
- sq.py — the logged single-request helper
- pairs.py — computes the positive-pair signals