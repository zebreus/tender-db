# 268 — `unrepresentable-value` is the last FED quarantine bucket (5,196 held, 3,108 arrivals/30d)

Status: needs-triage — filed 2026-08-22 (owner, out of Lennart's quarantine question).
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
