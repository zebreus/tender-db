# 489 — an F14 value correction never supersedes the figure it corrects

Status: ready-for-agent — UNITS 2+3 DEPLOYED AND DRAINED 2026-10-07 (see "Drain — result"): 12,738 corrections admitted corpus-wide; 6891632 €25.28 bn → €85.5 m, 7257797 → €12.42 bn, 4871119 unchanged (its flip-flop), ≥ €1 bn 3,822 → 3,817. NEXT: measure the unread (17,108) and ambiguous (5,191) classes on a window — the currency-less NEW_VALUE share decides whether a unit 4 (take the currency of the matched old figure) is worth it. Was: DRAIN RUNNING: sizing job 2937 counted 191,493 carrier notices (aborted, nothing written), wet refold-fields enqueued as jobs 2030/2031 (`expect` 191,493) with its paired project. Before: 3,822 Tenders ≥ €1 bn; 6891632 €25,280,256,000.00, 7257797 €12,545,764,416.17, 4871119 £25 bn. NEXT: read the paired project's `issue-489` clause, re-read those three and the ≥ €1 bn count. Was: UNIT 2 LANDED 2026-10-06 (see "Unit 2 — landed"; gate, deploy, then the drain). Was: filed 2026-10-06, split out of issue 471 unit 4(b) (the issue's own
"whether (b) stays here or becomes its own issue" decision: it is a different mechanism — free-text
amount parsing plus same-field supersession in the version state — and inside 471's band it moves
one row). UNIT 1 MEASURED on a window (see "Unit 1 — window measurement (2026-10-06)"); scope decided: II.1.5 / II.1.7 (tender scope) only. NEXT: unit 2 — read how a corrigendum version inherits amounts, then the strict reader + supersession with the pin.
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

## Unit 1 — window measurement (2026-10-06)

385's window (notice ids 21,000,000–21,020,000, r2.0.9 era), one bounded `/v1/sql` read each
(`489-values/f14v.sql`, `f14t.sql`; the 144 texts in `window-21000000-new-value-texts.json`):

