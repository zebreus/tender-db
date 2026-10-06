# 489 — an F14 value correction never supersedes the figure it corrects

Status: ready-for-agent — filed 2026-10-06, split out of issue 471 unit 4(b) (the issue's own
"whether (b) stays here or becomes its own issue" decision: it is a different mechanism — free-text
amount parsing plus same-field supersession in the version state — and inside 471's band it moves
one row). NEXT: unit 1, the measurement below.
Kind: data quality / canonical values
Relates to: 471 (the band; 4(b) moved here), 385 (F14 corrigendum DATES — the pattern this follows)

## What is wrong

A TED F14 corrigendum that corrects a value section publishes `TED-SECTION` (the form coordinate),
`TED-OLD_VALUE.TEXT` and `TED-NEW_VALUE.TEXT` per `CHG-n` block. Issue 385 maps
`TED-NEW_VALUE.DATE` to canonical dates by coordinate (`F14_TARGET_DATES`, `project.rs`); nothing
maps a value correction, so the corrected figure stays elected and the publisher's own fix is
visible only in `/v1/notices/{id}/content`.

Read on prod 2026-10-06 (the three band rows section 16 flags with a value corrigendum that 471's
exact-10ᵏ rule does not already handle):

| Tender | Corrigendum | What it says | What 4(b) would do |
|---|---|---|---|
| 6891632 | notice 21260539 | II.1.5 `25 280 256 000,00 EUR` → `85 536 000,00 EUR`; II.2.6 `25 200 000 000,00` → `5 280 000,00` | **fix**: €25.3 bn → €85.5 m (the head today) |
| 7257797 | 22142289, 22170703, 22216169 | II.1.5 €12,545,764,416.17 → 12,409,218,269.80 → 12,437,796,606.65 → 12,423,762,042.36 | follow the latest restatement (small move; the big figure is genuine) |
| 4871119 | 19280609, then 19334545 | II.1.5 £25 bn → £250 m, then a SECOND F14 corrects it BACK: £250 m → £25 bn | latest wins: £25 bn, unchanged — the flip-flop case a rule must order by publication |

Shapes the parser must read: `Value excluding VAT: 250 000 000.00 GBP` (English label, space
thousands, `.` decimal), `Valore, IVA esclusa: 85 536 000,00 EUR` (Italian label, `,` decimal),
`12.409.218.269,80 EUR` (bare, `.` thousands). Free-text sections (II.2.7, II.2.11, VI.3) also
carry amounts inside prose; those are NOT value sections and must not map (385's lesson).

## Proposed fix

1. **Measure first.** Count F14 `CHG-n` blocks whose normalised `TED-SECTION` is one of
   `BAND_VALUE_CORRIGENDUM_SECTIONS` (II.1.5, II.1.7, II.2.6, V.2.4; 2014-directive numbering
   only) and that carry `TED-NEW_VALUE.TEXT`, corpus-wide, windowed by `notice_id` (no
   `field_id` index on `notice_texts`: the bounded route is a data-quality-style windowed sweep or
   the project job's own counters, like 385's "issue-385 F14 corrigendum dates" line). Classify the
   NEW_VALUE.TEXT shapes: how many parse to exactly one (amount, currency) with the strict reader;
   how many are prose. Record how many Tenders' elected head would move.
2. **Decide the supersession semantics** against how a version's fact state is built across the
   chain (a corrigendum version inherits the earlier facts; an added `estimated_value` beside the
   old one would lose the MAX election). Keyed by (scope, field) from the coordinate: II.1.5 →
   tender `estimated_value`, II.2.6 → the lot's `estimated_value` (lot key from the block, if
   published), II.1.7 → tender `result_value`, V.2.4 → the award's value. Ordered by publication,
   so 4871119's second F14 wins.
3. **Pin** `an_f14_value_correction_supersedes_the_figure_it_corrects` (`crates/ingest/tests/project.rs`)
   with 6891632's shape, plus the 4871119 flip-flop and a prose II.2.7 block that must NOT map.
4. **Drain** with `refold-notices` over the affected Tenders' F14 notices and re-read section 16.
