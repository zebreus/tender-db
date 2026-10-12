# 513 — letter-only NHS ODS codes (`GB-NHS-RTH`) mint no identity, so their parties bind by name and never pair

Status: ready-for-agent — U6 SHIPPED AND DRAINED 2026-10-12 (6593d9c, deployed c786f0e; `../513-nhs/u5/results.md`). 171 of 174 codes are applied: 156 from U3, 6 from the residue cohort and 9 from the U6 cohort (wet 2286, all PPON survivors). 273 org rows removed, 26,705 mentions repointed, ledger 273, 0 regrown. Held, by design: DQF (already one org), RFF (no SAME holder) and RVY (after its legal end; 519). NEXT: the S4 read after the daily folds ([issue 513] alias diag lines: refusals ≤ binds; a dry run with `nhs_body_regrown` []). Then 469 U7 with 519.
Was: ready-for-agent — U5 APPLIED + RESIDUE READ 2026-10-12 (`../513-nhs/u5/results.md`). 162 bodies are applied: 156 from U3, plus 6 from the residue cohort `nhs-body-residue-2026-10-12` (QJM QKK QMF QMJ QOP QYG; wet 2281). 260 org rows removed, 25,347 mentions repointed, 0 regrown. 12 are held. 9 of them on `member-key-owned-elsewhere`, and 7 of those block on the PPON SURVIVOR, whose head key the alias never arms. NEXT: U6, the survivor-exempt D4 (see below), gated with 511 unit 4. Then re-census and post RHM / RNK / RXT / RYC / RYG / RRV / RCB per the read. Also the S4 read after the daily folds; then 469 U7 with 519.
Was: ready-for-agent — U5 APPLIED 2026-10-12 (`../513-nhs/u5/results.md`). Capped wet 2275 folded 20 bodies; the S3 read was clean; canary 2277 AGREES (29 of 29); full wet 2278 folded 136. All 156 bodies are applied: 119 merged, 37 one-member; 237 org rows removed; 23,395 mentions repointed; ledger 237 rows; 0 regrown. The 7 non-members that 'left' a code were strays folded into their own body (verified, 308). NEXT: S4 read after the daily folds ([issue 513] diag lines; a dry run with `nhs_body_regrown` []), the owner read of the 18-code residue, then 469 U7 with 519.
Was: ready-for-agent — U3 DONE 2026-10-11/12. Every holder of the 174 codes was adjudicated by a reviewer and a challenger per batch against ODS (wf_126dc78d-0f3, 38 agents; `../513-nhs/u3/adjudication-rows-u3.json`). 156 bodies have a HIGH merge verdict both agree on: 393 members, 37 of them one-member. They are POSTED as cohort `nhs-body-ods-confirmed-2026-10-11`, with `reviewed_holders` taken from census 2261. Re-census 2264: 156 admitted, 0 not admitted. Not posted: 18 codes held or disputed (DQF QJM QKK QMF QMJ QOP QUY QYG RCB RFF RHM RNK RRV RVY RXF RXT RYC RYG; see `u3-residue` below). NEXT: U4, the wet path, the alias armed from the ledger, the scoped unwind, and the merged/regrown reports, in one deploy. Then U5: dry run, a wet run capped at 20 bodies, a read, then the full wet run.
Was: ready-for-agent — U2 DEPLOYED 2026-10-11 21:54 UTC (a163c1e) and RUN. Dry census job 2261 (`../513-nhs/u3/nhs-body-plan-2261.json`): 335,827 FTS notices, 13,187 coded GB parties (9,234 on the allowlist, 7 by an O/0 fold); 174 codes, 534 holders, uncapped (12 orgs hold two or more codes); 254 tails off the allowlist; denials: 1 minted identity, 1 generic, 1 consortium, 2 successor names; 13 keys owned outside their body. The coded share by notice year is 0 before 2022, then 34 (2022), 112 (2023), 20% (2024), 34% (2025) and 36% (2026). NEXT: U3, adjudicating every holder of the 174 codes (a reviewer plus a challenger per body, against ODS), then posting HIGH verdicts with `reviewed_holders` from this live listing.
Was: ready-for-agent — U2 ON MAIN 2026-10-11 (084e4d0, gate green): the dry `nhs-body` census planner and the `reviewed_holders` verdict schema. Built by agent (wf_246fba7e-fdd), reviewed for correctness and test strength, 11 review fixes applied. NEXT: deploy once the 491 drain ends, then a dry `match-org-identifiers {"rule":"nhs-body"}` and a read of `nhs-body-plan` (uncapped holders, denials, off-allowlist tails, coded share by year). Then U3: adjudicate every holder (a reviewer plus a challenger per body, against ODS) and post the verdicts with `reviewed_holders` built from the live listing.
Was: ready-for-agent — U1 ON MAIN 2026-10-11 (gate green at 03e7816, pushed as 493457c; nothing calls it, so no deploy is needed). U2 (dry `nhs-body` planner, census and `reviewed_holders` verdict schema) is BUILDING (wf_246fba7e-fdd: build, then correctness and test-strength reviews). NEXT: review fixes, the gate, then deploy after the 491 drain ends, then the dry run and U3 (adjudicate the uncapped holders).
Was: ready-for-agent — U1 BUILT 2026-10-11, commit on claude/cool-sagan-5bk0rc, gating. `crates/ingest/src/nhs_ods.rs` + `sdk/nhs-ods.tsv`: 1,062 rows, 215 allowlisted, with predecessors, successors and legal ends. Built by agent (wf_ce5452c7-5ce), reviewed for fidelity and for test strength (mutation runs), review fixes applied. Over the 389 adjudicated holders the check admits 233/235 SAME registry and 116/116 SAME PPON holders, and 0 DIFFERENT/NOT_ORG. NEXT: the gate, then push to main (no deploy needed; nothing calls it), then U2 (the dry census, after the 491 drain).
Was: ready-for-agent — UNIT 2 DECIDED (second decision) 2026-10-11, after the design panel wf_75976f73-b4a (designs A allowlist mint / B ODS-name-gated mint / C reviewed consolidation; correctness, product and engineering judges all ranked C first, scoring 7 against 5/6, 3/4 and 4/6). C mints NO identity. It folds each body's SAME clusters and SAME PPON org into one survivor under a reviewed, ODS-confirmed, members-exact `nhs-body` verdict, ledgered and redirecting, with an alias on the members' own 234 keys. Panel files: `../513-nhs/u2-panel/`, `../513-nhs/u1/design-panel-wf_75976f73.json`. NEXT: U0 (the 300 §10 amendment and the u1 corrections), then U1 (the frozen ODS table and check; pure, no behaviour change).
Was: ready-for-agent — UNIT 1 DONE 2026-10-11 (`../513-nhs/u1/u1-results.md`). The first unit-2 decision (below) was WITHDRAWN the same day after a three-lens adversarial review (wf_4e7d846d; findings in `../513-nhs/u1/decision-review-wf_4e7d846d.json`, summary under "Review of the first decision"). Its shape rule would mint junk tails (`ICB` on two ICBs, O/0 typos, `00N`/`00P` sub-ICB codes). Its reclaim writes no redirect. And a mint moves only the coded ~10% of each body's mentions. NEXT: a design panel over (A) an ODS-allowlist mint with literal binding, (B) the same with a best-match ODS-name gate, and (C) no mint: verdict-driven consolidation of the SAME clusters with the PPON org under the 469 register machinery. Then the decision, then the build.
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

