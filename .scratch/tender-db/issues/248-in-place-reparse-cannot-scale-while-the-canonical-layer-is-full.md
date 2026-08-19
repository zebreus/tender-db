# 248 — an in-place re-parse cannot scale while the canonical layer is full: the FK proof is not index-served

Status: needs-triage — measured 2026-08-19 (issue 247's investigation, split out because it outlives that
issue's campaign)
Kind: engine-level constraint on the re-parse mechanism, with an operational consequence
Blocked by: —
Relates to: 247 (where it was measured), 244 (the campaign it blocks), 100 (the re-parse mechanism), 179
(what a full rebuild costs), 133 (guards for an emptied canonical layer), 63 (drop-and-recreate rather
than per-row DELETE), 239 (another turso planner gap)

## The constraint, in one line

Deleting one `organization_mentions` row costs **~2.2 seconds** while `tender_version_parties` holds
78,033,566 rows, because proving that no child row references it is not served by an index on the write
path — and every in-place re-parse must delete a notice's mentions before it can replace its sections.

## What was tried, and what each was worth

All measured on prod through `tender_db_reparse_clear_statement_seconds_total`:

| change | per notice |
|--------|-----------|
| baseline (prefix DELETE, no index, notices that had nothing to delete) | 153 ms |
| a row actually deleted | **10.2 s** |
| `organization_mentions_notice` built (41.78M rows) | 10.2 s — no change |
| `PRAGMA defer_foreign_keys = ON` (accepted, nothing logged) | ~4 s |
| DELETE by the FULL primary key, one statement per mention row | **2.2 s** |
| **the same notice with no mentions at all** (the seek skips the DELETE) | **~1 ms** |

That last row is the shape of the answer. A package whose notices had already been cleared re-parsed 870
notices in 80 seconds; the same mechanism on notices that still carry their mentions manages 0.45 a second.

Reads are not the problem and never were: `SELECT` with the same predicate, including the full FK pair
`(mention_notice_id, mention_section_id)`, answers in 1 ms.

## The consequence

The text era is 3.79M notices. At 2.2 s each that is 2,300 hours, so **an in-place re-parse of a whole era
is not available while the canonical layer is populated** — not for issue 244's awards, and not for any
future parser change that wants to re-read a large cohort.

`clear_canonical` (used by `project {rebuild: true}`) drops and recreates exactly the tables whose size
makes the proof expensive, `tender_version_parties` and `tender_version_bid_parties` among them. With those
empty the proof is instant and the era re-parses at the mechanism's real rate — 50–240 notices a second,
measured, so 4–21 hours for the era.

## The three options, sized

1. **Piggyback the next full rebuild.** Keep the extraction code (landed and tested) and re-parse the era
   in the window where the canonical layer is already empty. Costs nothing extra; the awards stay
   unextracted until a rebuild happens for its own reasons. **Recommended.**
2. **Open a rebuild window on purpose**: clear → re-parse the era (4–21 h) → rebuild. Delivers the awards
   now, at the price of serving an empty canonical layer for the whole window. Issue 133's guards mean the
   emptiness is at least visible rather than silent, but the API and dashboard are effectively down for it.
3. **Make the FK proof cheap.** The constraint is `tender_version_parties(mention_notice_id,
   mention_section_id) → organization_mentions`. An index on the exact pair is the obvious try — but the
   single-column index already there did nothing for the DELETE, and reads already seek, so there is
   little reason to expect it to. The alternative is dropping the constraint, which trades a real integrity
   guarantee for throughput and should not be done to work around a planner gap.

Option 1 needs no decision today beyond writing it down, which is what this issue is for. Option 2 is the
one that needs an owner's call, and the number that decides it is how long the window is: 4 hours is a
maintenance window, 21 hours is not.

## What would change the picture

- A turso version whose DELETE and FK enforcement use available indexes. Worth re-measuring on the next
  bump (issue 166's precedent) — this issue's numbers are the before.
- A schema in which parties reference the parse layer rather than the mention (bigger, and it would want
  an ADR).

## Acceptance

- The era's awards land, by option 1 or 2, with the density in section 3 of the data-quality report as the
  witness.
- Whichever path is taken, the sizing above is checked against reality once and corrected here.
