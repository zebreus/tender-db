# 514 — spellings of one register number (label text, linked-charity suffixes) split one registry identity

Status: ready-for-agent — UNIT 2 SHIPPED AND ROLLED OUT 2026-10-12 (fd509b6, deployed 31f0872; `../514-register-spellings/rollout/results.md`). 345 wet 2313 re-keyed 53 identities (L14 held), recording pre-images. E0 2315 + 2316 merged all 26 reviewed groups, bound to the listing: 26 org rows removed, 0 held. W6's footprint matched exactly the 26. Repeat dries plan 0, and altid's register pairs are unchanged. NEXT: check the next fold's 448 'register targets on two or more rows' = 0. Then unit 3 (the Companies House mirror, RS→bare, IP0, with alias canonicalisation first) and the deferred classes: CHC compounds, generic labels, the 453 near-miss re-keys, XI.
Was: ready-for-agent — UNIT 2 REVIEWED 2026-10-12 (`../514-register-spellings/u2-review/review.md`; wf_d1abdb0b-672: packet, 6 reviewers, 6 challengers). All 26 groups are merge, 26 lone movers are move-ok, and 1 is held: L14, Birmingham CAB on the national Citizens Advice number 279057, now on the hold list. The challenger agreed on all 53. Registers: all 53 numbers found. P0 is complete: S1 holds, B0 drained, no standing keep on the keys. NEXT: build U2.2/U2.3 per §7 after 511 unit 5 leaves project.rs: the normaliser rewrite, the 345 binding, E0's W6 and listing binding, and the hold list. Post the 26 verdicts in U2.3's keys before the gate (S8), then the §5.1 sequence.
Was: ready-for-agent — UNIT 2 DECIDED + P0 DONE 2026-10-12 (`../514-register-spellings/unit2-decision.md`, `p0/p0.md`). 345 dry 2288: rows 0 (S1). E0 dry 2289: B0 = 3 exact-duplicate groups, drained by E0 wet 2290 (5 org rows), so B0 is now 0. NEXT: measure first #3 (register records for the 54 canonical numbers) and #4 (the review packet, 52 members + 27 movers). Then post the 26 verdicts and build U2.2/U2.3 per §7.
Was: ready-for-agent — UNIT 2 DECIDED 2026-10-12 (`../514-register-spellings/unit2-decision.md`; design panel wf_9e0f1ab7-2e7, 3 designs, 2 judges, synthesis; all 12 owner rulings adopted). MINIMAL is adopted: a live-normaliser rewrite of only the spellings that 469's REGISTRIES shape refuses, into shaped values.
- Rewrite rules: UKPRN repeated token; CHC with 9 closed charity labels; NIC with 3 labels (spelling kept); MPR `IP`+5 digits+`R` and `REGNO`+R-number. The token, digits and R are never changed.
- Repair: the 345 re-key, bound to its plan.
- Merge: R2's E0, with a reviewed verdict on every group as the name gate.
- Reach: 26 of 36 splits, 743 of 852 mentions, no trap literal touched.
- Deferred: RS00NNNN → bare and IP0 (unit 3, with the Companies House mirror), generic CHC labels and compounds.
NEXT: 'Measure first' (the E0 wall test; P0 on prod; register records for the 54 numbers; the review packet), then the build units in §7.
Was: ready-for-agent — UNIT 1 MEASURED 2026-10-11 (`../514-register-spellings/`: census plus skeptic, wf_0e6a29e9-c2e; 29 + 15 bounded indexed reads, 0 × 408). There are 36 in-scheme splits (73 orgs, 852 mentions): MPR 15, CHC 15, UKPRN 5, NIC 1, SC 0. 34 are name-confirmed and 1 is partial (1179472). The cross-prefix mass is much larger, mostly Companies House mirrors of FCA numbers (`GBCOHIP…`/`GBCOHRS…`): about 3.2k more mentions for MPR. NEXT: the unit-2 decision, i.e. a per-scheme canonical form, the name gate reading ALL of an org's names (not the head name), the cross-scheme Companies House mirror question, and a 345-style re-key. Design panel first.
Was: ready-for-agent — triaged 2026-10-11 with issue 469 U2/U3's evidence. NEXT: unit 1, measure the standing
spelling pairs per scheme (two orgs holding two spellings of one register number).
Was: needs-triage — filed 2026-10-11 from issue 469's unit-2 decision (decision 4: spelling repair stays out of the 469 arm).
Kind: data quality (organization identity)
Relates to: 469, 470 (the GB O/0 company-number lookalike repair, the precedent)

