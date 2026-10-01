# 469 — FTS NHS, UKPRN, charity and mutual numbers never pair with their PPON: the altid arm keys only Companies House ↔ PPON

Status: ready-for-agent — filed 2026-10-01 from the owner's board survey (workflow wf_4eac8781-4d0, verified by an adversarial pass). The first unit is a dry measurement: for each scheme, count the distinct registry-number ↔ PPON pairs on GB FTS parties and how many of them stand as two organizations.
Kind: data quality (organization identity)
Relates to: 448 (e2-altid, the COH ↔ PPON arm; its design doc promised this follow-up), 460 (a merge drops the
loser's identifier from `identifier=` lookups), 456 (a publisher's identifier on the wrong party), 342 (the FTS
backfill, still adding notices)

## What is wrong

An FTS party publishes a primary `identifier` and, since the Procurement Act, `additionalIdentifiers`. Issue 448
built the arm that reads these pairs (`match-org-identifiers` rule `altid`, `Db::match_org_altid_pairs`,
`crates/store/src/canonical.rs:15307`), but it reads only Companies House ↔ PPON. It counts and drops every other UK
register:

- `crosswalk::altid_pair_key` (`crates/ingest/src/crosswalk.rs:1024`) keys a side only when the published scheme is
  `GB-COH` or `GB-PPON`. Its test asserts `None` for `GB-CHC`, `GB-SC`, `GB-UKPRN` and `GB-NHS` (`:1206`).
- The planner's harvest (`canonical.rs:15424-15434`) adds a row under any other scheme on a GB party to
  `unkeyed_scheme` and `continue`s, so the row never reaches the pair graph.
- The fold alias maps a PPON key to a `GB:coh` key only (`struct AltIdAlias`, `canonical.rs:2975`) and finds the
  owner in `canon_of` (`:10160`). A register with no canonical key has no path through it.
- No other merge arm pairs them either. The GB arm of `canonical_key` keys none of these registers
  (`crosswalk.rs:389-392`, "E0 exact equality only", pinned by `gb_other_registers_and_vats_key_nothing`), so R2,
  which groups on `(country, scheme, key)`, never sees them. E0 merges only equal literals, and an NHS code never
  equals a PPON.

`.scratch/tender-db/448-altid-design.md:244` names this gap: "Scope growth to NHS->PPON (381), UKPRN->PPON (179)
and CHC->PPON (108). These are exact-literal targets with no canonical key, so a different target map is needed.
They are a separate follow-up." It was never filed. 448's Status lines name 452 and 456 as follow-ups and do not
mention it. 448's own table (line 27, 14,647 FTS notices before the backfill) also counts MPR→PPON 71 and COH→CHC 58.

**Exhibits.** Read on the public API on 2026-10-01 between 12:1x and 12:2x UTC. Each notice publishes both
identifiers on ONE party, and each identifier keys its own organization:

| notice | party | ordinal 0 | ordinal 1 | org of ordinal 0 | org of ordinal 1 |
|---|---|---|---|---|---|
| 46743853 (FTS `054847-2025`, 2025-09-09, buyer of tender 8639643) | Leeds Teaching Hospitals NHS Trust | `GB-NHS-RR8` | `GB-PPON-PNQG-8433-QXBP` | **31540413**, `GBNHSRR8`, 4 mentions | **30913623**, `GBPPONPNQG8433QXBP`, 398 mentions |
| 46794997 (FTS `019846-2026`, 2026-03-05, buyer of tender 8642397) | Bradford College | `GB-UKPRN-10000840` | `GB-PPON-PRQR-6323-DRZQ` | **30928147**, `GBUKPRN10000840`, 11 mentions | **30969231**, `GBPPONPRQR6323DRZQ`, 12 mentions |

So `?buyer=30913623` omits Leeds' three NHS-keyed tenders (8639643, 8639862, 8691432), and Bradford College's
history is split roughly in half.

