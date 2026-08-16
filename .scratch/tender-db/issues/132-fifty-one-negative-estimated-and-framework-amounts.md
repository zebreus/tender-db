# 132 — the 51 negative `estimated_value` / `framework_maximum` rows the re-spec did NOT explain

Status: RESOLVED-DIAGNOSED (2026-08-16, owner) — row-by-row inspection done; **source-published, both
layers faithful, no defect in our code**. Determination below; issue 160 stays a pointer here.

## Determination (2026-08-16, owner — the row-by-row pass the narrowing bought)

Method: enumerated the negative `estimated_value`/`framework_maximum` rows over the 2026-forward slice
via bounded `/v1/sql` chunks (22 view-rows, 16 distinct tenders), then walked every one of the 16 through
the public API — `/v1/tenders/{id}` (canonical), `/v1/notices?tender=` (chain), `/v1/notices/{id}/content`
(parse layer, the 218-B endpoint) — and read two RAW ARCHIVE DOCUMENTS at the extremes of era and SDK.

**Q1 — P4 holds for every row checked.** All 16 tenders carry equal-magnitude negatives in their chains'
parse layers (BT-27 / BT-271 / DE1-…-EstimatedOverallContractAmount / …-FrameworkMaximumAmount). The fold
copies faithfully; issue 131 stays closed. (Issue 33's 17,738/17,738 exact-magnitude match already
included these rows; this confirms it per-row for the suspect subset.)

**Q2 — two clean strata, all eForms-era (sdk-1.7→1.13, eforms-de-1.1→2.1):**
- *Sentinels* (7/16 tenders): exactly −100 cents (−€1.00), plus one −300 (−€3.00) pair — German
  eforms-de publishers stamp −1.00 on `BT-27-Procedure` across entire chains (tender 858517: twelve
  notices, every one −1.00). This is the data-profile study's §2.1 sentinel pattern, now seen at the
  canonical layer: **−1.00 is a publisher "no value" marker, not a value.**
- *Real-magnitude negatives* (9/16): −€30k to −€11.8M plus −6M DKK, EUR/DKK, scattered across sdk-1.7,
  1.9, 1.10, 1.13 and four years — no version cluster, so no mapping fault to point at.

**Q3 — the minus is in the published document.** Two raw XMLs read from the archive:
- 2026, sdk-1.13, notice `00149565-2026` (tender 152904): literal
  `<cbc:EstimatedOverallContractAmount currencyID="EUR">-420000.00</…>` — and the canonical detail shows
  it as LOT-0006's estimate among positive sibling lots (€0.68M–€6.94M), i.e. a publisher data-entry
  artifact, published verbatim by TED, copied verbatim by us.
- 2024, sdk-1.7, notice `00243502-2024` (tender 98871): literal `-6000000` DKK in BOTH
  `EstimatedOverallContractAmount` and `TotalAmount`.
