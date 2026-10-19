# 574 skeptic 3: row 22, org 17915448

**Result: the W stands.** I tried to argue the change up to R or N and could not. The only route to N depends on a
reading of the identifier that the evidence makes the less likely one. Confidence: high that it is not R, and
medium-high that it is W rather than N.

| org | change | V/T/H | head / top / rival newest | v12 hold, reason | judge | skeptic |
|---|---|---|---|---|---|---|
| 17915448 | `ECO CENTRO SARDEGNA SRL` -> `RTI COSTITUENDO MASNATA CHIMICI S.P.A. + CHIMICA SARDA S.R.L. (DATI MANDATARIA)` | 11/8/1 | 2021-10-12 / 2022-02-28 / 2022-02-28 | family, top_stale | W | **W** (N only if the P.IVA is Masnata Chimici's) |

## What I read

- **Plan entry** (`rows.json`):
  - The identifier is P.IVA 01301570915 (IT, national).
  - There are 3 clusters:
    - the top: the RTI sentence ending "(DATI MANDATARIA)", 8 votes, newest 2022-02-28;
    - the rival: the same sentence ending "(mandataria)", 2 votes, newest 2022-02-28;
    - the head: `ECO CENTRO SARDEGNA SRL`, 1 vote and 1 procedure, 2021-10-12.
  - There are no undated mentions, and the org is not provisional or merged.
- **Saved body** `bodies/org_17915448.json`, fetched today at 12:56:49Z. It still serves `ECO CENTRO SARDEGNA SRL`,
  with 11 mentions and no merged identifiers. Both match the plan.
- **New GETs: 0.** None of the three allowed endpoints lists an org's tenders, and no tender or notice id for this
  org exists anywhere under `.scratch/`. I grepped the repo for the org id, the P.IVA, "Masnata" and "Eco Centro
  Sardegna". An org GET would only repeat the saved body, so no GET could reach the 2021 or 2022 procedure.

## The case for R: impossible

A "RTI costituendo" (raggruppamento temporaneo di imprese, still being formed) has no legal personality and no P.IVA
of its own. So 01301570915 belongs to one company, and the org is that company.

The new name names two companies and adds the annotation "(DATI MANDATARIA)", meaning "lead firm's details". That is
not the name of any one company. So whoever the holder is, R is ruled out.

## The case for N: the only defensible one, and weaker than W

N needs the old name to be wrong too. Then the change would be wrong for wrong, which 548 labels N: "organ for organ",
"both carry addresses". That happens only if the P.IVA belongs to **Masnata Chimici S.p.A.**, the mandataria. In that
case `ECO CENTRO SARDEGNA SRL` is a stray print of another firm, and the new name at least contains the holder.

In favour of that reading, the "(DATI MANDATARIA)" tail literally says the record carries the mandataria's details,
and it sits on 10 of the 11 votes.

Against it:

1. **By procedure, the votes are 1 to 1, not 10 to 1.** All 10 RTI votes share one date, 2022-02-28. That is one
   procedure, most likely one multi-lot award, and one buyer typing one record. The 2021 `ECO CENTRO SARDEGNA SRL`
   print comes from a separate procedure. This is the votes-per-notice shape behind 548's CPR and Barameda wrongs.
2. **The office code points away from Masnata.** The P.IVA's office code is 091 (Nuoro). As far as I know, Masnata
   Chimici is a Cagliari-area firm, which would carry code 092. I could not confirm this from here, so I give it
   weight but do not rest the verdict on it.
3. **A single Nuoro firm explains both prints.** Suppose the holder is Eco Centro Sardegna S.r.l. or Chimica Sarda
   S.r.l., two Nuoro-area SRLs.
   - If the two are one company under an old and a new name, the right new name is `CHIMICA SARDA S.R.L.`, not a
     sentence that puts Masnata first.
   - If they are unrelated, the change moves the name to another entity's grouping.
   - Either way it is W.
4. **Even in the Masnata case the new name is a group for a member, with an annotation.** 548 labelled these W:
   - 9168556, which adds a role prefix to the holder's own name ("Partner konsorcjum ...");
   - 3728305 and 10037554, whose new names carry annotation tails;
   - an RTI sentence that "was rightly not elected" (548 U3 `spot2654_3B.md`, 429451).

   Masnata is the one case where the old name is also wrong. That makes it N there, not R.

It is not REVERT either. The new name is the newer print (2022-02-28, after 2021-10-12).

## Shipping and guard notes

- **When it can ship.** Row 22 is held `top_stale` (3 clusters, top last printed 2022-02-28), so time alone never
  releases it. It ships only if the RTI spelling is printed again, which keeps the exposure small. The `family` H ≤ 1
  release is what opened it. 548 U1 read that class 22/22 R, so this is the class's first recorded wrong.
- **No guard catches it.** No v13 guard catches a consortium marker *added* in the new name. Even the 548 U1b probe
  lexicon (`548-triage/u1b/role_added.py`) has `consorzi\w*` but no Italian RTI marker: `rti`, `ati`, `costituend\w*`,
  `raggruppament\w*`, `mandatari\w*`, `mandant\w*`. So that probe would have missed this row too.
- **Only row of its kind.** Among the 81 rows in `rows.json`, it is the only one whose new name adds such a marker.

## Method

I read the saved body, the plan entry, `read_3.md` row 22, and the 548 U1, U1b and U3 precedents. I made 0 new GETs,
no /v1/sql request, no POST and no ssh, used no git, and wrote only this file.