| coordinate | CHG blocks with `NEW_VALUE.TEXT` | notices |
|---|---|---|
| II.1.5 (total estimated value) | 32 | 32 |
| II.1.7 (total value of the procurement) | 16 | 16 |
| II.2.6 (a lot's estimated value) | 66 | 35 |
| V.2.4 (a contract's value) | 31 | 17 |

About 0.4 % of the window's notices correct a value section; extrapolated over the r2.0.9 era that
is on the order of 10⁴ notices — a real population, mostly NOT scale errors (ordinary restated
estimates), so 4(b) is a currency-of-the-record fix more than a band fix.

**What the texts look like.** Mostly one figure, in every EU locale's formatting:
`379 502,40 EUR`, `Wartość bez VAT: 1 703 480,12 PLN`, `Valore, IVA esclusa: 224,425,00 EUR`
(a malformed grouping), `58 903,50EUR.`, `5000000`, `Hodnota bez DPH: 170 317 000,00`. Three
complications:

- **No currency** on roughly a fifth of them (`4 500 000,00`, `21 766 171,00`); some put it in a
  SECOND block of the same section (`Munt: EUR`, `Měna: CZK`) or a second line (`Valeur totale
  estimée:` / `Valeur hors TVA: 9 386 307,00 EUR.` as two CHG blocks).
- **II.2.6 names no lot key** in the block; several lots' corrections arrive as consecutive blocks,
  some as prose (`Per il lotto n. 17, provincia di Napoli: valore … 68 847 509,46 EUR, inclusivi di
  254 362,50 EUR …`, `Pour le lot 1 …, la valeur estimée est de 430 000,00 EUR. Pour le lot 2 …`).
- **V.2.4 carries two figures** in the Polish form (`Początkowa szacunkowa…` the initial estimate
  AND `Całkowita końcowa…` the final value) as two blocks of the same coordinate.

**Decision (owner, 2026-10-06): unit 2 maps II.1.5 → tender `estimated_value` and II.1.7 → tender
`result_value` only.** Both are tender scope, one figure per notice in the sample (32/32, 16/16),
so there is no lot or award to resolve. II.2.6 and V.2.4 stay unmapped (lot keying and two-figure
blocks would need guessing — 385's rule: a missing correction is visible as a stale figure, an
invented one is not), each recorded as a refused class in the project counters so their volume stays
visible. The reader is STRICT: after stripping a label ending in `:` and a trailing `.`, the text must
be exactly one amount (space / NBSP / `.` / `,` grouping, `,` or `.` decimal with exactly two
digits, or none) optionally followed by an ISO currency; a currency-less figure takes the currency
of the figure it supersedes only when the Tender's head carries that field in exactly ONE currency;
anything else (prose, two figures, malformed grouping like `224,425,00`) is refused and counted.

## Unit 2 — landed (2026-10-06)

- `project.rs`: `F14_TARGET_AMOUNTS` (II.1.5 → `estimated_value`, II.1.7 → `result_value`),
  `f14_new_value_amount` (the strict reader over `r209::value::display_cents`; currency REQUIRED),
  `f14_value_corrections` (one figure per field per notice; two different figures → ambiguous,
  none). `NoticeState::read` inserts the admitted ones as tender-scope `Fact::Amount`s, and the
  fold's per-field `supersede` replaces the carried figure — no new fold rule.
- The tally rides `F14TargetGate` (`value_estimated`, `value_result`, `value_unread`,
  `value_ambiguous`, `value_lot_or_award`), counted by the same function, and the project job's
  line gains `; issue-489 F14 value corrections: N mapped (estimated …, result …), … unread, …
  ambiguous field(s), … lot/contract refused`.
- Pins: `an_f14_value_correction_supersedes_the_figure_it_corrects` (tests/project.rs: 6891632's
  shape, the II.2.6 / II.2.7 blocks stay out, 4871119's flip-flop resolves by publication) and
  `an_f14_new_value_text_is_read_only_when_it_is_one_figure_and_one_currency` (project.rs unit).

**Known limit, recorded (lifted by unit 3 below):** 6891632 itself will NOT leave the band through this unit — its F14 also
corrects the lot's II.2.6 (€25.2 bn → €5.28 m), which stays unmapped, and the head is the MAX over
tender and lot amounts. A single-lot II.2.6 mapping is the natural unit 3.

**Drain (after deploy, queue idle):** size, then refold, as 385 did:
`{"kind":"refold-fields","profiles":["TED-NEW_VALUE.TEXT"],"tables":["notice_texts"],"expect":1}`
(aborts and prints the carrier count), then the same with `expect` set to that count. The
refold-fields handler queues its OWN paired `project` (rebuild false) — do not queue another; the
`issue-489` clause of THAT project's line is the corpus-wide measurement.

## Unit 2 — review fixes (2026-10-06)

Adversarial review (workflow wf_09336786-528: 3 lenses, each finding refuted-or-confirmed by a
skeptic) confirmed seven minor findings; fixed:

- **A late F14 correcting an OLDER notice overwrote the newer figure** (PIN → CN → F14 of the PIN;
  CAN1 → CAN2 → F14 of CAN1). Corrections now travel beside the facts as `ValueCorrection`
  {field, new, OLD} (the block's `TED-OLD_VALUE.TEXT`, currency optional) and the fold applies one
  only when the chain carries the old figure in that field. Pinned in the integration test (a stale
  correction is not applied).
- **An unreadable sibling block did not poison its field** (a currency-less ex-VAT block beside a
  readable VAT-inclusive one mapped the latter). Any unreadable block now refuses the field.
- **"N mapped" overcounted** (per block, before agreement): now per notice and field, labelled
  "admitted"; the fold's old-value check can still decline one, which a plan-time tally cannot see.
- **The drain doc queued a second, empty project** — corrected above.
- **A doc comment was split** by the new unit test — restored.

Recorded, not fixed: **an upward ×10ᵏ correction to ≥ €1 bn is refused by issue 471's
`ScalePartners`**, because the superseded figure stays a partner in the chain (any-version reach
is deliberate for 6721266). It needs the publisher to RAISE a figure exactly 1000× to ≥ €1 bn —
the opposite of every band case read so far; the drain's before/after will show whether it occurs.


## Unit 3 — II.2.6 by old-figure match (2026-10-06, landed)

The review fix made every correction carry the figure it replaces, which removes the reason II.2.6
was refused: the block names no lot KEY, but it names the lot's old VALUE. So II.2.6 →
`estimated_value` of the ONE lot of the chain whose carried `estimated_value` is the block's OLD
figure (currency compared when stated); no lot, or two lots carrying that same figure, and nothing
moves. Blocks need not agree with each other (one block per lot), only two blocks naming the same old
figure must; an unreadable block costs only its own lot. V.2.4 stays refused (two figures per
coordinate in the Polish form, and its target is an award, not a lot estimate).

Tally: `value_lot` (admitted lot corrections), `value_contract` (V.2.4 refused; was
`value_lot_or_award`); the project line reads `… N admitted (estimated …, result …, lot …), … unread,
… ambiguous, … contract refused`. The integration pin now carries 6891632's lot: after the F14 the
lot reads €5,280,000 and the head is the corrected €85,536,000.

## Drain — result (2026-10-07)

Job 2939 (`refold-fields` over `TED-NEW_VALUE.TEXT`, `expect` 191,493): re-queued 191,493 notices,
stamped 156,263 Tenders. Its paired project 2940 routed to the whole-corpus bucketed path (above the
100k threshold): 14,891,709 notices → 8,778,432 Tenders, 1,622,700 written, 7,155,732 verified
unchanged, 23:48 → 04:13 UTC (4 h 25 m). Its line:

`issue-489 F14 value corrections: 12738 admitted (estimated 3826, result 1076, lot 7836), 17108 unread,
5191 ambiguous, 5474 contract refused`

Read after:

| Tender | before | after | |
|---|---|---|---|
| 6891632 | €25,280,256,000.00 | €85,536,000.00 | fixed (II.1.5 + its lot's II.2.6) |
| 7257797 | €12,545,764,416.17 | €12,423,762,042.36 | the latest of three restatements |
| 4871119 | £25 bn | £25 bn | the second F14 corrected it back |

Tenders ≥ €1 bn: 3,822 → 3,817; ≥ €10 bn (section 16's band): 305 → 304.

**Open:** unread (17,108 blocks) is larger than admitted. From unit 1's sample, most are the
currency-less NEW_VALUE (`4 500 000,00`); since the fold now matches the OLD figure, a currency-less
new value could take the matched figure's currency safely. That is unit 4, after a window measurement of
the unread shapes.
