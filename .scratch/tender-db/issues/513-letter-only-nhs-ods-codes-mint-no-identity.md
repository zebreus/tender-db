# 513 — letter-only NHS ODS codes (`GB-NHS-RTH`) mint no identity, so their parties bind by name and never pair

Status: ready-for-agent — unit 1 BUILT 2026-10-11 (9e8e123, gate green, two-lens review wf_d682d870-832 found nothing; on main, deploy waits for the 491 drain): the altid dry run stores `registry_nhs_listing` — every GB-NHS ↔ PPON pair (literal, minted identity or null, the unit-1 class, ≤5 register / ≤3 PPON holders with provisional flags, witnesses), dry runs only, no count moves. NEXT: gate + deploy (after the 491 drain), a dry run, then the ODS read of every code (organisation vs site, active, name match, multi-target name variants vs different bodies) and the unit-2 decision.
Was: ready-for-agent — triaged 2026-10-11 with issue 469 U2's evidence. NEXT: unit 1, measure (an in-app read of the
letter-only GB-NHS values and the 73 multi-target codes; ODS lookups from the agent container, as U2's fetcher did).
Was: needs-triage — filed 2026-10-11 from issue 469's unit-2 decision (`../469-registry-pairs/unit2-decision.md`, "NHS").
Kind: data quality (organization identity)
Relates to: 469 (registry ↔ PPON pairs; NHS waits on this), 345 (re-key repair precedent), 300 (identity ladder: a
provisional ↔ canonical capture is E3, never merges)

## What is wrong

`project::normalise_identifier` mints an identity only for a value with a digit (project.rs, the digit rule), so a
letter-only NHS ODS code (`GBNHSQWO`, `GBNHSRTH`) mints none: its party binds a provisional org by name, the 469
altid arm can never pair it with the body's PPON, and different spellings of one trust's name stand as different
orgs. Measured by 469 unit 1 (job 2163): of 195 NHS ↔ PPON pairs, 131 mint no identity and 73 are multi-target (one
code name-bound to several provisional orgs).

## Proposed units

1. **Measure.** The distinct letter-only GB-NHS values; placeholders (`TBC`, `NA`); the 73 multi-target codes —
   name variants of one body, or different bodies under one code — each checked against the ODS ORD API
   (`directory.spineservices.nhs.uk/ORD/2-0-0/organisations/<code>`, reachable from the agent container).
2. **Decide.** An exemption to the digit rule scoped to `GBNHS` + an ODS shape, still under `idgate::condemns`; or
   not, if the 73 are different bodies.
3. **Ship with a re-key repair** (the 345 pattern): `publishes()` compares only the raw string, so standing
   mentions would not re-bind on a refold alone. Collapsing the multi-target orgs is itself a merge decision with its
   own gates.

## 469 U2 evidence (2026-10-11)

- In U2's 30-pair NHS sample, 14 codes mint no identity. Seven are letter-only ICB codes (`QGH`, `QMM`, `QOC`, `QPM`,
  `QUA`, `QVV`, `QWU`). The other seven are trust codes (`RBD`, `RBT`, `RBV`, `RCB`, `RCD`, `RCF` and `R0B`) whose
  register org carries NO identifier. `R0B` has a digit, so the digit rule alone does not explain it. Unit 1 must
  find out why `GB-NHS-R0B` (South Tyneside and Sunderland NHS FT, org 13569138) minted nothing. ANSWERED
  2026-10-11 (unit 1's build): the all-zero filler rule (project.rs `normalise_identifier`, ~8715) refuses a value whose
  only digit is `0`, so `GBNHSR0B` mints nothing and its party binds by name — the same digit-rule family as the
  letter-only codes, so unit 2's exemption must cover it too. Possibly the org
  predates the code's first mention, or the mention bound by name.
- ODS (fetcher `.scratch/tender-db/469-registry-pairs/u2/registers/ods.json`) answers all of them as Active
  organisation-level codes (ICB = `STRATEGIC PARTNERSHIP`, trust = `NHS TRUST`). So letter-only codes are org-level
  identities in ODS, which argues for the digit-rule exemption in unit 2.
- Shape cannot tell organisations from sites. 5-character codes include organisations (`8HN02`, `A0V4D`, `G6V2S`,
  `H6A5H`) and sites (`NCS04`, an NHS Property Services site). A letter-only exemption should therefore be
  ODS-confirmed, or limited to the 3-character trust and ICB forms (`R??`, `Q??`). The 3-character `\d\d[A-Z]` form
  (`06H`) is a sub-ICB location, not an organisation.