## Decision (unit 2, second) — 2026-10-11 — ADOPTED by the owner

Source: the synthesis of design panel wf_75976f73-b4a, adopted with the owner notes at the end. The full designs,
judge scores and grafts are in `../513-nhs/u1/design-panel-wf_75976f73.json`, and the scripts in `../513-nhs/u2-panel/`.

Replaces the withdrawn D1–D4. All three judges ranked design C first. Counts are over distinct orgs (352 SAME orgs, 129 bodies).

**D1. No mint. Consolidate each body by review (C).** A HIGH, members-exact verdict per allowlisted code folds the body's SAME clusters and SAME PPON org into one survivor. The survivor is the PPON org (for RYA, the real code's PPON), or else the largest SAME cluster as a provisional anchor. Each loser gets `org_merge_log` `nhs-body` (308), and the survivor records `GBNHS<code>` as a merged identifier. `normalise_identifier` does not change.
*Why:* orgs go 352 → 173 now → 129, with 0 bodies worse, 0 strays moved, 0 identities minted; a mint holds ~9.5% of a body. The code never binds, so unseen strays cannot bind wrong (B4 admits 'University of Oxford' under RTH).

**D2. Amend 300 §10 in writing:** a reviewed, ODS-confirmed, members-exact body verdict may fold provisional clusters into a canonical row and alias its reviewed keys. Nothing else captures by name.

