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
