# DE-1.x ledger wording — DRAFT for review (sdk-vendor, 2026-08-02)

**APPLIED 2026-08-02** to `crates/app/data/quarantine-ledger.json` on branch `issue98-de1x-org-refs`,
approved by team-lead with the residual-241 sentence added.

**Correction to my original sequencing note.** I first wrote "apply only once the re-verify is green",
which cannot work: the ledger is `include_str!`-compiled and the verification's section I checks the
*served* disclosure, so the edit must already be in the deployed binary for that gate to pass. The real
order is: ledger edit rides the deploy build -> deploy (app serves it on 127.0.0.1:8080, nginx still
down) -> scoped re-fold -> re-verify (fact gates prove the "resolved" claim true; section I proves the
disclosure is served) -> nginx up only on all-green. The public never sees the claim before the gates
prove it, which is what I was reaching for.

## What changes and why

The current entry claims the category is resolved because *"the recovery rebuild (ADR-0009) folds them
in"*. That was false when written (issue 85 recorded it) and is still not the whole truth: after 98+99
the facts and the organization roles land, but **award winners do not** (issue 100). The ledger is the
user-facing "Resolved categories" table, so this is the honesty gate — the `diagnosis` field is the
narrative column the dashboard renders (`ui.rs:623`), so the disclosure belongs there, not in a comment.

**Counts are deliberately not written into the prose.** The dashboard joins each entry against
`Db::quarantine_resolution` live, so resolved/held render themselves. Hard-coding them would go stale.

## The entry as applied (verbatim from the committed file)

```json
{
  "category": "eForms-DE 1.x (German dialect)",
  "reason": "unknown-customization",
  "detail_like": "%eforms-de-1.%",
  "diagnosis": "German eForms-DE 1.0/1.1/1.2 notices were held as unknown-customization: the national 1.x line is spec-and-schematron only, with no published field metadata to parse against. The inventory was rebuilt empirically from the archived corpus (issue 75), with grafts for the DÖE serializer's inlined parties (issue 78), then the cohort was reclaimed and projected. These tenders now carry their German title and description, CPV, NUTS, lots, deadlines and estimated values, the buyer, and the other named organization roles — review body, information and document providers, tender recipient, evaluator, mediator. AWARD WINNERS ARE NOT RESOLVED for this dialect (issue 100): the result graph references sections by their published ids while those sections are keyed synthetically, so the LotResult → LotTender → TenderingParty chain does not link. Award notices therefore show their lots, contracts and awarded values, and bidders where a notice names them, but not which bidder won. That is a parse-layer defect: fixing it requires re-parsing the cohort, so it is deliberately not part of this batch. A residual ~241 notices remain held pending re-parse and still carry their original unknown-customization reason, which is now stale; they are tracked separately (issue 87).",
  "fix": "issues 75/78/76/85/98/99 — award winners pending, issue 100",
  "resolved": "2026-08-02"
}
```

## Wording decisions, so review is quick

- **"AWARD WINNERS ARE NOT RESOLVED" in caps, early, in its own sentence.** A reader skimming the
  Resolved-categories table must not come away thinking awards are complete. It is the one claim we would
  be most embarrassed to have implied.
- **Names the mechanism** (published ids vs synthetic section keys) rather than saying "a known issue".
  Specific enough that a user can judge how much it affects their question.
- **States what awards DO carry** (lots, contracts, awarded values, named bidders) so the gap is bounded
  rather than sounding like "awards are broken".
- **"deliberately not part of this batch"** — says the omission was a decision, not an oversight.
- **`resolved` stays set** once the fold lands. The *quarantine category* genuinely is resolved: the
  notices parse and project. Winners are a downstream completeness gap, disclosed in the narrative, not a
  quarantine that is still held. Blanking `resolved` would misreport the parse status; the disclosure is
  what keeps it honest.
- **`fix` carries the pending pointer too**, because that column is what an auditor follows.

## Open for team-lead

1. **Resolved: `resolved` stays set.** The quarantine category genuinely is resolved; winners are a
   downstream completeness gap disclosed in the narrative. Date set to `2026-08-02`, the expected fold
   date — **bump it if the fold slips past midnight UTC**, since it is a dated claim.
2. **Resolved: add the 241.** Added as the final sentence, after the winners disclosure so prominence is
   preserved: an unexplained "241 held" on a Resolved row is its own small dishonesty.
3. **Resolved: file a follow-up** to also surface the gap in the data-quality panel — **issue 101**,
   additive, not blocking this ship.