The delta-mapped-as-absolute hypothesis (Q3's motivation) is dead: nothing is being transformed — the
sign arrives on the wire, in both eras, in both fields.

**Outcome:** no code change. "Store as published" is the correct behavior and is what happens. The one
remaining decision is the data-profile study's proposed **rule 8** (§3): flag exactly-−1.00 amounts as
`sentinel` and exclude them from canonical value columns — a canonical-semantics change, to be decided
deliberately, not slipped in here. Until then the standing gate keeps pointing at these rows by design.
Scope note: the per-row pass covered the 2026-forward slice (16 tenders); the historical slice is covered
by the global 17,738/17,738 P4 and the same-era document checks above.

Was: open — **CANONICAL** for this follow-up; issue 160 is a pointer here. The residual left by issue
33's re-spec. **Not noise, and deliberately not swept under the narrowed invariant.** Small enough to
inspect row by row.
Kind: data quality
Owner: unassigned (proj-fix filed; the parse-vs-fold half of 33 was mine)
Relates to: 33 (the triage + re-spec), 131 (the parse-vs-fold determination), `run_light` 3.7,
`standing_gate` `negative_amount`

## What is left

Issue 33 established that negative amounts are published by the source and faithfully copied by the fold
(P4 = 0; 17,738/17,738 exact magnitude matches against the chain's parse layer). 3.7 was re-specified to
permit negatives on `result_value` — 17,687 of the 17,738, award/result adjustments, 99.6% between EUR 1
and 100.

That leaves **51 rows the explanation does not cover**:

| field | rows |
|---|---|
| `estimated_value` | 29 |
| `framework_maximum` | 22 |

A negative *result* value is an adjustment and makes sense. A negative **estimated** value, or a negative
**framework maximum**, does not: an estimate of what a procurement will cost, and a ceiling on what may be
spent under a framework, are quantities with no meaningful negative reading.

## Why this is filed rather than closed with 33

The re-spec narrows *17,738 unexplained* to *51 genuinely-suspect*. That is the whole value of narrowing
rather than relaxing — the check now points at something specific instead of at everything or nothing.
Letting these 51 disappear into a passing gate would reproduce, at 1/350th the size, exactly the failure
issue 33 exposed: an invariant that stops telling anyone anything.

They are also **where a real defect would live**. Issue 33's conclusion (the source publishes them, both
layers are faithful) was established over the whole 17,738 and is dominated by the `result_value` mass. It
does not follow that it holds for these 51 — a mapping fault putting the wrong field's value into an
estimate, for instance, would look exactly like this and would be invisible in the aggregate.

## What would settle it

Small enough to inspect row by row (51 rows), which is the luxury the narrowing bought:

1. Do these 51 also satisfy P4 — an equal-magnitude negative in the chain's parse layer? (If not, the fold
   half of issue 131 needs re-opening for this subset.)
2. What are their source notices, profiles, and magnitudes? A cluster on one profile or SDK version points
   at a mapping fault; a scatter points at genuinely odd publications.
3. Do the published documents actually carry a minus on that field, or is a sign arriving from somewhere
   else (an unmapped element, a delta mistaken for an absolute)?

Question 1 is cheap and decisive about *our* code; 3 is the one that needs a human reading a notice.


---

## Merged from issue 160 (sdk-vendor's triage-side evidence)

We filed this same follow-up within minutes of each other from opposite ends of #33 — I from the fix
side, sdk-vendor from the triage side — and then each deferred to the other, briefly leaving two issues
pointing at one another and none canonical. Resolved: **132 is canonical, 160 is the pointer.** Their
evidence, kept so it is not lost with the number:

**Already established, so nobody re-derives it:**

- **Not the fold** — P4 = 0 and 17,738/17,738 exact magnitude matches against the chain's parse layer.
  *But those are whole-set figures*, dominated by the 17,687 `result_value` rows; whether these 51
  individually satisfy P4 is the open question below, and the aggregate does not answer it. (sdk-vendor
  caught themselves making that over-read in their own file while deduping.)
- **Not a parser-version defect** — the full set scatters across 13 SDK versions (1.6→1.14 plus DE) and
  4 years. A version-specific mapping fault would cluster.
- **Magnitudes differ from the benign mass** — €420k and €11.8M scale, against a `result_value`
  population that is 99.6 % between €1 and €100. These do not look like adjustment lines.

**The open questions, sdk-vendor's third being the one to chase first:**

1. Do the 51 cluster on profile/version/publisher *once isolated from the 17,687*? Whole-set scatter
   does not rule out a cluster inside the subset, and nobody has looked.
2. Is a negative `estimated_value` a published **sentinel** rather than a value? The recurring
   exactly-`-100` (−1.00) pattern across several currencies suggests sentinels exist in this data.
3. **Is some field whose published semantic is a DELTA being mapped as an absolute ceiling?** Invisible
   from the sign alone, and the hypothesis that best explains why a *ceiling* — which cannot meaningfully
   be negative — carries one. My area; this is where I would start after P4.

**Framing worth keeping verbatim** (team-lead): *17,738 unexplained → 51 worth explaining is progress,
not resolution.* The failure it guards against is using a mostly-benign explanation to wave through the
part it does not cover.

2026-08-09 (data-profile study, docs/research/data-profile-2026-08.md
§2.1): sample evidence for Q2 — in the 2024 eForms window ALL 99 negative
notice_amounts rows are exactly -100 cents across BT-720/710/711/161 etc.:
-1.00 is a publisher sentinel, not a value. Proposed rule 8 (§3): store as
published, flag sentinel, exclude from canonical value columns. The
larger negatives (eur 420k / 11.8M) remain real suspects.
