# 312 — Graduate the v4-GUID identifier class to the deterministic deny-floor

Status: ready-for-agent
Kind: data quality / ingest gate (idgate)
Relates to: 311 (found by the review campaign), 300 (canonical keys)

## What the review found

The issue-311 batch campaign measured that **306 of 823** identifier-bearing
Bietergemeinschaft rows carried a 32-hex **v4 UUID** (version nibble '4' at
hex position 13, variant 8/9/a/b at 17, stored undashed) in the national-
identifier field — platform-generated record keys leaked into the id slot by
one submission platform, structurally impossible as any EU registration or
VAT. Every one was individually reviewed and stripped (per-case verdicts in
`311-batch-verdicts.json`); the class is perfectly rule-shaped in hindsight.

This is Lennart's feedback loop working as designed: individual AI review
finds the class, and once a class is proven rule-detectable it GRADUATES to
the deterministic floor so the gate catches future instances at ingest.

## The change

- `idgate`: condemn a candidate identifier whose normalized form is 32 hex
  chars with the v4/variant nibbles (dashed or undashed) — same handling as
  the existing placeholder condemns (no org identity minted from it; raw
  value stays on the mention).
- Scope check first: one bounded corpus query for how many NON-consortium
  org rows also carry v4-GUID identifiers (the platform surely leaks them
  on plain company rows too) — that number sizes a follow-up strip cohort
  which still goes through per-case review (the floor only PREVENTS new
  ones; standing rows keep the review bar).
- Tests: nibble arithmetic pinned both dashed and undashed; a real GUID
  specimen from the campaign; a 32-hex NON-v4 value must pass through.

## Also noted by the campaign (smaller follow-ups)

- Register-format impossibility as a REVIEW CALIBRATION (not auto-strip):
  FN/HRB/HRA numbers on unregistered GbR/GesbR-shaped consortium names were
  the recurring medium-band class in both audit rounds. Keep as reviewer
  calibration; too much legal-form nuance for the floor.
- Phone numbers and postal codes in the identifier slot (t:-prefixed, area
  codes) — candidates for the same floor treatment as GUIDs; measure first.
