# 513 — letter-only NHS ODS codes (`GB-NHS-RTH`) mint no identity, so their parties bind by name and never pair

Status: ready-for-agent — UNIT 1 DONE 2026-10-11 (`../513-nhs/u1/u1-results.md`). The first unit-2 decision (below) was WITHDRAWN the same day after a three-lens adversarial review (wf_4e7d846d; findings in `../513-nhs/u1/decision-review-wf_4e7d846d.json`, summary under "Review of the first decision"). Its shape rule would mint junk tails (`ICB` on two ICBs, O/0 typos, `00N`/`00P` sub-ICB codes). Its reclaim writes no redirect. And a mint moves only the coded ~10% of each body's mentions. NEXT: a design panel over (A) an ODS-allowlist mint with literal binding, (B) the same with a best-match ODS-name gate, and (C) no mint: verdict-driven consolidation of the SAME clusters with the PPON org under the 469 register machinery. Then the decision, then the build.
Was: ready-for-agent — UNIT 1 DONE 2026-10-11 (`../513-nhs/u1/u1-results.md`): all 129 three-character identity-null codes are Active RC1 ODS organisations; multi-target holders are one body's spellings (238 SAME holders), and strays are 0.96% of coded mentions. UNIT 2 DECIDED (below, "Decision (unit 2)"): exempt the three-character ODS organisation form from the digit and all-zero rules, bind literally, and repair by a refold-invariant mention re-bind, dry first. NEXT: unit 2's build (normaliser arm + tests), then unit 3 (the re-bind job, dry → read → wet), then 469 U7.
Was: ready-for-agent — unit 1 BUILT 2026-10-11 (9e8e123, gate green, two-lens review wf_d682d870-832 found nothing; on main, deploy waits for the 491 drain): the altid dry run stores `registry_nhs_listing` — every GB-NHS ↔ PPON pair (literal, minted identity or null, the unit-1 class, ≤5 register / ≤3 PPON holders with provisional flags, witnesses), dry runs only, no count moves. NEXT: gate + deploy (after the 491 drain), a dry run, then the ODS read of every code (organisation vs site, active, name match, multi-target name variants vs different bodies) and the unit-2 decision.
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

## Decision (unit 2) — 2026-10-11 — WITHDRAWN (see "Review of the first decision" below)

The evidence is `../513-nhs/u1/u1-results.md`. The owner decides as follows.

**D1. Exempt the three-character ODS organisation form.** `normalise_identifier` mints a GB national identity for a
value that is `GBNHS` followed by exactly three alphanumerics, when the digit rule or the all-zero filler rule would
otherwise refuse it. That covers letter-only tails (`GBNHSRTH`) and zero-only tails (`GBNHSR0B`, `GBNHS0AR`). It does
not cover:
- any other length. Typed names (`GBNHSNHSCHESHIREANDMERSEYSIDEICB`) stay refused, and five-character codes, which mix
  sites with organisations, keep today's rule;
- any tail that `idgate::condemns` condemns. The arm still runs under the v2 gate.

The tail must also pass a small placeholder denylist (`TBC TBA TBD NHS NIL NON UNK XXX ZZZ AAA` and same-letter runs)
with its own tests.
- *Why:* ODS answers every one of the 129 measured codes as an Active organisation-level record. The digit rule
  exists to stop placeholder text minting, and here it refuses real organisation codes while it already lets sites and
  sub-units through.
- *Why no lead-letter list:* ODS organisation codes lead with R, Q, T, 0, D and N. A list would be a second register
  to maintain, and the placeholder risk sits in the tail, which the denylist covers.

**D2. Bind literally (exact triple), name-blind, as every digit-bearing GB-NHS code already binds.** No name gate.
- Strays are 0.96% of coded mentions, and almost all are a neighbour's code picked by mistake: names close enough that
  a core-token gate passes most of them, such as north-east vs south-east London ICB, or Leeds vs Bradford Teaching
  Hospitals.
- A strict gate (`altid_keys_agree`) would refuse the SAME variants the data is full of: ICB / Integrated Care Board,
  FT / NHS Foundation Trust, typos. That is issue 518's regrowth again, at worse rates.
