# 513 — letter-only NHS ODS codes (`GB-NHS-RTH`) mint no identity, so their parties bind by name and never pair

Status: needs-triage — filed 2026-10-11 from issue 469's unit-2 decision (`../469-registry-pairs/unit2-decision.md`, "NHS").
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
