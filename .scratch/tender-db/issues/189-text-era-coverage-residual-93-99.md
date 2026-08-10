# 189 — text-era coverage residual: 1993 and 1995–1999 miss ground truth by 7–22%, and the loss is upstream of quarantine

Status: needs-triage
Kind: coverage investigation
Blocked by: —
Relates to: 35 (OC reclaim — landed, did not close this), 72 (measured ~92% and predicted bounded real loss), 27/30 (the coverage grid and its ±2% tolerance), 15 (backfill)

## Why

Post-OC-reclaim coverage, TED `text` profile (public `/api/dashboard`, 2026-08-10):

| year | held | published | ratio |
|---|---|---|---|
| 1993 | 67,192 | 74,433 | 0.903 |
| 1994 | 96,340 | 94,954 | **1.015** |
| 1995 | 127,770 | 138,824 | 0.920 |
| 1996 | 140,576 | 151,945 | 0.925 |
| 1997 | 155,185 | 166,394 | 0.933 |
| 1998 | 163,395 | 177,012 | 0.923 |
| 1999 | 162,861 | 209,009 | **0.779** |
| 2000+ | | | ~1.000 |

Three facts make this an unowned gap rather than a known one:

1. **The 576,743-row OC reclaim did not move these ratios** — they still read ~92%, the same as
   the 2026-07-21 measurement that issues 35/72 quoted. The reclaimed OC notices were evidently
   already-represented duplicates, as issue 72's bounded-loss analysis predicted.
2. **The shortfall is bigger than all remaining text-era quarantine.** Missing notices across
   these years ≈ 79k (46,148 in 1999 alone), while every still-held text-attributable quarantine
   bucket together is ≤ ~5k (RP 3,326, not-utf8 1,700, translation-structure 94). So ~90% of the
   gap never reached quarantine at all: not fetched, not dispatched, or the ground truth counts
   things the archive does not carry (ted-access-channels.md notes the 90s zips started
   English-only — the ground-truth series may include publications with no archived XML).
3. **The shape is diagnostic**: 1994 at 101.5% and 2000 at 100.0% bracket the gap years, and 1999
   is 2.5× worse than its neighbours. A uniform parser defect does not do that; an
   archive-composition or fetch-registry story might.

## What

1. Per-year, diff the fetch registry against the TED archive listing for one bad year and one
   good year (1999 vs 2000): are there packages we never fetched, or packages whose member count
   falls short of ground truth?
2. If the archive itself lacks them: document the ground-truth caveat on the coverage panel
   (a `published` figure the source cannot substantiate must not read as our gap — the issue-30
   honesty rule, applied to coverage).
3. If we missed packages: backfill them (issue 15 machinery) and re-measure.
1999 first — it is the outlier and the cheapest to falsify against.
