# 520 — an award summary stands as one winner org where the notice names a winner per lot

Status: needs-triage — filed 2026-10-12 with issue 511 unit 2 (decision §5).
Kind: data quality (results; organizations)
Relates to:
- 510 (`partyname::names_an_award`);
- 511 (the `AwardSummary` class, which the fold keeps);
- 491 (per-award figures in the text era).

## What is wrong

About 500 orgs are named after a whole award list instead of a body. Examples:
- `Lot 1) Sarl Bremond. Lot 2) S.A. Les Rapides Varois. Lot 3) Déclaré infructueux`
- `lot 43 INFRUCTUEUX - 44 reckitt benckiser 28692.07 - …`
- `Infructueux … attribué à l'entreprise Promocash`

510's drain keeps them: they are listed in the `void-names-refold` report's `exempted`, job 2182. 511 classes them
`AwardSummary`, and the fold keeps them.

The right outcome is one of:
- **(a)** parse per-lot winners (and figures) out of the summary into their `RES-n` lot results, and drop the summary
  org; or
- **(b)** drop the summary, leaving each result without a winner, where parsing is not reliable.

## Proposed units

1. **Measure.** Read the ~500 names by era and publisher, and class each one:
   - parseable (`Lot n) <name>` lists);
   - prose relaunches (`… puis attribué à X`);
   - mixed.
2. **Decide** (a) or (b) per class, and decide which parser owns each era. The text era is `text/parse.rs`; r208 and
   r209 are the legacy readers.
3. **Build** behind the same refold machinery as 510 and 511.