**Size: not known.** The live counter cannot answer the question. Dry job 1799 (2026-10-01 11:35:33 UTC, after
chunk 8's project 1798, 293,063 FTS notices) reads:

    unkeyed_scheme {"GB-CHC": 3463, "GB-MPR": 1434, "GB-NHS": 16459, "GB-NIC": 287, "GB-SC": 216, "GB-UKPRN": 3476, "GG-RCE": 3, "IM-CR": 3, "JE-FSC": 1}

Those are `BT-501` ROWS on GB parties, at any ordinal and once per notice. A buyer on 1,000 notices counts 1,000
times, and a party that publishes its NHS code with no PPON beside it counts too. The figures say the registers are
common, 24,832 rows for NHS, UKPRN, CHC and MPR together, far above the "34 rows" premise in the GB arm's comment at
`crosswalk.rs:392`. They do not say how many pairs exist, or how many pairs stand as two orgs.

**The exact-literal target is not clean.** The design doc's "exact-literal targets" will meet publisher spellings
of one number. Read the same day:
- University of Manchester stands as org 31600847 (`GBUKPRN10007798`, 48 mentions) and as org 31612560
  (`GBUKPRNUKPRN10007798`, 18 mentions). Notice 46919193 publishes `GB-UKPRN-UKPRN 10007798`.
- Barnardo's charity number 216250 is held by org 31561821 (`GBCHC216250`) and org 31594691
  (`GBCHCCHARITYNUMBER216250`).

## Proposed fix

**Unit 1: measure (dry, writes nothing).** Extend the altid harvest in place. At `canonical.rs:15426`, keep each
unkeyed-scheme row of a GB party as `(scheme, E0 identity)`, where the E0 identity is the value that
`project::normalise_identifier` gives under the party's country (the value the resolver minted the org with). Pair
it with every E1 PPON on the same party, in either order. Collect the identities the harvest saw, and resolve them
to owners in the existing org walk by exact `identifier`. The PPON side keeps its existing `GB:ppon` owner map.
Report each of the following by scheme in `altid-merge-plan`:
- `literal_pairs`, `already_one`, `both_distinct`, `no_target_registry`, `no_target_ppon` and `multi_target`;
- PPONs beside two values of one scheme, and PPONs beside a COH and another register on one party (the charitable
  company shape);
- a 30-pair `both_distinct` sample per scheme with both orgs' names, in R2's listing shape.

`plan_pairs` and every existing counter must read the same as before the change. Pin it with a store test, e.g.
`an_nhs_ppon_pair_is_counted_and_never_planned`: one FTS notice whose GB party carries `GB-NHS-RR8` and a PPON, and
two orgs. Expect `both_distinct` 1 under `GB-NHS` and `plan_pairs` 0.

**Decide after the measurement, with the numbers in hand:**
1. Which schemes merge. NHS, UKPRN, CHC and MPR are the owner's scope. SC and NIC are the same shape, and GG/IM/JE
   are not GB registers.
2. The survivor. The rule should match the COH arm: the register's org keeps, and the PPON org (a platform
   registration) folds into it.
3. A party with COH, CHC and PPON. The COH arm already pairs its COH with its PPON. Decide whether the charity org
   then folds into the same survivor, or whether the triple is a conflict.
4. Spellings. Should the registry identity strip a repeated scheme token or a label (`GBUKPRNUKPRN…`, `GBCHCCHARITYNUMBER…`), or is
   that a separate within-register merge?

**The root-cause fix (after the decisions).** Generalise the pair graph from `GB:coh ↔ GB:ppon` to `registry ↔
GB:ppon`. A COH side keys canonically, as it does today. Every other register keys by its scheme-qualified E0
identity and never passes through `canonical_key_flat`. The hole that `altid_pair_key`'s scheme check closes, an
8-digit charity number arriving as a company number, stays closed, so `gb_other_registers_and_vats_key_nothing`
and the `None` assertions at `crosswalk.rs:1206-1209` stay as they are. The pairs take the same structural and
judgment gates, scheme-qualified verdict keys under `GB:altid` (existing COH keys unchanged), and `e2-altid` ledger
rows naming the registry scheme. The alias (`arm_altid_alias`) must resolve a non-COH target through the
resolver's E0 lookup, not through `canon_of`. Otherwise, the next PPON-first notice after a merge mints the PPON
org again. Pin it with a store test, e.g. `an_nhs_ppon_pair_merges_into_the_nhs_org_and_the_alias_binds_the_next_ppon_mention`.

## Verify

    curl -s -w '%{http_code} ' -o /dev/null https://tenders.zebreus.click/v1/organizations/31540413 -o /dev/null https://tenders.zebreus.click/v1/organizations/30913623 -o /dev/null https://tenders.zebreus.click/v1/organizations/30928147 -o /dev/null https://tenders.zebreus.click/v1/organizations/30969231

This checks the last unit, the merge. Unit 1 is read from its new counters in `/admin/reports/altid-merge-plan`.

- **open** (2026-10-01 12:23 UTC): `200 200 200 200`. Leeds Teaching Hospitals (NHS RR8 / PPON) and Bradford
  College (UKPRN / PPON) each stand as two orgs.
- **done**: `200 308 200 308`. The PPON orgs 30913623 and 30969231 redirect (issue 455) into the NHS-keyed and
  UKPRN-keyed orgs. Under a different survivor rule (decision 2), the 308 moves to the other org of the pair.
  Either way, each pair answers exactly one 308.
