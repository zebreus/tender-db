# 575 — per-notice hex keys outside `uuid_v4` (the FR 8-4-4-16 platform shape) split one organization into a row per notice

Status: needs-triage — filed 2026-10-19 from issue 530 unit 2's decision (`.scratch/tender-db/530-triage/decision.md`, "530-F"),
which declined to refuse 1/64 of a platform inside a v4-scoped issue. Next: unit 1 — size the raw 35-character 8-4-4-16 BT-501
shape across the whole hex-hash class (not only the `uuid_v4` passes), by platform, with 530's census definitions.
Kind: data quality (organization identity, FR): identifier admission, split side
Relates to:
- 530 (the v4 sub-class; its census `guid-census` job 2655 counted 3,511 single-use FR "ted other" rows of this shape, 0 spanning
  several Tenders, 0 stable, and 10,548 `nibble_only` values — the 3:1 ratio random hex gives);
- 312 / 300 design §84-85 (the "merge-inert singletons" stance; 530 upheld it for the class and amended it for the twin
  sub-class);
- 365 (the `national` catch-all that admits these values).

## Verify

    python3 -c "import json;d=json.load(open('.scratch/tender-db/530-triage/guid_census_2655.json'));b=d.get('body',d);b=json.loads(b) if isinstance(b,str) else b;print(b.get('nibble_only'), [p for p in (b.get('by_platform') or []) if 'other' in str(p)][:3])"

- **done**: a decision is recorded here (refuse the shape at the gate with an R5-style read passed, or decline with the cost
  written down) and, if refused, the standing rows are dissolved and a fresh census reads 0 single-use rows of the shape.
- **open (2026-10-19)**: job 2655 reports 10,548 nibble-only values and 3,511 single-use `ted other` v4 rows of the FR shape;
  the whole shape population (estimated ~225k) is unmeasured.

## What was found (issue 530 unit 2)

- One FR platform (Avenue-Web, per the 530 judges' reads; notice 277 carries `FD29FC2F-A2BF-49A1-9B234B870B531C27`) publishes
  BT-501 as 35 hex characters with dashes at offsets 8, 13 and 18 only (8-4-4-16), apparently a fresh random key per ORG section
  per notice.
- About 1/64 of random hex passes `idgate::uuid_v4` (version nibble 4 and variant 8-b), 3/64 carry the version nibble but fail the
  variant (`nibble_only`). Job 2655: 3,511 v4 passes in the FR `ted other` cell (all single-use, 0 multi-Tender, 0 stable) and
  10,548 nibble-only values — consistent with ~225k rows of the shape overall.
- Avenue-Web Systèmes also has 191 one-mention 32-hex rows (6 of them v4).

## Proposed units

1. **Size (S/M).** Extend `guid-census` (or a sibling arm) to the raw-shape class: every organization whose stored identifier is
   32 hex and whose mentions' raw BT-501 is the 8-4-4-16 shape, regardless of `uuid_v4`; report single-use / multi-Tender / stable
   by platform and year, and whether any value recurs.
2. **Decide (S).** Starting bar = 530-F's R5: every mention carries the 35-character shape, 0 rows of the shape span ≥ 2 Tenders,
   0 are stable, and 15 FR fallback targets read 0 W. Refuse at the gate by the raw shape (the publisher's own format, a category
   argument like idgate's `notice_number`) or decline with the cost recorded.
3. **Build and dissolve** per 530's mechanics (per-mention rehome/dissolve, frozen keep-list if any value recurs).
