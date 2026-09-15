# 385 — every F14 corrigendum date is folded into `submission_deadline` regardless of the section it changes, and the MAX election then serves the IV.2.7 opening time (or an IV.2.6 validity date months later) as the tender's deadline

Status: needs-triage — filed 2026-09-15 by the API/data-quality review fan-out (32 lenses, every finding independently reproduced and adversarially judged)
Kind: defect (ingest → canonical projection, r208/r209 F14 corrigenda)
Relates to: 366 (the MAX election this rides on; its line 56 lists `TED-NEW_VALUE.DATE` among the
co-published deadline sources and calls two deadlines "structurally normal" — its fix was a
plausibility horizon, not a section-aware mapping, so it does not cover this), 09 (where the design
"typed NEW_VALUE date → submission-deadline delta" was written, line 54, without section
discrimination), 174 (the r208 deadline-id gap, the same field one era over), 201 (CHG-N sections as
a quarantine bucket), 216 (`sort=deadline` and the head deadline column this feeds)

## Observed (verified 2026-09-14 on prod)

An F14 corrigendum publishes one `CHANGE`/`ADD`/`REPLACE` block per changed field, each naming its
target in `TED-SECTION`. The projection ignores the target.

    ssh root@zebreus.click 'echo "SELECT notice_id, section_id, field_id, substr(value,1,90) AS v FROM notice_texts WHERE notice_id IN (21000077) AND section_id LIKE \"CHG-%\" ORDER BY section_id, field_id" | /root/sq.sh'

| section | `TED-SECTION` | `TED-LABEL` |
| --- | --- | --- |
| CHG-1 | IV.2.2 | Termin składania ofert… (submission deadline) |
| CHG-2 | IV.2.7 | Warunki otwarcia ofert (opening conditions) |

    ssh root@zebreus.click 'echo "SELECT section_id, ordinal, strftime(\"%Y-%m-%d %H:%M\", utc_seconds, \"unixepoch\") FROM notice_dates WHERE notice_id = 21000077 AND field_id = \"TED-NEW_VALUE.DATE\"" | /root/sq.sh'

| section | new value |
| --- | --- |
| CHG-1 (IV.2.2, the deadline) | 2020-04-24 09:00 |
| CHG-2 (IV.2.7, the opening) | 2020-04-24 09:30 |

The F14's own `PROCEDURE/TED-DT_DATE_FOR_SUBMISSION` is 09:00 as well. What is served:

    curl -s https://tenders.zebreus.click/v1/tenders/6762566

`"submission_deadline":"2020-04-24T09:30:00+00:00"` — the tender-**opening** time. `dates` carries
`opening_date` 2020-04-17T09:30 (the stale value from the original CN, never corrected),
`submission_deadline` 09:00 and `submission_deadline` 09:30; `versions` seq 2 `caused_by_notice_id`
21000077.

**Which sections the F14 date changes actually target**, r209 window 21,000,000–21,020,000 (the
group-by returns both the parenthesised and bare spellings of a section id; rows are notices):

    ssh root@zebreus.click 'echo "SELECT s.value AS section, COUNT(DISTINCT d.notice_id) AS notices FROM notice_dates d JOIN notice_texts s ON s.notice_id = d.notice_id AND s.section_id = d.section_id AND s.field_id = \"TED-SECTION\" WHERE d.notice_id BETWEEN 21000000 AND 21020000 AND d.field_id = \"TED-NEW_VALUE.DATE\" GROUP BY s.value ORDER BY notices DESC LIMIT 15" | /root/sq.sh'

| target section | notices | what it is | correct destination |
| --- | --- | --- | --- |
| IV.2.2 | 1,696 + 961 | submission deadline | `submission_deadline` |
| IV.2.7 | 1,237 + 893 | tender opening | `opening_date` |
| IV.2.6 | 175 + 145 | tender validity "until" | not a deadline at all |
| II.2.7 | 26 | contract duration | duration |
| VI.3 | 18 | additional information | parse-layer only |
| IV.2.3 | 17 | date of invitations | not the deadline |

