Issue 490 unit 1: measured gap between a naive SQL per-lot value and the value REST serves (prod, 2026-10-08)

**Result.** Across 6,297 current lots in five 500-tender windows, 1,267 have a lot-scoped amount in their current version. On 53 of those, a naive SQL `MAX(cents)` or `MAX(eur_cents)` per `lot_id` disagrees with the REST lot `value`. That is 4.2% of lots that have a lot amount, and 0.84% of all lots. Both naive forms disagree on exactly the same lots. In all 53 the REST row serves `value: null` and SQL serves a figure: none of these lots had a lower figure the pick would accept.

**The comparison is exact.** I rebuilt `summarise`'s pick in Python from the SQL rows. It skips withheld rows, applies `sentinel_amount` and the €100bn ceiling, and runs `ScalePartners` over the chain's amounts and lot awards up to the version. It matched the served REST value on 6,297 of 6,297 lots. No REST row's version differed from `tenders.current_seq`, so the running drain did not skew any window.

| window (tender ids) | era | lots | lots with a lot amount | disagree | causes: lots (tenders) | after `quality IS NULL AND cents > 0` |
|---|---|---|---|---|---|---|
| 224000–224499 | eForms, 2025 (ted 468, doe 23) | 1,222 | 539 | 8 | zero 6 (6), one-unit 1 (1), scale rule 1 (1) | 2 |
| 1000000–1000499 | eForms (ted 466, doe 19) | 1,245 | 333 | 8 | zero 6 (4), one-unit 2 (2) | 2 |
| 8784500–8784999 | FTS, 2026 | 1,046 | 350 | 37 | one-unit 37 (4 tenders: 8784625 ×18, 8784778 ×7, 8784682 ×6, 8784846 ×6) | 37 |
| 6941300–6941799 | TED r209, 2021 | 1,783 | **0** | 0 | — | 0 |
| 6290000–6290499 | TED r209, 2018–19 | 1,001 | 45 | 0 | — | 0 |
| **total** | | **6,297** | **1,267** | **53** | zero 12 (10), one-unit 40 (7), scale rule 1 (1) | **41** |

**Causes I asked about that did not occur:**
- **Withheld (-1):** 0. In every window, lot-scoped amounts are only `estimated_value` and `framework_maximum`. Withheld rows exist only at tender scope (10 and 8 rows in the two eForms windows).
- **All-nines / repdigit sentinel:** 0.
- **Over the €100bn ceiling:** 0.
- **Currency or tie differences:** 0. No lot had lot-scoped rows in mixed currencies, and none had an unconvertible row (`eur_cents` NULL). REST ranks by published cents across currencies, so this cause can only arise on mixed-currency lots, and there were none.
- **Tender-scoped fallback:** REST never falls back for the value; `summarise` filters on `s.lot_id IS NOT NULL`, unlike the deadline. I confirmed it: 0 REST values on lots without a lot-scoped amount. 3,176 of the 6,297 lots have no lot amount but their tender has one. A consumer who fell back to the tender figure would differ from REST on all of them. That is a design difference, not an election gap.

**What a stored value would fix.** If the consumer also filters `quality IS NULL AND cents > 0`, 41 disagreements remain: 40 one-unit and 1 scale-rule. Those 41 are the part SQL cannot reproduce without the fold's rule.

**Examples:**
1. **224156 LOT-0002** (lot 612131): `framework_maximum` EUR 40,000,000,000.00 (4,000,000,000,000 cents). Naive SQL serves €40bn and REST serves null, because the scale rule finds a partner: LOT-0001's €40,000,000.00 is exactly 10³ smaller. The tender head is 4,000,000,000 (€40m), so head and REST agree and only SQL is out. This is issue 471's known exhibit, and the window was chosen to contain it.
2. **FTS 8784625**: 18 lots, each `estimated_value` GBP 1.00 (100 cents, `eur_cents` 120). Naive SQL serves £1.00 / €1.20 per lot; REST serves null (the one-unit sentinel). The tender carries its own £500m estimate at tender scope.
3. **224024 LOT-0001** (lot 611799): `estimated_value` EUR 0. Naive SQL serves 0; REST serves null (the zero sentinel). Same shape at 224120, 224138, 224144, and 1000082 LOT-0001..0003.
4. **224476 LOT-0001** (lot 612945): `estimated_value` and `framework_maximum` both EUR 1.00. REST serves null.
5. **Both surfaces agree on a likely error, 8784848 lot 1**: `estimated_value` GBP 60,000,000,000.00. Naive SQL, REST and the tender head (7,203,054,094,936 EUR cents) all serve it. It is issue 492's ×100 case, below the k ≥ 3 rule.

**Era finding.**
- **r209, 2021 window:** it projects no lot-scoped amounts at all, in any version. Lot figures there exist only as `tender_version_lot_results.awarded_cents` (8,828 rows), so REST serves null for all 1,783 lots.
- **6941544**, issue 471's ×1000 r209 exhibit, is in this window. Its LOT-1 REST value is null, but `v_lot_results` / `v_awards` still serve its £80bn award (8,000,000,000,000 GBP cents) raw. Lot awards are never elected, and no lot-value column on `v_lots` would cover them.
- **r209, 2018 window:** only 45 of 1,001 current lots carry a lot amount, though 149 lot-scoped rows exist across all versions. Lot estimates mostly sit in superseded versions.

**Read-time cost today.** Of 515 tenders whose current version has lot amounts, 6 reach the €1bn gate that makes `summarise` load the chain on every read: 224156 and five FTS tenders (8784520, 8784744, 8784848, 8784856, 8784940).

**Method and load.** Five single-table range reads per window, all through `/v1/sql` with `sq.sh`. Each was a 250-tender chunk on `tenders`, `lots`, `tender_version_lots`, `tender_version_amounts` and `tender_version_lot_results`, with no joins. No response was truncated, none errored, and none hit a 408.
- **Plans:** read locally with `target/debug/plan-probe` on a scratch copy of an Oct-1 schema DB. All are index range seeks:
  - `SEARCH tenders USING INTEGER PRIMARY KEY`
  - `SEARCH lots USING INDEX sqlite_autoindex_lots_1 (tender_id>=? AND tender_id<=?)`
  - `SEARCH tender_version_lots USING INDEX sqlite_autoindex_tender_version_lots_1 (tender_id>=? AND tender_id<=?)`
  - `SEARCH tender_version_amounts USING INDEX tender_version_amounts_version (tender_id>=? AND tender_id<=?)`
  - `SEARCH tender_version_lot_results USING INDEX sqlite_autoindex_tender_version_lot_results_1 (tender_id>=? AND tender_id<=?)`
- **REST:** paged `/v1/lots?cursor=<lot_id>&limit=500` over each window's lot-id run, plus `?tender=` for 4 tenders whose lots sit outside it. About 20 requests in all, at about 4 ms each.
- **Totals:** about 67 SQL requests. I edited no repo files.

Files are in /tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/m490:
- data/{eforms,eforms2,fts,r209,r209b}.sql.json — the raw SQL rows
- data/*.rest.json — the REST rows
- data/*.cmp.txt — the comparison output
- scripts/sqlpull.py, restpull.py, compare.py — the scripts
- q.sql — the planned statements