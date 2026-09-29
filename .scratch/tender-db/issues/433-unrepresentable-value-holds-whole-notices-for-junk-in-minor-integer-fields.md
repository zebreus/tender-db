# 433 — `unrepresentable-value` holds 326 whole notices, and only ~44 are the "astronomical" class the terminal ledger names: the rest are junk in minor integer fields

Status: **DONE 2026-09-28** — Verify reads 3 (282 → 3; 4088754/4311463 BT-720-Tender astronomical and 3355452 zoneless BT-145-Contract, strict by design); reprocess jobs 1618/1620 reclaimed 323 and fold 1622 folded them; ledger rows and the terminal-policy comment match the measured composition. Was: ready-for-agent — Verify read done 2026-09-28 (282 → 3, the three held on purpose).
Was status (before 2026-09-29): ready-for-agent — **Verify read done 2026-09-28 (282 → 3, the three held on purpose; see the Verify block)**; the reclaimed notices fold in job 1622. Was: **DECIDED (option c) and BUILT 2026-09-27** (see the foot): the eForms walk keeps a failed integer/indicator/number value, and the eSender transmission stamp, as raw text under its own field id ("raw kept, typed absent", the r209/text-era rule); every amount and every other date stays strict. The deploy waits for the 393/397 re-parse fold (with 432), then the per-pattern reprocess reclaims the held rows. Was: ready-for-agent — filed 2026-09-27 from the hourly audit (step 3).
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
- **read 2026-09-28 ~17:58 UTC**: **3**, after the three reprocess jobs ran on rev `da917b4` with `reclaim_only`.
  Job 1618 (`%not an integer%`) reclaimed 310 across 61 packages (0 still held), job 1619 (`%not a number%`) found no
  held packages left, and job 1620 (`BT-803(%`) reclaimed 13 across 6. The 3 left are shapes the rule holds on
  purpose, because the fold reads those fields: 4088754 and 4311463 carry `BT-720-Tender` 74654684654465480000
  (astronomical), and 3355452 carries `BT-145-Contract` "2024-04-12" with no zone offset. **done** for the count; the
  reclaimed notices are folded by the `project` queued behind 394's refold (job 1622).

## Decided and BUILT 2026-09-27 — option (c), sharpened by a read-only research fan-out

**The research that settled it** (5 lenses, each adversarially verified; kept in the session scratchpad):

- The fold reads NONE of BT-44, BT-113, BT-171, BT-661, BT-58, BT-686, BT-33 or BT-803(d). The main fold loop has no
  Integer or Number arm, and the only eForms integer/number it reads is BT-759 (received submissions,
  `project.rs` `read_results`). Of the "other" five, BT-720-Tender (bid value, summed into contract and awarded
  values) and BT-145-Contract (conclusion date) ARE read.
- eForms was the outlier: the r209 and text-era parsers already turn a bad integer, amount, number or date into a
  text row under the same field id ("raw kept, typed absent"), citing ted-legacy-mapping.md §8.2 — "Quarantine is
  for unconsumed structure, not low-quality values" — which ADR-0004's amendment points to. No ADR forbade it; the
  earlier "hold" rulings were taken at n=1 (144 H) or on the unmeasured "10^50 class" premise (268, ADR-0010's note).
- BT-803(d) fails in the DATE path (`split_offset`), not the integer arm. It is the eSender TransmissionDate, not the
  dispatch date (BT-05(a)); issue 418's resolver never reads it. Because the walk reaches it first, each such hold
  may hide a later failure: the 13 are an upper bound on what the stamp alone reclaims.

**The rule** (`crates/ingest/src/eforms/parse.rs` `soft()` / `SOFT_DATE_FIELDS` / `keep_raw`):

- **SOFT**: every `Integers` (integers and indicators) and `Numbers` field, plus the exact ids `BT-803(d)-notice`,
  `BT-803(t)-notice`, `DE1-TransmissionDate` and `DE1-TransmissionTime` (the DE 1.x spelling of the same stamp,
  added at review). A value that does not convert keeps its trimmed raw text as a text row under the same field
  id and ordinal sequence, and the notice imports. No zone is guessed for the stamp. The time half of a soft pair
  keeps its own raw text.
- **One fold-read soft field, by decision**: BT-759-LotResult. Junk there drops that result block's statistics row
  (the fold emits a statistic only as a kind+count pair) instead of holding the notice. Statistics are served per
  block and never summed, so absent reads as "not published".
- **STRICT**, by type, whether or not the fold reads the field: every `Amounts` field (a silently absent BT-720 would
  under-report a summed contract value) and every other `Dates` field.
- `value.rs`: exact widenings only — `.00` → 0, case-insensitive `True`/`False` → 1/0.

**Tests** (real published fixtures, mutated in the test; mutation-checked): `junk_in_an_integer_field_keeps_its_raw_text`
(BT-171 `_DEFAULT_VALUE_CHANGE_ME_`), `an_astronomical_count_keeps_its_raw_text` (BT-113 10^40),
`lossless_integer_shapes_convert`, `junk_in_an_amount_still_quarantines` (BT-720), `zoneless_fold_read_date_still_quarantines`
(BT-145), `zoneless_esender_stamp_keeps_its_raw_text`, `zoneless_de1_esender_stamp_keeps_its_raw_text`, the converting-pair
negative in `ted_transmission_stamp_is_claimed_on_older_minors`, and the unit tests
`soft_is_every_count_and_only_the_listed_dates` / `integers_are_exact_or_fail`.

**Ledger and docs**: three `issue 433` rows (`%not an integer%`, `BT-803(%`, `%not a number%`, `resolved: null`); the
terminal-policy comment (`dashboard.rs`) and ADR-0010's note now state the measured composition.

**After deploy (with 432, after the 393/397 fold): the reclaim, one reprocess per pattern — never reason-wide** (a
reason-wide job re-stamps `first_reason` on the strict rows that stay held and drops them from the weekly arrival
rate):

    {"kind":"reprocess","reason":"unrepresentable-value","detail_like":"%not an integer%"}
    {"kind":"reprocess","reason":"unrepresentable-value","detail_like":"%not a number%"}
    {"kind":"reprocess","reason":"unrepresentable-value","detail_like":"BT-803(%"}

Then read the Verify (282 → near 0) and the held residue's details (a reclaimed row that fails a strict field later in
the walk rewrites its detail and stays held — expected, and it names a new cause).

