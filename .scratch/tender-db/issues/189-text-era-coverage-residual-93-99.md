# 189 — text-era coverage residual: 1993 and 1995–1999 miss ground truth by 7–22%, and the loss is upstream of quarantine

Status: RESOLVED-VERIFIED (2026-08-12) — re-vendored distributed counts live; grid reads 1.00–1.02 across 1993–1999 (was 0.78–0.93)
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

## Comments

**2026-08-10 (orchestrator)** — Lennart delegated the ground-truth stance; decided: **the vendored
ground truth is a claim to verify, not a fact** — but the dashboard changes only on evidence, not
suspicion. Concretely: (a) investigate 1999 first (the outlier, cheapest to falsify) by diffing
the fetch registry against what TED's archive channel actually offers for 1999 vs the clean 2000;
(b) if we missed packages, backfill them (issue 15 machinery) and re-measure — the ratio is ours
to fix; (c) if the archive cannot substantiate the ground-truth figure, the coverage panel gets a
per-year ground-truth caveat (a published count the source cannot substantiate must not read as
our gap), and we do NOT pursue recovery beyond what the archive holds — no heroics for data that
does not exist. Prod reads this needs are bounded (fetch-registry seeks) but still gate on the
team lead's word per read (docs/agents/prod-box-reads.md); prefer the public pipeline panel,
vendored ground truth in-repo, and TED's public listing where they answer the same question.

**2026-08-11 evening (orchestrator) — the 1999 arm is ANSWERED, on-box archive sweep.** Method:
every 1999 monthly tar extracted month-by-month (box idle, Lennart's go-ahead), every language
zip of all 254 dailies unzipped, `ND:` document numbers collected: 1,791,471 ND lines across all
languages → **162,861 distinct** — the EXACT held count for 1999, to the row. Editions are
continuous (1999001–1999254, none missing; all 12 monthlies present, sizes comparable to 2000).
And the kicker: ND numbers span **1 to 209,009** — precisely the vendored ground-truth figure.
So the "published" series for 1999 is the Office's ASSIGNED document-number counter; **46,148
numbers (22%) are gaps that no daily delivery ever carried**. Two independent paths (ingest
pipeline and this sweep, different code) agree exactly: the pipeline ingests 100.000% of what
TED distributed. Per the decided stance (c): this is a ground-truth caveat, not our gap, and no
recovery is possible or warranted. Fix shape: re-vendor text-era ground truth as DISTRIBUTED
counts (derivable by this exact method) or annotate the coverage panel per-year; recommend
re-vendoring — it makes the ratio honest instead of footnoted. Remaining before closing: run the
same count for one more bad year (1995 or 1998) to confirm the pattern generalizes, then land the
vendored-count fix. 1994's 1.015 ratio fits the same story (documents distributed can exceed the
year's assigned numbers when prior-year numbers ship late).

**2026-08-12 morning (orchestrator) — 1995 confirms the pattern.** Same sweep over all 12 1995
monthlies: **max ND = 138,824 — the vendored ground-truth figure exactly**, assigned-counter
theory confirmed on a second year. (The sweep's distinct count under-read at 120,588 vs 127,770
held because plain grep treated 32 record files as binary and swallowed their lines — use
`grep -a` in any rerun; ALSO a lead for issue 181: 1995-era record files contain bytes that make
grep call them binary.) The generalization holds; remaining work is only the fix: re-vendor
text-era published counts as DISTRIBUTED documents (per-year distinct-ND by this method, grep -a)
and re-measure the coverage grid — 1993–1999 should all go green at ~1.00.

**2026-08-12 midday (orchestrator) — RESOLVED: all seven years swept, fix landed.** Full sweep
of 1993–1998 (all languages, grep -a, widened case-insensitive zip matching), plus 1999 from the
11th. max ND == the old vendored figure EXACTLY on 1993/1995/1996/1997/1998/1999; 1994's max
(97,362) even exceeds its vendored 94,954 — the old method sampled the year's LAST daily, which
didn't carry the year's max, which is exactly why 1994 read an impossible 1.015. Distributed
distinct-ND per year: 66,521 / 94,457 / 126,385 / 138,533 / 152,339 / 160,892 / 162,861. Fix:
`ted-notice-counts.csv` re-vendored for 1993–1999 with the method documented in the header, and
the research doc (§6) carries a correction note preserving the superseded counter series. New
held/published ratios: 1.010 / 1.020 / 1.011 / 1.015 / 1.019 / 1.016 / 1.000 — all inside the
±2% verify band, all marginally ABOVE 1.0 because TED reused ~1–2% of document numbers within a
year and the pipeline rightly holds each reused number as its own record (1993/94 are
single-language years: their line counts equal held almost exactly, confirming the reuse
reading). CAVEAT recorded: 1994 passes the +2% side by only 6 notices, and future text-era
reclaims (e.g. the RP co-financing 3,326) will push held UP against these denominators — if
verify ever flags text-era years Over, that is this known reuse effect, and the refinement is
counting reused numbers into the denominator, not a duplicate-ingestion hunt. Distributed-count
sweep method preserved in this issue + the CSV header; sweep artifacts in /root/sweep189 on the
box (summary.txt).
