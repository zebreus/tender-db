# 410 — the continuity check tests the days IMMEDIATELY adjacent to a stretch, so one straggler notice hides a 432-day publication gap

Status: needs-triage — filed 2026-09-17, measured end to end on prod. `fts` has a **432-day silent
stretch inside the continuity window** and section 14 reported `none`. The mechanism is exact and
arithmetic, not a judgement call: the day after the gap carries **1** notice against a floor of
**165**, so the stretch is disqualified by the bracket rule while the first ordinary day sits one
day further on carrying 492.
Kind: defect (data-quality instrumentation) — the same class as issue 402 itself: a completeness
signal reading green over a hole
Relates to: 402 (built this section; its unit A added the bracket rule, and its unit B made the
window publication-time — this is the next layer of the same check), 342 (the FTS source, whose
plan step 11 backfill is why the DATA gap is expected; the detector's silence is not), 368 unit 4b
(a diagnostic that ran and showed nobody anything — same shape, different report)
Blocked by: nothing

## Observed

Found while verifying issue 402's namespace fix on the live dashboard: `fts` renders
`daily 2026-09-07 … 2026-09-15 · monthly 2025-06 … 2025-06`. Served probes confirm the shape —
`/v1/tenders?source=fts` returns rows for 2025-06 and for 2026-09-07 onward, and nothing for
2025-12, 2026-06.

Every `fts` publication day, one bounded read (`notices(source, member_path)` has `source` leading,
and the whole source is 10,600 notices):

| era | days | shape |
| --- | --- | --- |
| 2025-06-01 … 2025-06-30 | 30 | ~274–402 on publishing days, 1–6 on weekends |
| **2025-07-01 … 2026-09-05** | **0** | **432 silent days** |
| 2026-09-06 | 1 | **one notice** |
| 2026-09-07 … 2026-09-15 | 8 | 415–577, and 3 on the Sunday |

## Why section 14 said `none`

`publication_gaps` walks CONSECUTIVE present days and requires both bracketing days to be ordinary —
at least [`ORDINARY_BRACKET_SHARE_PCT`] (50 %) of that source's median day in the window. For `fts`:

    39 days measured, median 330  ⇒  floor = 165
    stretch 2025-07-01 … 2026-09-05, 432 silent days
    before = 2025-06-30 → 329   ≥ 165 ✓
    after  = 2026-09-06 → **1** < 165 ✗   ⇒ not listed

So a single notice on 2026-09-06 — one row, the day before the source genuinely resumes — is the
entire reason a 432-day hole renders as `none`. Move that one notice and the gap appears, at 433 days,
bracketed 329 / 492.

**The bracket rule is not wrong; its REACH is.** Issue 402 unit A added it for a real reason: the
first corpus run printed two stretches whose brackets were 3,765/1 and 1/3,534, artifacts of an id
window that barely sampled those days. The rule correctly refuses to treat a barely-sampled day as
evidence. What it then does is *discard the whole stretch* rather than *look one day further* for a
day that IS ordinary.

Note also that unit A's semantics shifted under it when unit B landed. Under the old ingest-ordered
window a 1-count day meant "the window clipped this day". Under a publication-time window it means
"the source published one notice that day" — a straggler, which is exactly what sits at 2026-09-06.
The rule was calibrated against the first reading and is now being applied to the second.

## Is the fts gap itself a defect?

**No, and that matters for how this is prioritised.** Issue 342 took FTS live with a measured pilot
month (2025-06) and its 69-month backfill is plan step 11, not yet run. The DATA is expected. The
detector's SILENCE is not: section 14's own caption says a source with a different rhythm "may show
a benign entry here; read it against that source's calendar". A benign entry is the designed
outcome. Nothing is what this issue is about.

That makes this a clean test case, incidentally: a known, explainable, 432-day gap that the check
must surface and currently does not.

## Fix

**Measure stretches between consecutive ORDINARY days, not between consecutive present days.** Filter
the day list to those at or above the floor, then look for gaps in the filtered list. The brackets
are then ordinary by construction and the predicate disappears into the filter.

This would have caught both of the holes the section exists for — 402's TED blackout (brackets 3,470
and 3,722, both ordinary, unaffected) and this one (brackets 329 and 492) — while still suppressing
unit A's artifacts, since a 1-count day is never a bracket. It also removes the awkward case where a
stretch's length depends on whether a straggler happened to land beside it.

Two things to get right:

- **The reported span.** With filtering, `from`/`to` should stay the true silent range (the day after
  the last ordinary day, to the day before the next), so a reader is not told a day with 1 notice was
  silent. The low-count days inside the stretch are worth a column — "3 barely-sampled days inside" —
  rather than being erased.
- **A source whose ordinary days are all sparse.** A source publishing 1–2 notices a day has a
  median of 1–2 and a floor of 0, so every day is ordinary and nothing changes for it. That is
  correct and worth a test, because it is the case where the filter could silently empty the list.

## Verification

Re-run `data-quality` and confirm section 14 lists `fts 2025-07-01 … 2026-09-06, 433 days,
before 329, after 492`, and that `ted` still reports nothing. Both arms in one run — a rule that
flags everything is as useless as one that flags nothing, which is the control issue 395 and 402
both used.
