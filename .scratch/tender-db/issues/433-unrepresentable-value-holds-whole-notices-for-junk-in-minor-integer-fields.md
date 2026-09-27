# 433 — `unrepresentable-value` holds 326 whole notices, and only ~44 are the "astronomical" class the terminal ledger names: the rest are junk in minor integer fields

Status: ready-for-agent — filed 2026-09-27 from the hourly audit (step 3). A decision is the next step, and it is
mine to take.
Kind: coverage (ingest: the eForms value parser's integer arm, `crates/ingest/src/eforms/value.rs:60`) and ledger
accuracy (`crates/model/src/dashboard.rs` `quarantine_terminal_policy`)
Relates to: 268 (CLOSED 2026-08-22: reclaimed the sub-cent 94 %, left "298 … the garbage class (10^50 integers)"
held by design), 303 (the terminal ledger: `unrepresentable-value` → `AcceptedInflow`, "the astronomical-magnitude
class"), 144 (its cause H: one BT-44 `_DEFAULT_VALUE_CHANGE_ME_`, decided "KEEP per ADR-0004", small), ADR-0004
(mapped-or-ignored; quarantine rather than guess), ADR-0010 (sub-cent amounts), 372 (the `withheld` quality-marker
precedent: a value the source did not usefully publish is marked, not fatal), 413 (the same whole-notice-hold shape
for an unclaimed block)

## Observed (2026-09-27, `/v1/sql` over `quarantine`, still-held rows)

`/metrics`: `tender_db_quarantine_reason_members{reason="unrepresentable-value"} 326`,
`tender_db_quarantine_actionable 326` — every one a real notice held whole. By field and value shape:

| field | shape | held | examples |
|---|---|---:|---|
| BT-44-Lot (prize rank) | text | **153** | `_DEFAULT_VALUE_CHANGE_ME_` (template placeholder, ~115), `erster Preis`, `siehe Pkt. 5.1.12`, `--` |
| BT-113-Lot (max. participants) | huge digits | 44 | `1000…0` (10^20–10^41), `9999999999999999999` — the astronomical class |
| BT-171-Tender (tender rank) | text / digits+symbol / decimal | 39 + 16 + 2 | `prima`, `PRIMA`, `si`, `aggiudicatario unico offerente`, `1°` |
| BT-661-Lot (recurrence) | text | 21 | `True`, `False` |
| BT-58-Lot (max. renewals) | decimal | 17 | `902269.04`, `2266730.05` — an amount typed into a count |
| BT-686-LotResult | decimal | 16 | `.00` |
| BT-803(d)-notice (dispatch date) | no zone offset | 13 | `2024-09-03` |
| BT-720-Tender, BT-33-Procedure, BT-145-Contract, BT-44-Lot (decimal) | other | 5 | |
| **total** | | **326** | |

Arrival: 298 held on 2026-08-22 (268's close) → 326 today, so about +28 in five weeks.

## What is wrong

1. **The ledger's rationale describes about 14 % of the population.** `quarantine_terminal_policy` accepts
   `unrepresentable-value` inflow as "held-by-design garbage … the astronomical-magnitude class". That class is the 44
   BT-113 rows. The other ~280 are a template placeholder, prose, booleans and decimals typed into integer fields. No
   ledger row covers them either: both `unrepresentable-value` entries in `quarantine-ledger.json` key on
   `%more than two fraction digits%` (sub-cent), which matches none of the 326.
2. **The cost is a whole notice per junk field.** A lot's prize rank, a tender's rank among bidders, a renewal count or
   a recurrence flag is not identity, money or a date the tender layer keys on. Yet one unparseable value holds the
   notice's buyer, lots, values and award — ADR-0004's "nothing of it is imported" — for a field the fold may not even
   read. 144 decided KEEP for ONE such row as "small"; the class is now 153 rows for BT-44 alone.
3. **BT-803(d) is different in kind**: a dispatch date without an offset is a real date. Since issue 418 the resolver
   can carry a date-only instant with its precision, so "no zone offset" may no longer need to be fatal. That needs
   checking, not assuming.

## What to decide (the next step)

- **(a) Soft-drop the non-key integer fields**: an unparseable value in a field on an explicit allow-list (BT-44,
  BT-171, BT-661, BT-58, BT-686, BT-113 …) is dropped with a parse-layer marker (the 372 `withheld` shape, e.g.
  `unparseable`), and the notice imports. Needs an ADR-0004 note: this is not a guess, the value is absent and says
  so. The list must be explicit — an amount, date or identifier stays fatal.
- **(b) Keep holding**, but make the ledger honest: a ledger row per shape above with its reason, and the
  `AcceptedInflow` comment rewritten to the measured composition.
- **(c) Split**: (a) for rank/count/flag fields, and (b) for BT-113's 10^40 (the only class where "the publisher
  published garbage" is the whole story) and for the 5 "other" rows until they are read.

My lean is (c), after reading one member per shape from the archive to confirm the field is what the BT id says.
Measure the fold's use of each field first: a field the fold never reads is the strongest case for a soft drop.

## Verify

    ssh -o BatchMode=yes root@zebreus.click 'curl -s --max-time 15 -H "Authorization: Bearer $(cat /root/tender-sql-token)" --data-binary "SELECT COUNT(*) FROM quarantine WHERE reason = '"'"'unrepresentable-value'"'"' AND reprocessed_at IS NULL AND skipped_at IS NULL AND detail NOT LIKE '"'"'BT-113-Lot:%'"'"'" https://tenders.zebreus.click/v1/sql'

- **done**: the non-astronomical remainder is either reclaimed (under (a)/(c): near 0) or every shape carries a ledger
  row naming its decision (under (b)), and the terminal-policy comment matches the measured composition
- **open**: `282` (read 2026-09-27: 326 held, 44 of them BT-113)