- Strays stay identifier-verdict material (452's withheld-identifier path), as for every other scheme.

**D3. Repair by a refold-invariant mention re-bind (unit 3), not by promoting clusters.**
- *What it does:* a dry-first job re-resolves every standing mention whose raw identifier now mints an identity under
  D1 and whose org has no identifier. Such a mention moves to the identity org, which its first mention mints. A
  provisional cluster left empty is reclaimed through the standing reclaim, which stamps ledger rows (old ids redirect
  through the ledger). A cluster that keeps uncoded mentions stays as the body's provisional twin.
- *Why not promote the cluster:* promoting the whole cluster to the code would keep its uncoded mentions with the
  identity. But a rebuild undoes that, because identifier-less mentions bind only `identifier IS NULL` rows (issue 234).
  A promotion would also be the provisional ↔ canonical capture that issue 300 rules out. The twin is the standing
  state of every digit-coded NHS body (u1-results §4); it is issue 300's E3 problem, not this issue's.
- *Expected result:* 131 identity orgs hold the 7,408 coded mentions now spread over 238 SAME clusters. About 36
  clusters empty out. After 469 U7 pairs each identity with its PPON org (53 of 54 `both_distinct` codes are one body),
  each NHS body stands as one canonical org plus its uncoded twin or twins. That is no more orgs than today's k
  variant clusters plus the PPON org, and the canonical row carries both identifiers.
- *Why the job is needed:* `publishes()` compares the raw string, so a refold alone does not re-bind.

**D4. Dry plan, reading and stop rules for unit 3.** The dry plan lists, per code: mentions to move, source orgs,
clusters left empty, and the distinct names moved, with the most frequent first.
- *S1:* stop and revisit D2 if, corpus-wide, more than 3% of moved mentions carry a name sharing no core token with
  the code's most frequent name. The listing measured about 1% of strays, most of them with overlapping names.
- *S2:* stop if a 50-code random sample of the newly minted values has more than 2 codes ODS does not answer as an
  Active organisation.
- *S3:* the wet run holds against the dry plan's stored keys (the 453 rekey pattern).

**Out of scope, filed as 519:** sub-ICB location codes (`03N`, `15M`) and transposed typo codes (`RF1` for `R1F`).
Both already mint identities that split their body.

## Review of the first decision (2026-10-11, wf_4e7d846d — code, data and design lenses; all "sound-with-fixes")

Verified findings that withdraw it:
1. **The shape rule mints junk (blocking).** The 129 codes were only the PPON-paired ones. The same orgs' mentions carry
   20 further tails that would mint, and only RWK and AAV are real (u1-results §5). `GBNHSICB` would fuse two ICBs, and
   the zero-only exemption admits the 18 `00[A-Z]` sub-ICB codes. `idgate::condemns` can never fire on this shape
   (idgate.rs:389), so the denylist would be the only gate.
   - Fix direction: an ODS-derived frozen allowlist of three-character organisation codes (trust RO197, ICB, CSU RO213,
     care trust, measured IS providers), plus an O→0 fold of the tail. Across 634 such codes no O/0 twin pair
     collides. Size it with a dry census of every distinct `GBNHS???` raw value first.
2. **No redirect on reclaim.** The orphan sweep writes `org_sweep_log` and a `removed` change, never `org_merge_log`
   (canonical.rs ~32747). An emptied cluster's id would 404. Retiring a cluster needs a merge-shaped delete with its own
   rule, plus stale-stamping of the affected Tenders, as apply_rehoming does (~24873).
3. **The net effect is weak.** Coded mentions are a median 9.7% of a body's mentions. Orgs go 355 → 448 → 332, and only
   after U7 (13 codes end worse). The canonical row would hold about a tenth of the body.
4. **Strays have no remedy.** 452 verdicts are triple-wide, and a withheld triple still binds by exact triple
   (canonical.rs ~7267, ~15622). `org_mention_rehoming` is not read by the resolver. Name-blind literal binding would
   move about 70 correctly-bound stray mentions into neighbours, with no way back.
   - Measured alternative (design lens): a best-match gate against ODS names refuses a bind when another code's ODS
     name fits the mention's name better. It refuses 63 of 68 stray mentions and 109 of 7,402 SAME ones; 106 of those
     are RWW's 2026-04 rename, which predecessor names would cover.
5. **Stop rules.** S1's core-token test cannot see strays, because NHS words are core tokens: the baseline is 0.19%,
   and 1 of 68 DIFFERENT mentions is flagged. S2 passes Active sub-ICB codes and wrong-body codes. Both ran after the
   normaliser had gone live. Any census gate must come before prevention ships.
6. **U7 hazards.** QKK ↔ QMF (swapped London ICBs), RXX (Sussex Partnership PPON) and RTF (NHFM PPON) would reach U7
   as single-owner cross-body pairs. U7 must hard-deny pairs whose code was multi-target, or whose PPON holder was
   adjudicated non-SAME.
7. **Coverage.** The listing capped registry holders at 5, so 35 of 304 were never adjudicated. They sit in 11 codes
   with a higher stray rate: 12.7% against 5.6%.
8. **Smaller points.** Name the minted org from ODS or from the earliest notice, not "the first mention". Exclude
   mentions placed by rehoming, dissolve or case verdicts (canonical.rs ~10258). A mint yields at most 129 identities.
