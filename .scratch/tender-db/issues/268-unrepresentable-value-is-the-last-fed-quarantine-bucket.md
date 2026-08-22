# 268 — `unrepresentable-value` is the last FED quarantine bucket (5,196 held, 3,108 arrivals/30d)

Status: CLOSED 2026-08-22, filed-to-drained in one day — deployed (845ed54), reprocess job 308
reclaimed **4,898 of 5,196 (94 %)**, the paired fold wrote 3,740 tenders, and the ledger carries
the resolution row. The 298 still held are the garbage class (10^50 integers) and hold by design;
quarantine's outstanding total is now ~306 across the whole corpus, all diagnosed. Was: DIAGNOSED
AND FIXED IN CODE 2026-08-22, same day — the bucket split cleanly with two
bounded reads of the held rows' own `detail` strings (no archive sampling needed):

- **3,249 (62 %) "more than two fraction digits"** — publisher mills (`555.242`), float
  artifacts (`893513.4400000001`), deep trailing zeros. NOT garbage: sub-cent precision the
  cents policy refused. FIXED: `cents()` now rounds half-away-from-zero to the cent (the archived
  member stays byte-faithful; the canonical layer is a projection; error ≤ half a cent). Gates:
  the unit table pins the real held samples; `sub_cent_amounts_round_to_the_cent` pins it at
  payload level (336.13445 → 33,613). This class was the DAILY FEED.
- **1,574 (30 %) "OPT-999: no zone offset"** — ALL predate issue 195's OPT-999 exception (newest
  hold 2026-08-13; the fix deployed ~08-16). Zero code needed: they drain on reprocess under the
  current parser.
- **282 (5 %) "not an integer"** — the 10^50-class garbage (e.g. BT-113 with fifty zeros).
  Verdict: hold forever; this is what the gate is FOR. Remainder (~91) mixed small shapes.

Remaining: deploy, run the reprocess over the bucket, ledger row with the reclaim counts, and the
282-class verdict note. No epoch move — quarantined members never entered the layer, so stored
chains are untouched (the 234 rule). Was: needs-triage — filed 2026-08-22 (owner, out of
Lennart's quarantine question).
Kind: data-quality investigation → drain or verdict
Blocked by: —
Relates to: 40 (the ledger this ends in), 137 (the outstanding-vs-total honesty), 267 (the >1e12
tripwire for this class ESCAPING into the layer — this issue is about the members correctly held
at the gate), ADR-0004/0010 (hold-and-diagnose)

## Where quarantine stands (2026-08-22, dashboard)

2,419,499 ever held → 1,812,657 reclaimed (75 %) + 601,638 skipped-by-policy + **5,204
outstanding**, of which 8 are source-corrupt zips (permanent residue) and **5,196 are
`unrepresentable-value` — the ONLY bucket still being fed: 3,108 arrivals in the last 30 days,
newest 2026-08-21. Suspected parser gaps: zero. This bucket is the whole remaining live cost of
quarantine.

## The question

An unrepresentable value is an amount the parse refuses to store (overflow-class magnitude,
unparseable numeric shape). Two populations can hide under one reason:

- **Source garbage**: the notice genuinely publishes `999999999999999999` or `1,2E+15` — holding
  is correct forever; the verdict goes in the ledger and the bucket becomes settled residue.
- **Representation too narrow**: legitimate large-currency amounts (IDR, VND framework ceilings
  can exceed 1e15 minor units) or locale shapes our numeric parse rejects — a fix reclaims them.

The split decides everything and a sample answers it.

## Plan

1. Sample ~50 held members across arrival dates (the reason rows carry member paths); read the
   raw values from the archive.
2. Classify: garbage vs legitimate-but-unrepresentable, per currency/era.
3. If a representable class exists: widen (i128 cents? currency-scaled representation?) behind a
   test per real sample, reclaim the bucket, ledger row.
4. If all garbage: ledger verdict + a caveat in the report's section 5 note, so the fed bucket
   stops reading as unaddressed work.

## Acceptance

The bucket either drains (reclaim count in the ledger) or carries a written verdict; either way
`unrepresentable-value` stops being the unexplained live bucket. Issue 267's tripwire stays the
guard for the values that make it PAST the gate.