## What is wrong

FTS register literals carry label pollution (`GBUKPRNUKPRN10007798`, `GBCHCCHARITYNUMBER216250`) and glued
linked-charity suffixes (`216250-1`), so one register number can mint several identities. 469's arm refuses them
(its shape gate) rather than repairing them.

## Proposed unit

Measure the spellings per scheme on the 469 dry plan's literal shapes (U0), then decide a per-scheme canonical form
behind the name gate, as 470 did for GB company numbers.

## 469 U2/U3 evidence (2026-10-11)

The spellings are real. U3's shape gate keeps them apart, never folding them:
- **MPR.** The FCA's own number for a 2014-Act society is bare (`7639`, Ongo Homes). Companies House mirrors it as
  `RS007639`, and FTS publishers use both (`GB-MPR-RS007639` in the U2 sample). Two identities, one society.
  - Bare and `R`-suffixed are NOT spellings of each other: `7522` is Thirteen Housing Group, `7522R` an allotment
    society (FCA list). A canonical form may map `RS0NNNNN` to bare `NNNN`, and nothing else.
  - `IP16243R` is Companies House's spelling of FCA `16243R`. The sample's `ip6243r` is a typo of it and stays
    refused.
- **NIC.** `NIC 100095`, `NI 100770` and a bare `100095` are three identities for CCNI numbers (U3 accepts all
  three as published).
- **UKPRN / CHC.** Label pollution (`UKPRN10007798`, `CHARITYNUMBER216250`) and glued linked suffixes (`-1`) are
  refused, never stripped.

A unit-2 canonical form must ship with a 345-style re-key repair (`publishes()` compares raw strings), behind the
name gate as 470 did.

## Unit 1 — measured (2026-10-11)

Census and verification: `../514-register-spellings/` (`splits.json`, `unit1-census.md`, `unit1-verify.md`, and the
workflow output `workflow-wf_0e6a29e9.json`).

**In-scheme splits** (both stored identifiers carry the scheme's own token), as corrected by the skeptic:

| scheme | identities scanned | splits | orgs | mentions | classes |
|---|---:|---:|---:|---:|---|
| GB-MPR | 135 | 15 | 30 | 492 | 10 × `NNNNNR` vs Companies House `IPNNNNNR` under GB-MPR; 4 × bare 4-digit vs `RS00NNNN`; 1 label (`Reg No 30938R`) |
| GB-CHC | 748 | 15 | 31 | 63 | bare vs label 9, label vs label 2, bare vs charity+company compound 3, leading-zero `0518801` 1 |
| GB-UKPRN | 522 | 5 | 10 | 295 | all label pollution (`UKPRN: 10005790`) |
| GB-NIC | 19 | 1 | 2 | 2 | label (`Charity Commission No NIC102575`) |
| GB-SC | 11 | 0 | 0 | 0 | — |

**What these findings mean for unit 2.**
- **The name gate must read every name an org carries.** UKPRN 10007157: the bare org's head name reads "Sheffield
  Hallam University", but 15 of its 16 mentions are University of Sheffield, which is 10007157. A head-name gate would
  wrongly refuse this true pair. This is issue 518's lesson again.
- **Cross-prefix mass.**
  - MPR: 76 groups beyond the in-scheme splits, 3,238 more mentions, mostly the Companies House mirror (`GBCOHIP…`,
    `GBCOHRS…`).
  - 3 MPR groups are a subsidiary published on its parent's number (Guinness 31693R, Homes Plus 30224R, IDS Trustees
    14044R). These must not merge.
  - A `REG…` range adds 9 MPR and 4 CHC groups; bare `SC0…` adds 3 SC groups. These are lower bounds.
- **Traps the canonical form must refuse.**
  - Typo spellings of the wrong number:
    - `GB-MPR-ip6243r` (Hastoe is 16243R);
    - `IPI6637R` (Metropolitan is 16337R);
    - `RSA007814` (Beyond Housing, 52 mentions);
    - UKPRNs with a dropped or doubled zero (5).
  - HMRC charity references published under GB-CHC: `X1089`, `XC1089`.
  - Linked charities: only 2 glued `-N` literals exist (`1099485-2`, `1169762-2`). The stored value cannot tell them
    apart, so unit 2 must read the raw literal.
- **Merges so far.** No merge has folded any of these pairs yet; no twin spelling appears in the other's mention
  literals.
- **Read limit.** `organization_merged_identifiers` is not on `/v1/sql`'s allow-list.

