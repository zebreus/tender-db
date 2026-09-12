# 383 — `TED-DATE_OF_CONTRACT_AWARD`: a second legacy spelling of the award date that the results reader does not match

Status: ready-for-agent — units 1–2 DONE 2026-09-12 (read, matched, fixture, tests; see Answer); unit 3 (size, refold its carriers, re-probe) waits for the box to go idle after issue 368's fold. Was: ready-for-agent (filed 2026-09-12 by the owner, from issue 368's per-profile probe)
Kind: defect (data) — a MODELLED concept (the award decision date, issue 255) missing for one
spelling, which is exactly issue 368's shape
Blocked by: nothing

## What the probe showed

`GET /admin/unmapped-fields?profile=ted-export-r208` over the era's own head window (ids
27,061,440–27,161,439), with the per-channel sieve (`aa9c080`):

| field id | table | rows in window |
| --- | --- | --- |
| `TED-CONTRACT_AWARD_DATE` | notice_dates | 1,688 — read by `read_legacy_results`; was a predicate gap, fixed in 368 unit 2 |
| `TED-DATE_OF_CONTRACT_AWARD` | notice_dates | **79 — matched by nothing** |

`read_legacy_results` (`crates/ingest/src/project.rs`) matches `LEGACY_AWARD_DATE_FIELD` =
`TED-CONTRACT_AWARD_DATE` only. `grep -c DATE_OF_CONTRACT_AWARD crates/ingest/src/project.rs` → 0.
The r209 parser's element list carries the spelling (it is a real published element), so the parse
layer stores it and the projection drops it, and those awards carry no `decided` instant.

## What is not known yet

- Which form/era publishes this spelling (79 rows in a 100k-id window of r208's head; corpus size
  unknown — `refold-fields` with `expect: 1` sizes it in one sweep, ~46 min).
- Whether it sits on the award block the way `CONTRACT_AWARD_DATE` does (the reader keys on the
  section), or somewhere the LotResult synthesis does not look.

## Units

1. Read one carrier (bounded band + `notice_dates`, then the archive member) — which form, which
   section, same instant semantics as `CONTRACT_AWARD_DATE`?
2. If it is the same fact in another spelling: match it beside `LEGACY_AWARD_DATE_FIELD` in the
   reader AND the date-channel predicate (the same constant, so they cannot drift — issue 384 is
   the guard), fixture, `refold-fields` over it.
3. Re-run the probe: the row disappears from the r208 listing.

## Answer, units 1–2 (2026-09-12)

**Unit 1, read.** One carrier from the r208 head band: notice 27,159,613 = publication
`070248-2010`, `FORM="6"` (contract award, utilities), `R2.0.7.S02.E01`, archived at
`2010-03.tar › 03/20100310_2010048.tar.gz › 20100310_48/00070248_2010.xml`. It carries **seventeen**
`<DATE_OF_CONTRACT_AWARD><DAY>01</DAY><MONTH>06</MONTH><YEAR>2009</YEAR>` blocks, one per award
block (`RES-1` … `RES-17` in the notice layer), stored as one instant each (1243814400 =
2009-06-01). Same section, same fact as `CONTRACT_AWARD_DATE`; R2.0.7 spelled it differently, and
R2.0.8 kept the DAY/MONTH/YEAR shape under the new name. So this is the era-spelling case, not a
different fact.

**Unit 2, matched.** `LEGACY_AWARD_DATE_FIELD_R207` beside `LEGACY_AWARD_DATE_FIELD` in
`read_legacy_results` (one alternation arm), in the date-channel arm of `has_destination`, and in
issue 384's guard list — the guard is what makes the two places unable to drift. The notice is
committed verbatim as `r208/f06-r207-070248-2010.xml`; the r208 exhaustive-consumption guard
consumes it (ten fixtures now), and `the_r207_award_date_spelling_dates_the_award` folds it and
asserts every lot result carries 2009-06-01 as its decision instant.

**Unit 3, pending the box.** Size the carriers with `refold-fields` + `expect: 1` (one full sweep,
~46 min), then the real run, then the probe should no longer list the id. Not started: issue 368's
345k-carrier refold and fold hold the box.

## Unit 3, first attempt: the sizer answered 0, and the sizer was wrong (2026-09-12)

`refold-fields` over `TED-DATE_OF_CONTRACT_AWARD` with `expect: 1` (job 1326, 2,318 s): **"0 notices
carry [...]"**. For a field the probe lists at 79 rows in one 100k-id window and a direct read had
just shown on notice 27,159,613. The sweep (`notice_ids_carrying_fields`) opened `notice_amounts`
and `notice_texts` only — its doc comment said to extend the list when a mapped id needed another
channel, and nobody had — so a date field could never be found, and the answer was indistinguishable
from a mistyped id. That is the worst shape for a cohort finder: a silent, confident zero.

Fixed: the sweep walks every value table by default (`NOTICE_VALUE_TABLES`, the store's constant),
the request may narrow it with `"tables": [...]` when the channel is known, and a name outside the
eight is refused at enqueue rather than after an hour. Store test seeds a date-channel carrier and
finds it; a narrowed sweep pointed at the wrong table honestly misses it; the supervisor refuses
`notice_typo` naming the list. Deploying, then the sizer again with `"tables": ["notice_dates"]`.

### Sized with the fixed sweep (2026-09-12, `79534cf` deployed)

`refold-fields` over `TED-DATE_OF_CONTRACT_AWARD`, `tables: ["notice_dates"]`, `expect: 1` (job
1328): **88,251 notices carry it**, in **116 s** — against the 2,318 s the blind two-table sweep spent
answering zero. The narrowing is what makes a single-channel sizing cheap: the dates table is a
fraction of texts.

88,251 carriers is the R2.0.7 era's award notices (2010–2011 F06 and its siblings), every one of
which folded with no decision date until now. The real run is submitted (job 1330, `expect: 88251`,
same narrowing) with the incremental `project` queued behind it (1331); the fold plans the whole
corpus regardless of cohort size (issue 368's 319k-tender cohort took 4.04 h), so the re-count and
the probe re-run follow that.