**What is served for the IV.2.7 class**, same window:

    ssh root@zebreus.click 'echo "SELECT COUNT(DISTINCT d.notice_id) AS corrigenda_changing_IV27, COUNT(DISTINCT t.id) AS tenders, COUNT(DISTINCT CASE WHEN t.current_deadline = d.utc_seconds THEN t.id END) AS served_deadline_equals_IV27_value FROM notice_dates d JOIN notice_texts s ON s.notice_id = d.notice_id AND s.section_id = d.section_id AND s.field_id = \"TED-SECTION\" JOIN tender_versions v ON v.caused_by_notice_id = d.notice_id JOIN tenders t ON t.id = v.tender_id WHERE d.notice_id BETWEEN 21000000 AND 21020000 AND d.field_id = \"TED-NEW_VALUE.DATE\" AND s.value LIKE \"IV.2.7%\"" | /root/sq.sh'

| window | corrigenda changing IV.2.7 | tenders touched | `current_deadline` = the IV.2.7 opening value |
| --- | --- | --- | --- |
| 21,000,000–21,020,000 (2020-04) | 2,133 | 1,991 | **1,229 = 62%** |
| 21,000,000–21,006,000 (judge's narrowed re-run) | 705 | 695 | **414 = 60%** |
| 19,500,000–19,510,000 (2019) | 473 | 462 | **288 = 62%** |

The 2019 window's targets are the same shape: IV.2.2 577, IV.2.7 471, IV.2.6 76, IV 41, II.2.7 17,
II.3 8, IV.2.5 7. So this is not a 2020 artefact — it is the whole F14 era.

**The error size depends on which section was changed.**

| class | share of F14 date changes | how far off | example |
| --- | --- | --- | --- |
| IV.2.7 (opening) | ~40% | 30 minutes to a day | 6762566 serves 09:30, deadline is 09:00 |
| IV.2.6 (validity) | ~6% | weeks to months | 6737588 serves `2020-07-09` (a date-only validity "until" value) against a IV.2.2 of 2020-05-11 10:00 |
| II.2.7 / VI.3 / IV.2.3 | ~1% | arbitrary | — |

Third example, same shape: tender 6752749 (notice 21000075) serves 2020-05-20T09:00 beside a
2020-05-19T17:00 `submission_deadline` fact.

The list surface uses the same wrong instant, so it is not a detail-payload cosmetic:

    curl -s 'https://tenders.zebreus.click/v1/tenders?deadline_after=2020-04-24T09:15:00Z&deadline_before=2020-04-24T09:45:00Z&country=PL'

returns 6762566; the 08:45–09:15 window — the one containing its real deadline — does not.
`sort=deadline` places it by the opening instant too.

**What is NOT true**, and the fan-out's own title overstated it: no live surface is affected. The
closes-soon head (`deadline_after=<unix now>&sort=deadline&order=asc`) is entirely 2026 eForms
tenders, and 0 of the 5,234 `NEW_VALUE.DATE` values in the 21M window are still in the future (max
2025-05-31). Every F14-era tender is closed by now regardless of which instant we serve. The harm is
to historical deadline values, their sort position, `deadline_after`/`deadline_before` placement —
and to `opening_date`, which is silently stale for exactly the tenders whose opening time was
corrected.

Attribution correction carried over from the repro: 6737588's served 2020-07-09 comes from notice
**21014489** (seq 5, CHG-5, IV.2.6), not 21000079 as first written. 21000079's own IV.2.6 is
2020-06-25. The defect is unchanged; only the carrier id was wrong.

## Why it matters

A consumer asking "when could I still have bid on this?" gets the wrong answer for roughly 60% of
tenders that ever received an IV.2.7 corrigendum — about 2 in 5 of all F14 date corrigenda — and the
API documents the field as exactly that question: OpenAPI describes `status` as "By submission
deadline" and `deadline_after` as "the current version's submission deadline". For the IV.2.6 class
the served deadline is a tender-validity expiry, months after bidding closed, and nothing in the
payload marks it as a different kind of date. Anyone reconstructing a procurement timeline, measuring
time-to-close, or filtering a historical window by deadline gets a value that is wrong in a
direction they cannot detect: all three instants sit in the `dates` array as `submission_deadline`
facts, indistinguishable from each other.

The second loss is quieter. A corrigendum that moves the opening is the only event that could correct
`opening_date`, and it is consumed as a deadline instead — so 6762566 still advertises an opening of
2020-04-17T09:30, a week before the opening actually happened, from the superseded CN.

## Why this is ours, not the publisher's

The publisher states the target correctly and we store it: `Rule::Section(Kind::Change)`
(`crates/ingest/src/r209/rules.rs:339` for the r209 `CHANGE` block, `:683` for the r208
`ADD`/`DELETE`/`REPLACE` shape) writes `TED-SECTION` and `TED-LABEL` per `CHG-n`, and the r209 test
suite already asserts it (`crates/ingest/tests/r209.rs:331`). The projection then discards it:
`crates/ingest/src/project.rs:276` is `("TED-NEW_VALUE.DATE", "submission_deadline")` with no
section predicate, and the comment three lines above says so in as many words — "Section-aware
mapping of every F14 WHERE target is deferred". `grep -rn TED-SECTION crates/ --include=*.rs` finds
no consumer outside that one test. `head_deadline` (`crates/store/src/canonical.rs:1286`) is MAX over
the head's `submission_deadline` facts, carrying only issue 366's plausibility horizon, so an opening
time 30 minutes later or a validity date months later always wins the election. Every input needed to
map correctly is already in the database; the defect is entirely in what we do with it.

## Repro

Under two minutes, no snapshot needed (metadata and bounded reads only for steps 1–2; step 3 is the
public API).

1. `ssh root@zebreus.click 'echo "SELECT notice_id, section_id, field_id, substr(value,1,90) AS v FROM notice_texts WHERE notice_id IN (21000077) AND section_id LIKE \"CHG-%\" ORDER BY section_id, field_id" | /root/sq.sh'` → CHG-1 = IV.2.2, CHG-2 = IV.2.7.
2. `ssh root@zebreus.click 'echo "SELECT section_id, ordinal, strftime(\"%Y-%m-%d %H:%M\", utc_seconds, \"unixepoch\") FROM notice_dates WHERE notice_id = 21000077 AND field_id = \"TED-NEW_VALUE.DATE\"" | /root/sq.sh'` → 09:00 on CHG-1, 09:30 on CHG-2.
3. `curl -s https://tenders.zebreus.click/v1/tenders/6762566` → `submission_deadline` is 09:30, the opening.
4. `curl -s 'https://tenders.zebreus.click/v1/tenders?deadline_after=2020-04-24T09:15:00Z&deadline_before=2020-04-24T09:45:00Z&country=PL'` → 6762566 present; swap to `08:45`/`09:15` → absent.
5. For the months-off class: `curl -s https://tenders.zebreus.click/v1/tenders/6737588` → `submission_deadline` `2020-07-09`.

Side observation found on the way, not part of this defect: the documented literal
`deadline_after=now` is rejected with 400 `must be unix seconds or RFC 3339`. Use a unix timestamp.
File separately if it is not already tracked.

## Done when

- `crates/ingest/src/project.rs` maps `TED-NEW_VALUE.DATE` (and the r208 `ADD`/`REPLACE` equivalents)
  by the `TED-SECTION` of its own `CHG-n` section, not unconditionally: IV.2.2 and IV.3.4 →
  `submission_deadline`, IV.2.7 → `opening_date`, IV.2.6 → not a date fact (or its own canonical
  concept), II.2.7 → duration, anything else → parse-layer-only with a recorded reason, so a new
  target is refused rather than silently folded as a deadline. The deferral comment at lines 273–275
  is gone or narrowed to what is still deferred.
- A section id we have never seen cannot become a `submission_deadline` by default; the fallback is
  "not mapped", and it shows up in the unmapped-field probe (issue 368's sieve) rather than in the
  headline field.
- Fixtures committed for both eras: an r209 F14 whose CHG blocks target IV.2.2 and IV.2.7 (21000077
  is the natural candidate) and an r208 `REPLACE` carrier, with tests asserting the IV.2.2 value
  lands on `submission_deadline`, the IV.2.7 value on `opening_date`, and the IV.2.6 value on
  neither.
- A test pins the election end-to-end: fold the two-change corrigendum and assert `head_deadline` is
  09:00, not 09:30.
- The F14 carriers are refolded (`refold-fields` over `TED-NEW_VALUE.DATE` with
  `tables: ["notice_dates"]` to size first, then `refold-notices`), and the served values move:
  6762566 → `submission_deadline` 2020-04-24T09:00 and `opening_date` 2020-04-24T09:30 (the stale
  2020-04-17 value corrected by the same pass); 6737588 → 2020-05-11T10:00, not 2020-07-09; 6752749 →
  2020-05-19T17:00.
- The census re-runs to zero on both windows: in 21,000,000–21,020,000 and 19,500,000–19,510,000, the
  count of tenders whose `current_deadline` equals an IV.2.7 `NEW_VALUE.DATE` value drops from
  1,229/1,991 and 288/462 to 0 (barring the rare tenders where the two genuinely coincide, which the
  re-run should name rather than tolerate silently).
- `deadline_after=2020-04-24T08:45:00Z&deadline_before=2020-04-24T09:15:00Z&country=PL` returns
  6762566 and the 09:15–09:45 window does not.

## Unit 1 BUILT 2026-09-15 (owner) — the mapping is section-aware, and two premises were measured first

`crates/ingest/src/project.rs`: `TED-NEW_VALUE.DATE` is gone from the flat `DATES` table and now
resolves per `CHG-n` through a new `F14_TARGET_DATES` table, paired with the block's own
`TED-SECTION` via a `change_targets` map built before the value loop — the `tax_bases` shape this
file already uses for the other sibling-qualified field, so pairing is a lookup rather than a rescan
per date.

| target coordinate | destination |
| --- | --- |
| `IV.2.2`, `IV.3.4` | `submission_deadline` |
| `IV.2.7`, `IV.3.8` | `opening_date` |
| everything else | **no canonical date fact** |

Two decisions the issue left open, both settled by measurement rather than by preference:

**1. A block that states no target contributes nothing — and that costs nothing.** The strict
fallback is only safe if real corrigenda always state their target. They do:

    SELECT COUNT(*) AS date_rows,
           SUM(CASE WHEN EXISTS (SELECT 1 FROM notice_texts s
                                 WHERE s.notice_id = d.notice_id AND s.section_id = d.section_id
                                   AND s.field_id = 'TED-SECTION') THEN 1 ELSE 0 END) AS with_target
      FROM notice_dates d
     WHERE d.notice_id BETWEEN 21000000 AND 21020000 AND d.field_id = 'TED-NEW_VALUE.DATE'

**5,234 of 5,234.** Every one. So "no target stated" is a shape the corpus does not produce, and
refusing it drops no real correction. The one place it did appear was our own synthetic test fixture
(`an_f14_corrigendum_moves_the_deadline_as_a_version_event`), which published a `CHG-1` with a
`NEW_VALUE.DATE` and no `TED-SECTION` — a notice TED never emits. That fixture now carries
`TED-SECTION = "IV.2.2)"` and its assertion is unchanged.

**2. Both spellings are real, so the lookup normalises.** Every coordinate is published with and
without a trailing `)`, and the parenthesised form is the majority — `IV.2.2)` 2,051 against
`IV.2.2` 964, `IV.2.7)` 1,699 against `IV.2.7` 1,018 in the same window. A literal match would have
silently dropped whichever spelling the table omitted, which is the same class of defect one level
down. `f14_target_date` trims and strips the paren; the unit test asserts all four spellings.

**The r208 fixture the "Done when" asks for has no corpus to draw on.** `TED-NEW_VALUE.DATE` appears
**only** in the r2.0.9 id range. Counts over `notice_dates`, one bounded window per era:

| window | era | `TED-NEW_VALUE.DATE` rows |
| --- | --- | --- |
| 4,500,000–4,600,000 | text/r207 | 0 |
| 8,000,000–8,100,000 | text | 0 |
| 12,600,000–12,700,000 | r2.0.8 (2012) | 0 |
| 13,500,000–13,600,000 | r2.0.8 | 0 |
| 17,000,000–17,100,000 | — | 0 |
| **21,000,000–21,100,000** | **r2.0.9** | **20,739** |

`SELECT COUNT(*) FROM notice_sections WHERE kind = 'Change' AND notice_id BETWEEN 4400000 AND
13000000` is **0** as well: the r2.0.8 range holds no corrigendum sections at all, although
`rules.rs:683` files `ADD`/`DELETE`/`REPLACE` as `Kind::Change` and would emit the same field if one
arrived. The mapping carries the 2004-directive coordinates (`IV.3.4`, `IV.3.8`) so a future r2.0.8
corrigendum maps correctly on arrival, but **no r2.0.8 fixture is committed, because there is no
r2.0.8 carrier to build one from.** Committing a hand-written one would assert a shape nobody has
observed. If the era's corrigenda turn out to be filed under a different field id, that is a separate
finding and the count above is where to start.

Tests: `a_corrigendum_moves_the_opening_and_the_deadline_to_different_fields` (the 21000077 shape —
IV.2.2 at 09:00 and IV.2.7 30 minutes later; asserts the deadline is the earlier value, the opening
carries the later one, and exactly ONE deadline fact exists, since two indistinguishable ones is the
defect), `a_corrigendum_to_an_unmapped_section_contributes_no_date` (IV.2.6 six months out leaves the
CN's deadline standing), and `the_f14_target_vocabulary_normalises_and_refuses_by_default` (all four
spellings, the nine refused coordinates, every destination is a real canonical field, and the field
still reads as mapped so issue 368's sieve does not report it as dropped).

### Still open on this issue

- **The refusal is not observable.** A coordinate the table does not name is refused silently, which
  is the failure mode issue 364 spent a unit removing for citations. It wants the `CitationGate`
  treatment — a per-coordinate tally on the run's `Report` and the job row — so a new TED coordinate
  shows up as a number rather than as a date that quietly stopped being corrected. Not built here
  because the date mapping happens in `NoticeState::read` on the fold path, where no counter is
  plumbed; that plumbing is its own unit.
- **The refold and the re-measurement.** The carriers must be refolded and the census re-run to zero
  on both windows (the `## Done when` bullets). Not started: the box is processing the issue-395
  backfill.

## Unit 2 BUILT 2026-09-15 (owner) — the refusal is a number on the job row

Unit 1's strict fallback was correct and **silent**, which is the failure mode issue 364 spent a
unit removing one instrument over: a date that quietly stopped being corrected looks exactly like a
date nobody ever corrected. `F14TargetGate` in `crates/ingest/src/project.rs` is the `CitationGate`
treatment for it — same shape, same rail, same place on the `Report` and the job row.

| slot | meaning |
| --- | --- |
| `to_deadline` / `to_opening` | admitted, by destination |
| `to_other` | admitted to a destination with no slot — **structurally zero**, see below |
| `validity` / `duration` / `information` / `invitations` | refused, and classified (`IV.2.6`, `II.2.7`, `VI.3`, `IV.2.3`) |
| `other` | refused, and **unclassified — the only number anyone acts on** |
| `untargeted` | a new date with no `TED-SECTION` at all (0 of 5,234 measured, so any movement is news) |

**Where it is counted, and why there.** In `Ident::read`, the PLAN sweep — not in
`NoticeState::read` where the mapping happens. The plan sweep already carries `citations` to the
`Report` in both the full pass (`build_plan`'s parallel producer, via `PlanChunk`) and the
incremental one (`project_incremental_chunked_observed`), so the tally needed no new plumbing at
all: no change to `store::Applied`, none to the postcard `BucketRow` format, no atomics across the
pre-pass shards. The refusal is a pure function of the parsed layer, so reading it one phase earlier
costs an early-return predicate per notice and allocates nothing outside the r2.0.9 era.

Both wiring sites are real, and **each was a separate chance to ship a counter that reports zero
forever** — the first draft wired only the incremental one and the end-to-end test caught it by
returning `(0,0)` where it wanted `(1,1)`. The test now folds through both doors.

**The tally cannot drift from the mapping.** `count()` routes through `f14_target_date` rather than
re-testing the coordinate, so adding a coordinate to `F14_TARGET_DATES` starts counting it as
admitted in the same commit that starts mapping it — the property the `sentinel_amount` census arm
is built for. `to_other` is the tripwire for the one remaining way to drift (a new *destination*
with no slot), and `every_f14_destination_has_a_slot` fails the build if it can ever be non-zero.

**What it deliberately does NOT do: name the coordinate.** `Report` is `Copy` and a per-string map
is not, so `other` is a count, not a list. That is the accepted limit, stated rather than hidden —
and the identity is one bounded `GROUP BY` away, which the type's doc comment carries verbatim so
whoever reads a non-zero `other` does not have to reconstruct it. Naming coordinates in the counter
would mean either dropping `Copy` from `Report` (a ripple through every `report` use for a number
nobody reads weekly) or inventing labels for `II.2.4`, `II.2.14`, `III.1.3`, `I.3`, `II.2.2` and
`II.1.4` — coordinates that were counted but never identified. They sit in `other` rather than being
guessed into a slot.

`UNCLASSIFIED` prints even at zero, alone among the numbers on the line: a reader must be able to
tell "checked, still none" from "not measured". The rest of the line is silent when a run saw no
corrigendum date at all, which is every run that does not touch r2.0.9.

Tests: `every_f14_destination_has_a_slot` and
`the_f14_gate_separates_the_unclassified_from_the_merely_refused` (classification, both spellings,
`add` folding slot for slot), `the_run_reports_which_sections_its_corrigendum_dates_targeted` (a
four-block corrigendum — mapped, mapped, classified-refusal, unclassified-refusal — through the full
pass AND the incremental one, plus a corrigendum-free corpus reporting nothing), and
`the_f14_suffix_prints_the_unclassified_count_even_at_zero` in the supervisor.

### Still open on this issue

- **The refold and the re-measurement** (the `## Done when` bullets). Job 1384's projection is the
  unit-1 refold and is still running; the acceptance reads (6762566 → 09:00 + `opening_date` 09:30,
  6737588 → 2020-05-11T10:00, 6752749 → 2020-05-19T17:00, and the IV.2.7 census to zero on both
  windows) are for the next idle window. Unit 2 is not deployed yet either, so the first job row to
  carry the new line will be the projection after that deploy.
- **A corpus-wide coordinate census.** The counter says how many; `other` moving says someone should
  ask which. Doing that in the weekly DQ report was considered and NOT built: `TED-NEW_VALUE.DATE`
  lives only in the r2.0.9 id band, so `unmapped_fields_sql`'s newest-1M-ids window returns nothing
  for it, and the unwindowed form is a whole-corpus `GROUP BY` over `notice_dates` with no index to
  help (`field_id` is the 4th PK column) — the exact shape the issue-278 turso lesson warns about.
  It needs its own id-band window, and that band needs measuring on an idle box first.