**D3. A frozen ODS table and check (ingest, pure).**
- The table is B's: 1,053 codes, with RO98, Inactive and five-character codes as rivals, plus predecessor names, RO107/RO172 and a version.
- A's 213-code allowlist decides who may be a survivor.
- The B4 check also refuses non-NHS institutions (university, council, ltd with no NHS role token), and a tie refuses.
- A member the check refuses needs HIGH with cited ODS rename or predecessor evidence; RCX 'Queen Elizabeth Hospital FT' is held.
*Why:* review answers to ODS; the check never binds.

**D4. Alias keys are the members' own 234 keys** (head `name_norm`, GB). A key owned by a standing identifier-NULL row outside the body is denied (`alias_key_owned_elsewhere`), its member held. *Why:* the 234 path matches head names (canonical.rs ~17357), so the alias never binds wider than today.

**D5. The alias reaches its target through `Some(identifier)`, on the synthesised PPON** (org_of → rekey → canon_of → altid alias → mint). Any mint of a ledger target takes the ledger's `keep_name`. *Why:* a survivor a later fold absorbs is followed, not re-minted, and the head name is order-independent.

**D6. Never members:** HOSTED_OR_SITE, NOT_ORG and DIFFERENT holders; minted-identity holders (519's); successor names under the 7 legally-ended codes; generic or consortium names; flagged rehomed or case-moved members. The QKK/QMF cross-assignment is dropped (it fails members ⊆ live), so both stay anchors. Verdicts come from the live listing only: adjudication-rows.json gives TAJ's PPON as 31542073, which is QMM's.

**D7. A scoped `nhs-body` unwind ships with the wet path and is tested before any wet run.** It retires a body's ledger rows and then dissolves the body through 234/E0. *Why:* nothing can undo these 223 merges today.

**D8. Any future forward arm goes in the resolver, never in `normalise_identifier`, and binds to the survivor's merged identifier, never to a twin.** *Why:* the identifier feeds buyer_key and the FTS ocid gate (canonical.rs:14088).

### Units
- **U0** This record and the 300 amendment. u1 corrections: NHS TRUST is 99; uncoded is 47,467; the TAJ id.
- **U1** D3. No behaviour change.
- **U2** The census, a dry `nhs-body` run: every holder uncapped, key ownership, check verdicts, D6 flags, country-less and bare GB-NHS values, non-allowlisted tails, coded share by notice year. Adds `reviewed_holders` and 1-member verdicts. Deploy dry.
- **U3** Adjudicate the 35 holders (reviewer plus challenger) and POST the verdicts. Also post 469 keep verdicts for the QKK, QMF, RXX, RTF, RGT, 0CX and RAL PPON pairs.
- **U4** One deploy, after 491 3(b) leaves canonical.rs and supervisor.rs: the wet path, the ledger-armed alias (boxed), the D7 unwind, the `nhs_body_merged` and `nhs_body_regrown` reports, and the re-resolve read.
- **U5** Dry → capped wet (20) → read → full wet.
- **U6 (conditional)** D8's arm. It opens only if the last-12-month coded share is above 9.5% and S4 shows coded new spellings regrowing. It ships only if its own dry census admits ≤ 0.5% non-SAME names.

### Stop rules
- **S0** No verdict for a body with an unread holder (11 such bodies today). 0 elsewhere-owned keys admitted.
- **S1** SAME agreement ≥ 98%. 0 flagged members without evidence. 0 successor-name members.
- **S2** Parity within max(2%, 5) bodies, otherwise abort before writing.
- **S3** Capped read: reviewed survivors, non-members untouched, survivor mentions = members' sum, losers 308, `?identifier=GBNHS<code>` answers the survivor for 20 codes.
- **S4** On every fold, alias refusals ≤ binds. On every dry run, `nhs_body_regrown` = 0.
- **S5** A store test that rebuilds in notice order yields one org per body. On prod, 3 notices × 3 bodies (TED uncoded, FTS coded, PPON-first) re-resolved through a fresh resolver must agree 100%.
- **S6** No rollback past U4 without the unwind first.

### Handoffs
- **469 U7:** digit-bearing NHS codes only. ODS confirmation and the witness check use D3's table. No letter-only REGISTRIES shape. Survivors are holders via merged identifiers. Keep verdicts deny the cross-body pairs.
- **519:** RO98 sub-ICB codes (incl. 10 Active `00[A-Z]`) and typo codes. Their orgs never join a body. D3's table classifies them, and A's (triple, name_norm) pin is a candidate fix. Lands before or with U7.

### Trade-offs accepted
- No `GBNHS` identity; lookups go through merged identifiers.
- Clusters that never carry the code stay split (E3).
- New coded spellings regrow until the next cohort, and a newcomer makes the verdicts stale.
- 13 anchor bodies stay provisional.
- Review costs ~2× 469 U6.
- Hosted sites stay separate.
- The legally-ended codes have no date fence.
- A one-member body's `GBNHS<code>` is lost on a from-archive rebuild: its fold writes no ledger row (no loser), and
  no marker row replays it (owner ruling on the U4 review, 2026-10-12). Its unwind drops the record.

### Owner notes on adoption (2026-10-11)

- **The 300 §10 amendment (D2) is accepted, scoped exactly as written.**
  - A reviewed, ODS-confirmed, members-exact body verdict may fold provisional clusters into a canonical row and arm
    an alias for its members' own 234 keys. Nothing else captures by name.
  - It is recorded in U0 beside 300's design doc and ADR-0003's 2026-10-02 amendment ("nothing is deliberately
    forbidden if it is correct"), with this issue as its only user.
- **The u1 corrections are verified** and land in U0:
  - NHS TRUST is 99, not 98;
  - over distinct orgs, the SAME clusters are 235 orgs carrying 47,467 uncoded and 7,362 coded mentions (the 238 / 48,360
    figures counted rows);
  - TAJ's PPON holder is 30917612, not 31542073. The adjudication agent recorded QMM's PPON org; corrected in
    `adjudication-rows.json`.
- **Sequencing.**
  - U1 (pure table and check) can be built and gated now.
  - U2 (the dry census) touches canonical.rs, but planner-only, behind `dry_run`, as unit 1 did. It may deploy once the
    491 drain ends.
  - U4 (the wet path) waits for U3's adjudication.

## U1 — built (2026-10-11)

- **Files.**
  - `crates/ingest/src/nhs_ods.rs` (the module, 25 tests plus an ignored tally harness);
  - `crates/ingest/sdk/nhs-ods.tsv` (`ods-2026-10-11`);
  - the generator `../513-nhs/u1-table/gen_nhs_ods_table.py` and its raw ORD cache (`--offline` rebuilds the TSV byte
    for byte).
- **The table.**
  - 1,062 rows: RO197 544, RO107 12, RO261 125, RO213 36, RO98 344, plus RO172 DQF.
  - 215 allowlisted: trust 163, care trust 4, ICB 39, CSU 8, DQF. The allowlist is 215 rather than 213 because the
    first RO107 fetch adds TAF and TAH.
  - No O/0 twin pair anywhere in the table.
- **The check: two changes beyond the decision's D3 wording, both owner-accepted.**
  - (a) `university` and `college` yield to `hospital`, the trust naming of University Hospitals Birmingham and
    King's College Hospital. Hospital charities stay refused through a `charity` marker.
  - (b) A company code's own legal form is no marker (DQF).
  - The check admits the REM group brand "University Hospitals of Liverpool Group". D6's adjudication, not the
    check, keeps that HOSTED holder out.
- **Legal ends (review finding).** 35 allowlisted codes carry an ODS legal end while ODS still answers them Active,
  not 7:
  - the 2026-04 ICB mergers: QJG, QMJ, QMM, QNQ, QXU, RNK, RYV, QHG, QNX, QRV, QUE;
  - TAF and RRP in 2024 (successor G6V2S);
  - some long before the GB-NHS scheme existed, e.g. RBF 2011 (successor RTH).

  The TSV now carries `legal_end` and `successors`, so U2's census can flag a code published after its legal end.
  QRV's own ODS name ties its successor Z9B2Z, so QRV is a Tie flag by construction.

## U3 — adjudicated and posted (2026-10-11/12)

- **Workflow** wf_126dc78d-0f3: 19 batches, one reviewer and one challenger each, holder by holder against ODS, the ORD
  API used where needed.
- **Holder classes**: SAME 454, HOSTED_OR_SITE 43, DIFFERENT 42, NOT_ORG 4, SUCCESSOR 2, UNSURE 1.
- **Posted**: 156 verdicts with reviewer and challenger agreed and HIGH (393 members), in
  `../513-nhs/u3/verdicts-nhs-body-2026-10-11.json`. Re-census 2264: **156 admitted, 0 not admitted**. That satisfies S0:
  no unread holder, and no elsewhere-owned key admitted.

### u3-residue: the 18 codes not posted

**Held by both, for a reason the decision names:**
- DQF: the only holder is a minted COH org, under review, and its key is owned elsewhere.
- QKK and QMF: the swapped London ICBs; both carry NE/SE London clusters and PPONs.
- QOP: medium confidence.
- QUY: a "North East Somerset" holder.
- RCB, RNK, RRV, RXF, RXT, RYC and RYG: keys owned elsewhere, successor or legal-end questions, typo codes.

**Disputed by the challenger:**
- QJM: a typo spelling the check refuses as Zero.
- QMJ: 31625150.
- QYG: 31579965, the operating brand "NHS Cheshire and Merseyside", which the check refuses.
- RHM: 30926769.

**No SAME holder:**
- RFF: the only holder is Epsom & St Helier, which is RVR.
- RVY: every coded notice falls after its legal end; this one belongs to 519's territory.

Each needs an owner read (the challenger's reasons are in `adjudication-rows-u3.json`) before a second cohort.

## U6 — survivor-exempt D4 (filed 2026-10-12 from the residue read)

`plan_nhs_bodies` gives `member-key-owned-elsewhere` to any member whose 234 key a standing identifier-NULL row
outside the body owns (canonical.rs ~26691). U4 arms the alias on LOSER keys only:
- the ledger alias reads `$.loser_name_norm` (`NHS_BODY_ALIAS_LEDGER_SQL`);
- `arm_nhs_body_alias_on` already refuses a loser key that such a row owns.

So when the caught member is the one `nhs_body_survivor` keeps, the gate protects nothing. It blocks the body's own PPON
org, whose uncoded twin owns its head key: RHM 30926769, RNK 31554431, RXT 30926645, RYC 30916110, RYG 30918096,
RRV 30916969 and RCB 31535291 (QUY's 31536069 has its own question). The uncoded twin stays split, which is the
accepted E3 trade-off.

Change: apply the key-owner test only to members other than the survivor the code would pick. Update
`a_member_whose_key_a_row_outside_the_body_owns_holds_the_verdict` (crates/store/tests/nhs_body.rs:916) so it still
holds a LOSER whose key is owned elsewhere, and add a test that admits a PPON survivor whose key is. Then deploy,
re-census, and post the seven verdicts as the read gives them (`../513-nhs/u5/residue-read-wf_2411cb6d.json`,
`unblock`), re-checking `coded_notices_after_legal_end` for RNK (legal end 2026-03-31, successor G6V2S).

