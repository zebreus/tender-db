# 270 — five text-era amounts over 1e12: the 267 escape tripwire has fired

Status: needs-triage — filed 2026-08-23 (owner) from data-quality run #335, the campaign's
acceptance read
Kind: data-quality (suspected extraction defect, bounded: 5 rows)
Blocked by: —
Relates to: 267 (the tripwire that caught it), 244 (the text-era money extraction these came
from), 268 (the class this gate exists to keep OUT of the layer)

## What

Section 8 of the first post-campaign data-quality run (#335, 2026-08-23) shows the text era
with **5 amounts > 1e12 cents** (of 445,193 projected text-era amounts; zero negatives,
18 zeros). Before the campaign the era projected essentially no amounts, so these five are
new — produced by the issue-244 prose-body money extraction.

Issue 267's own framing: `over_1e12` is "a deliberately crude tripwire for the
unrepresentable-value class escaping quarantine into the layer". At the gate (268) the
10^50-magnitude garbage is held; these five are IN the layer, so either

1. the source genuinely published a >€10B amount (possible but rare — that is why the
   tripwire is a rate, not an invariant), or
2. the text-era value parser mis-joined digit runs (thousands separators, appended
   reference numbers, currency-name concatenation — the prose-body shapes slice 7/8 dealt
   with) and fabricated a magnitude.

Five rows decide it either way, and (2) would be a parser defect worth a slice-9 fix plus a
scoped re-parse of affected packages.

## How to find them (bounded)

No index on `cents`, so no direct public-SQL scan. Windowed, per the DQ pattern — one
bounded probe per 250k-tender window, only windows the campaign touched, reading
`tender_version_amounts a JOIN tender_versions v … JOIN notices n` filtered
`n.profile = 'text' AND a.cents > 100000000000000` with a tender_id range predicate
(indexed drive off `v.tender_id`). Or simpler: run it on the snapshot box (269's snapshot
exists now) where a 40s scan harms nobody.

For each hit: pull the notice's archived body (`/v1/notices/{id}` raw payload), read the
published prose, compare. Verdict per row → fix or documented-keep, then a ledger-style note
here.

## Acceptance

- The five rows are identified and each carries a verdict (source-published vs fabricated).
- If fabricated: the parse defect is named, fixed with the slice test-style fixture, and the
  affected packages re-parsed + re-folded; section 8's next run shows the era at 0 (or at
  its source-published truth).
- 267's framing is honoured: the tripwire read stays a rate; no per-amount "fix" that
  rewrites source-published values.
