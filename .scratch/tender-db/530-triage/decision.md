# 530 unit 2 — decision (synthesis of the three judges, 2026-10-19)

Inputs: issue 530 (What was found, Proposed units, Unit 1 measured), census job 2655
(`guid_census_2655.json`, decoded body), `crates/store/src/guid_census.rs` (the twin key is `(notice, org_name_norm)`
of the mention's published name; `nibble_only` = version nibble 4 with a variant nibble outside 8-b),
`crates/ingest/src/idgate.rs` (`condemns` :453, `uuid_v4` :479), `crates/ingest/src/project.rs` (`gated_identifier`
:5314, `mentions_and_dropped` :5880-6052, `content_words` / `guard_name_words` / `generic_body_name` :10574-10740),
`crates/store/src/canonical.rs` (`repair_placeholder_orgs_batch` :41122, which feeds an id list to
`dissolve_condemned`), issue 312, 300 design :84-85 and §10, and the three judges' notes (`judge_precision.md`,
`judge_coverage.md`, `judge_operations.md`). I made 4 public GETs, listed at the end.

## Decision

**(a), pair-scoped, plus (c).** (d) is declined, notice-scoped (a) is declined, and (b) is declined here; its one
clean cell goes to a new issue.

- **Rule 530-T.** A pure test of one notice, run in the ingest projection beside `idgate::condemns`.
- **Exemptions.** A frozen, value-keyed keep-list (`KEEP_530`), the existing name-safety guard plus a 530-only
  list of generic names, and a group key that needs a country.
- **Stock.** One dissolve of the rows the rule reaches whose mentions never left one Tender, through 440's
  dissolve machinery with an id-set selector.
- **(c) for everything else.** The cost is written down here, and one sentence goes into the API docs.

Vote: two judges out of three (precision and operations) chose pair-scoped (a) plus (c). The coverage judge
dissented for (d) plus notice-scoped (a). Its numbers and its anchor observation are recorded below as the price
of this decision and as reopen conditions.

What each judge contributed:

| from | taken |
|---|---|
| operations | Gate placement in `mentions_and_dropped`, which makes it a pure function of the notice. A frozen value-keyed KEEP instead of an `org_of` lookup, so a refold, a 434 refresh and a from-archive rebuild agree. The id-set dissolve requirements: boxed, stoppable, windowed by target key, with a restore manifest, and the wet run re-plans. The interactions with 456, 473 and 572, and the scheduling. The 404 and change-feed cost. |
| precision | Exemption (i)'s intent, "a value already doing real linking keeps binding". KEEP is widened to carry it, see below. A group key that needs a country. The generic-name list for the guard. The dissolve tripwires. The R2 scoring (R/N/W) and its bar. Göttingen goes to 2294259 and never to 16393 or 22183653. |
| coverage | The case against (b) by axis (platform, profile, era, country). The (d) clause about a recorded key, and the anchors, kept for a reopen. The estimate for the hex class outside v4. Name-plus-town evidence in R2. The R3 check that the keys with the most folds are each one body. |

## Rule 530-T (wording for unit 3)

**Placement.** The rule runs in `crates/ingest/src/project.rs`, `NoticeState::mentions_and_dropped` (the body
behind `mentions_and_void`). It goes immediately after the `.map(... gated_identifier(...))` collect that sets
`m.identifier`, and before the `extracted` loop, so that `ExtractedWinners` sees the final identity.
- It sits beside `idgate::condemns`, not inside it: `condemns` sees one value, and this rule needs the whole notice.
- `condemns` and `uuid_v4` stay unchanged.
- It is a pure function with no DB read and no resolver change. Call it
  `guid_twin_refusals(&[TwinView], &Keep530) -> refused section ids`.
- The unit-3 planner calls the same function, passed to store as a fn pointer, the way `condemns` is passed to
  `repair_placeholder_orgs_batch`. So the dry plan, the gate and any rebuild decide with one ruler.

**Per notice.** A TED copy and a DÖE copy are each their own notice and are each decided alone.

1. **Candidates.** Mentions whose gated identifier has kind `national` and whose value passes `idgate::uuid_v4`.
2. **Groups.** Group the candidates by `(identifier.country, store::org_name_norm(mention.name))`. This is the
   census's twin key, made finer by the country, and it is the same key the 234 fallback resolves on. A mention
   with an empty name or no country joins no group. Census 2655 has 0 GUID rows with a NULL country, so this costs
   no reach in the stock; it keeps a refusal from ever landing on a country-NULL row such as 16393.
3. **Twin.** A group holding **2 or more distinct values**.
4. **Refusal.** In a twin group, set `identifier = None` on every candidate whose value is **not in
   `KEEP_530`**. `raw_identifier` and `scheme` stay as published. The mention then resolves through the
   issue-234 identifier-NULL path: the `(name_norm, country)` row with identifier NULL, or a fresh provisional
   row. It never lands on an identified row (300 §10).
5. **Name guard.** If the group's name is not safe as a fallback key, the whole group binds as before. Not safe
   means any one of these:
   - `content_words(guard_name_words(name)) < 2`. This already excludes bare `Vergabekammer`, `Vergabestelle`,
     `Tiefbauamt`, `Hochbauamt`, `Bauamt`, `Bauverwaltung`, `Gemeindeverwaltung`, `Finanzbehörde` and
     `Beschaffungsstelle`.
   - `generic_body_name(words)`, the existing list, unchanged.
   - The folded words equal an entry of a **new 530-only list** `GUID_TWIN_UNSAFE_NAMES`. It is seeded with the
     multi-word generics the precision judge's GETs found (`Zentrale Vergabestelle`, `Tribunal administratif`)
     plus whatever R2 convicts.

   The list is separate on purpose. Adding these words to `GENERIC_BODY_NAMES` would change the head and prefix
   tokens of 481's buyer guard (`project.rs:10547`, `:10827`), which is a different feature.
6. **Everything else is untouched.**
   - A GUID in no twin group binds or mints as today. That covers the Südbayern three-notice shape, Bremen, CH,
     and a stable key recurring across notices (DB, 8378).
   - An SBH-shaped pair with both values in KEEP binds.
   - A KEEP value twinned with a fresh one: only the fresh one is refused.

**`KEEP_530`** is frozen and value-keyed, in `crates/ingest/sdk/guid-keep-530.tsv`, read through `include_str!`
(513's `nhs-ods.tsv` precedent). Its columns are country, value, org id at freeze, mentions, tenders and reason.
- **Contents.** Every uuid_v4 value whose row spans **≥ 2 distinct Tenders** (7,300 at job 2655), plus the
  **stable population**: 254, 255, 8378 and every uuid_v4 row with ≥ 100 mentions (345; 13 of them single-Tender).
  That makes **7,313 values**.
- **The minimum the bars need** is 454 values: the 179 twin rows spanning several Tenders plus the 345 stable rows.
  The wider set carries the precision judge's exemption (i), "a value doing real linking keeps binding", without
  its dependence on database state. It also covers pairs below the stable bar that span several Tenders, such as
  2529/2530 at 338/328 mentions and 21983973 at 72. In the stock it changes nothing, because the dissolve takes only
  rows spanning one Tender.
- **Changes.** No job edits the file. It changes only by a reviewed commit that adds values, after the monthly
  census shows a twinned row spanning several Tenders outside KEEP. Values are never removed.
- **Pinned by a test.** 254, 255, 8378, 3866, 3868, 2529, 2530, 21983972, 21983973 and 21987063 must be inside it.

**Thresholds in one place.**
- Stable means 254, 255, 8378, or ≥ 100 mentions (census `stable_mentions`).
- Single-use means the mentions span exactly 1 distinct Tender (`tender_versions.caused_by_notice_id`).
- The ≤ 2-mention bar (`GUID_CENSUS_FEW_MENTIONS`) belongs to the declined (d) and is not used.

**Gate doc comment, which must say this.** Identity becomes contextual. A tool that rebuilds a mention's identity
from its stored row alone sees a GUID where the fold refused one. Those tools are `gated_identifier`'s callers: 513's
nhs-body canary and 511's guard 3. None of them reads DE GUID mentions today. A future reader re-derives the twin
from the notice's stored mentions, as the census does.

## What is declined, and why

- **(d), cross-notice name equality.** Declined by two of three judges.
  - It is a stateful lookup at mint time that depends on order, in the fold's write path, with two
    implementations (batch and rebuild). Org ids are not mint order, so the census's 55,783 and 11 are id-order
    figures.
  - It consumes its own evidence. It needs a marker store and a recurring dissolve, and that dissolve makes
    "added, then 404" the normal life of about 1.8k orgs a month.
  - It would have refused 11 stable rows at mint (Vergabekammer Westfalen 5351, 8756, 21987063) and 1,570 rows
    spanning several Tenders.
  - Its fallback lands generic names in buyer roles: `Tiefbauamt` CH ×9, `Hochbauamt` CH (the name already sits on
    3 national keys), and `Vergabekammer` DE ×24 into 1431661 (4,280 mentions of every chamber).
- **Notice-scoped (a).** Its premise is false by measurement. 254 and 255 sit in twin notices (114 stable rows
  reached), so a notice mixes stable and fresh keys.
- **(b), by publisher or era.** No measured axis is clean.
  - Platform: doe+ted dashed-lower holds 40,388 single-use rows and 338 of the 345 stable rows; ted dashed-lower
    has 1,821 rows spanning several Tenders.
  - Profile is a schema era that 254 and 255 share.
  - Year: single-use share 70.6 → 89.0 → 92.2 → 96.1%, with no cutover.
  - CH is per role.
  - The publisher fields were not measured.
- **530-F, the FR 8-4-4-16 cell (3,511 rows, 0 spanning several Tenders, 0 stable).** Deferred to a new issue and
  not shipped here.
  - The cell is clean, but `nibble_only` = 10,548 ≈ 3 × 3,511. That is the 3:1 ratio random hex gives
    (P(version 4, variant outside 8-b) = 3/64 against P(passes uuid_v4) = 1/64).
  - So these rows look like the 1/64 of one platform's random keys that pass the v4 test by chance, out of a
    population of roughly 225k (estimate).
  - Refusing 1/64 of a platform inside an issue scoped to v4 is arbitrary.
  - The new issue takes the raw 8-4-4-16 shape across the whole hex class. It starts with the precision judge's
    R5 as its bar: every mention carries the 35-character shape, 0 rows of the shape span ≥ 2 Tenders, 0 are
    stable, and 15 FR targets read at 0 W.
- **Exemption by bound key (an `org_of` lookup).** Declined in favour of the wider frozen KEEP. A from-archive
  rebuild re-mints ids from 1 with an empty `org_of`, so a bound-key rule would give a different org layer
  depending on order.

## Dissolve plan (unit 3)

**Selector.** An id set from the planner. `repair_placeholder_orgs_batch` takes only a value predicate, so its
transaction tail, which already hands an id list to `dissolve_condemned`, is factored into a helper that takes ids.
The `placeholder_dissolve` tests stay green.

A row is dissolved **if and only if** all of these hold:
- its identifier passes uuid_v4;
- its value is not in `KEEP_530`;
- at least one of its mentions sits in a twin group that step 4 refuses, with the guard applied;
- its mentions span **exactly 1** distinct Tender (tripwire);
- it has **< 100** mentions and is not 254, 255 or 8378 (tripwire).

A planned row that fails a tripwire aborts the run.

**Targets.** Each recorded mention re-resolves through the post-234 provisional path, as 440 does. The target is the
mention's own `(org_name_norm(name), country)` identifier-NULL row. Party and bid-party rows follow their
(notice, section) link, and winners follow through tier 2.

**Job.**
- A new kind, `dissolve-guid-twins`, in `STOPPABLE_KINDS`. `repair-placeholder-orgs` sits in `UNSTOPPABLE_KINDS`
  because it takes seconds, and this job takes tens of minutes.
- The `Spec` arm is wrapped in `Box::pin(async move { ... }).await`.
- A stop checkpoint between windows (543).
- **Windows go by target key**, not by org-id range. 542's re-derivation of name variants inside a window reads
  every mention of each rebound org, so org-id windows would re-derive a large chamber row once per window.
- `dry_run` defaults to true. The job takes `extra_unsafe_names: [..]` so that R1 can be re-run after an R2 finding
  without a deploy.

**Report.**
- Planned rows and mentions, by country, role and first year.
- Guard exclusions by reason: content words, generic, the 530 list, no country, KEEP partner.
- Rows that have a mention outside any refused group (TED and DÖE copies that disagree).
- Windows, the largest target, and the runtime.
- The canaries.
- The KEEP candidate list, the stable population plus rows spanning several Tenders, as country, value, id,
  mentions, tenders and reason. It is about 240 KB, and Build 2 commits it.
- The R2 sample with its evidence. For each row: id, value, name, country, role and notice ids. For each target: id,
  name, mentions, and 3 distinct notice ids of its existing mentions.

**Wet run.** It writes a **restore manifest**: each dissolved org's id, country and value, every
(notice_id, section_id), and the target. It **re-plans** instead of trusting the dry list. Its count must equal the
dry count, minus the R2 guard additions, plus the rows minted since; every tripwire is re-checked.

**Copies that disagree.** If more than 1% of planned rows (about 113) have a mention outside any refused group, the
selection narrows to rows whose **every** mention the rule refuses. The difference is recorded as kept under (c),
and this is not a stop.

**Afterwards.** A refold moves nothing, and keeps the new binding: `RecordedMention::publishes` (434) compares name,
country, raw and scheme, and the gate changes none of them. That is why the dissolve is required. The deploy itself
causes no churn.

**Cost.** About 11k `organization removed` rows and about 10k Tender `changed` rows, once. Dissolved ids answer 404
with no redirect, as in 440, 510 and 522. At job 1615's rate (425 orgs in under a minute) the drain takes roughly
30 minutes.

**Schedule.**
- Run in a queue gap after 551's flip, and not on 560's drain-free mornings.
- Serialize with 473's drain on the single writer.
- 456's measurement runs after this drain, or excludes v4 rows.
- 572 is untouched: agent rows span several Tenders, and KUBUS 11107 and EKK plus 21986305 are in KEEP.

## Build sequence

1. **Build 1: no change to identity.**
   - The pure `guid_twin_refusals` and the name guard, with the R0 unit tests.
   - The planner arm, dry only. Its KEEP is computed in memory from the census walk, with the same definition.
   - Not wired into the projection: the gate cannot be wired before the KEEP file exists, or it would refuse SBH,
     GMH and Nürnberg pairs.
   - `ops/check.sh`, deploy, dry plan, then **R1 and R2**.
2. **Build 2: only after R1 and R2 pass.**
   - Commit `guid-keep-530.tsv` from the dry report.
   - Compile in R2's guard additions.
   - Wire the gate, enable the wet path, and add the resolver test (R0's last line).
   - Add the docs sentence.
   - `ops/check.sh`, deploy. Between the deploy and the wet run, the gate refuses only new twin notices, and does so
     consistently.
3. **Wet run** in a queue gap (re-plan, tripwires, manifest), then **R3**.

## Read that must pass before Build 2 ships

**R0, tests (`ops/check.sh`, in `project.rs` or `idgate.rs`, plus one resolver test in Build 2).**
- A Göttingen-shaped twin (Procedure-Buyer and Lot-AddInfo on two sections, two values) refuses both. The resolver
  lands both on the DE identifier-NULL row, never on the VAT row.
- The Südbayern three-notice shape (no twin) binds.
- A DB-platform GUID recurring across notices binds.
- An SBH-shaped KEEP pair binds.
- A KEEP value plus a fresh value refuses only the fresh one.
- A bare `Vergabekammer` twin binds, and so does a `Zentrale Vergabestelle` twin.
- A twin with no country binds.
- The TED copy and the DÖE copy are each decided alone.
- The planner and the gate give the same refused set on a fixture. This is the parity test.

**R1, dry plan (in-app, Build 1, writes nothing). Exact bars; any miss means stop.**
- Planned ≤ **11,258**. Twin rows minted above id 31,970,152 are counted separately.
- **0** planned rows with ≥ 2 Tenders, with ≥ 100 mentions, or with a value in KEEP.
- **Not planned:** 254, 255, 8378, 30530641, 3866, 3868, 2529, 2530, 21983972, 21983973, 21987063, 5351, 8756,
  10966, 1085, 22040636, 22040637. Also the no-twin controls: 23014305, 30971684, 30971734, 30971757, 30532145,
  30532147.
- **Planned with target 2294259:** 22759261 and 22759262. The census marks both as pair twins.
- Expected and reported, but not bars because they are inferred from id adjacency and one GET:
  - the Göttingen triple 22761155, 22761156 and 22761159, with target 2294259;
  - Vergabekammer Niedersachsen 23014306 and 23014307, with target that name's DE identifier-NULL row.
- Every target has identifier NULL, in the mention's country.
- **KEEP self-check:** the walk's (twin ∧ ≥ 2 Tenders) ∪ stable set is contained in the KEEP candidate list.
- Reported and checked against the expectations below: the Göttingen planned and kept split (49 and 3 expected),
  the guard exclusions, and the share of copies that disagree.

**R2, fallback precision (n = 60).**
- **The sample.**
  - (1) The **30 targets receiving the most planned mentions**, ties broken by target id. Any of the **20 targets
    with the most planned rows** that is not already among them is added (coverage's R3).
  - (2) **30 random planned rows, stratified**: review_or_info 15, buyer 10, other or tenderer 5. Within each
    stratum, take the rows with the smallest `((org_id XOR 530002) × 2654435761) mod 2³²`, the salted-uniform
    formula of `head_election.rs:1456`.
  - The salt **530002** is fixed in the job before the dry run.
- **Scoring.** Each sampled row or target is scored on name plus town or address from the notices, and on the
  target's other mentions:
  - **R**: the target is the same body.
  - **N**: cannot tell.
  - **W**: the target's name denotes more than one body, or the target holds another body's mentions.
- **Also read** every planned key with exactly 2 content words whose first word is an office noun (Amt,
  Vergabestelle, Bauamt, Referat, Abteilung, Fachbereich, Zentrale, Hochbauamt, Tiefbauamt and the like). If there
  are more than 45, read the top 30 by mentions plus 15 at salt 530002.
- **Bar: 0 W in 60, N ≤ 3.** 0 in 60 puts the 95% upper bound near 5%.
- **On any W:** add that name or shape to `GUID_TWIN_UNSAFE_NAMES` (via `extra_unsafe_names`), re-run R1, and read a
  fresh random 30 at salt **530003**, excluding rows already read. That 30 must come back 0 W.

**R3, after the wet run.**
- The Verify block below.
- The manifest's mention count equals the mentions moved.
- A `guid-census` re-run:
  - `one_tender_twin` falls to about the guard exclusions plus any narrowed rows;
  - `twin.stable_rows` stays 70, because the KEEP pairs are still twins;
  - `twin.rows` is about 179 plus the exclusions;
  - stable stays at **345 rows and at least 311,552 mentions**;
  - `multi_tender` stays at 7,300, give or take new mints.

## Expected effect (census 2655)

**Stock dissolve.**
- At most **11,258** single-use rows, 15.2% of 74,023, less the guard exclusions (not measured; bare one-word chamber
  names are among them).
- By country: DE 11,237, FR 18, PT 2, AT 1.
- By role: review_or_info 8,262 (mostly Vergabekammer ReviewInfo/ReviewOrg pairs), buyer 2,774, other 183,
  tenderer_winner 39.
- About **22k mentions**, about 2 per row (the TED and DÖE copies). The dry plan gives the exact sum.
- **0 rows spanning several Tenders and 0 stable rows, by construction.** Stable `one_tender_twin` = 0, and all 179
  twin rows that span several Tenders are in KEEP.

**Göttingen (Verify line 1).**
- 49 of the 52 GUID rows are expected to fold into **2294259** (DE, identifier NULL, 216 → about 314 mentions).
- 3 are expected to stay: **23014305, 23055364 and 23110616**. The basis:
  - 40 of the 52 ids come in consecutive (n, n+1) pairs, and 9 more sit at n+4 of a pair, a third section in the
    same notice.
  - The 3 have no neighbour.
  - GET: in tender 623993, 23014305 carries Procedure-Buyer *and* Lot-AddInfo on **one** section. That notice's twin
    is Vergabekammer Niedersachsen 23014306/23014307, which folds instead.
- Reason recorded for the 3: "no within-notice twin (one ORG section); cross-notice name equality declined (530
  unit 2)". The dry plan's count is authoritative.
- Nothing goes to VAT row 22183653 (649) or to country-NULL row 16393 (804).

**Verify lines 2-3.** 254 (24,219 today) and 255 (73,039) are in KEEP and are not pair twins, so they hold.

**Forward.**
- Twin minting has nearly stopped. Single-use twin rows by first year: 2023 10, 2024 7,459, 2025 3,753,
  **2026 36**.
- So the gate's job is durability: a rebuild or a 434 refresh cannot re-mint what the dissolve removed.
- No weekly upkeep. The monthly 12-second census is an optional monitor.

**Left by decision (c).**
- **62,765 single-use rows (84.8%)**, plus the guard exclusions:
  - CH 27,837 (0 pair twins);
  - DE about 30,803 (the Südbayern and Bremen no-twin shape, the Westfalen runs);
  - FR 3,579 (including the 8-4-4-16 cell, deferred);
  - other about 546.
- Also left: the 7,300 rows spanning several Tenders, which bind by design.
- Live growth is about **1.8k single-use rows a month** (17,533 first seen in 2026 up to 10-19, 13,085 of them
  reached by the cross signal). This decision does not stop it.

## Verify (to replace the block in the issue when unit 3 lands)

The commands are unchanged.
- **Done:** line 1 reads `3`, or the dry plan's exact Göttingen residue, recorded with the reason above. Line 1 read
  `52` on 2026-10-14. Lines 2-3: both rows are still GUID-keyed, with mentions not lower than 24041 and 72723
  (24,219 and 73,039 on 2026-10-19).
- **Also:**
  - `/v1/organizations/22759261` and `/22759262` answer 404;
  - 2294259 reads about 314 mentions;
  - the census re-run meets R3.

## docs sentence

Where: the `identifier` lookups bullet, `docs.rs:284` today. The issue's `:880` was e2cbbb8's line.

> A v4 UUID in `identifier` (`identifier_kind: "national"`) is a submission platform's record key, not a register
> number. Some German and Swiss platforms mint a fresh one per notice, so one body can appear as many organizations
> with one or two mentions each; aggregate such rows by name. Where one notice names a body twice under two such
> keys, the keys are dropped and its mentions join the body's name-keyed organization (issue 530).

## Risks

1. **Coverage, by decision.** 84.8% of the single-use class and nearly all of 2026's minting stay split, the
   cross-reached kind included. The reopen conditions are below.
2. **The guard is incomplete.** A two-word generic, or a town name shared by two places (`Stadt Neustadt`), can fold
   two bodies into one identifier-NULL row. R2 bounds this. It can be undone: `raw_identifier` stays on every
   refused mention, and the manifest maps value → (notice, section) for an `apply-rehoming` restore.
3. **Some fallback rows already hold more than one body** from 234 (`Hochbauamt` CH 6956143, `Vergabekammer` DE
   1431661). The guard keeps those names out, and R2 reads the targets.
4. **KEEP is frozen.** A stable pair born after the freeze loses its twin-notice mentions to the 234 row, a one-time
   split. The bound is 36 single-use twin rows in 2026; the monthly census flags it, and a reviewed commit adds to
   KEEP.
5. **The TED and DÖE copies can disagree,** splitting one notice pair between a GUID row and the 234 row. It is
   counted, and above 1% the selection narrows.
6. **New id-set code in the dissolve transaction that 440 and 522 share.** The tests stay green, and 543's
   checkpoint and 542's windowing are both required.
7. **Identity becomes contextual** (see the gate doc comment).
8. **About 11k dissolved ids answer 404 once,** with no redirect.

## 300 design:84-85 ("merge-inert singletons, not junk to dissolve"), answered

**Upheld for the class, amended for a sub-class proven per notice.**
- **"Merge-inert" stands.** `canonical_key` returns None for a v4 UUID (312 step 1a), 312 found 75,548 distinct
  values over 75,555 rows, and nothing here makes a GUID a merge key. `condemns` and `uuid_v4` are unchanged.
- **"Singletons" is now measured, and it is the defect, not inertness.** 91.0% of v4 rows (74,023 of 81,323) span
  one distinct Tender. A key minted per ORG section per notice never merges, so its split never heals. 312's "93%
  span more than one notice" counted the TED and DÖE copies: on the Tender basis it is 9.0%.
- **"Not junk to dissolve" stands for the class.** 7,300 rows span several Tenders, and the 345-row stable population
  (311,552 mentions, 96.2% spanning several Tenders) does real linking. 70 stable rows sit in twin pairs. So there is
  no class-wide condemn and no class-wide dissolve.
- **The amendment.** A row is dissolved when all of these hold:
  - its own notice publishes one name-safe name under two v4 values;
  - its value is not on the frozen KEEP list;
  - it never left one Tender.

  It is dissolved as a **per-notice key whose 234 row is the entity**, not as junk. The value survives on
  `raw_identifier`, and the grounds are evidence inside the notice, not the value's hex shape.
- **Everything else stays a merge-inert singleton by decision (c),** with its cost recorded here and in the docs.
- **Not decided here:** the wider 273,982-row hex-hash population, including the FR 8-4-4-16 platform (new issue).

## Reopen conditions and follow-ups

- **Reopen (d), or a successor, only in a form that is pure at the gate or convergent at the stock, and measured
  first.** Either of:
  - (i) a mint-order replay of (d) (an in-app dry job replaying GUID mints in notice-ingest order) that shows 0
    stable rows refused under an exemption, plus a fallback read at 0 W in 60;
  - (ii) a probation design: a new GUID outside KEEP binds only once its value recurs in a second Tender, promoted
    from the kept `raw_identifier`. Measure first the promotion lag and the share of stable keys still in their
    first Tender.

  The coverage judge's "recorded key" clause (a marker, or a v4 `raw_identifier` on the key's 234 row) and its anchor
  fold (up to 4,481 more rows) are the parts to reuse. Either reopen must account for the 404 and change-feed cost of
  anything that dissolves on a schedule.
- **Reopen (b) for CH** if a country × role breakdown shows a cell with 0 rows spanning several Tenders (CH buyers
  look per notice, and CH bidders keep their keys, e.g. FR_Swisscom at 53 Tenders).
- **New issue:** per-notice hex keys outside `uuid_v4`: the FR 8-4-4-16 shape (Avenue-Web; 3,511 rows pass v4,
  10,548 are nibble-only, about 225k estimated) and Avenue-Web Systèmes' 191 one-mention 32-hex rows (6 of them v4).
  Its starting bar is 530-F's R5.
- **Unchanged from the issue:** the Köln `T0221974921104` phone side note (size it with the other phone values first).

## GETs (4 of 15, at least 2 s apart)

1. `/v1/organizations?name_prefix=Landkreis Göttingen&limit=200`: 52 GUID rows, all at 2 mentions (104). 40 sit in
   consecutive-id pairs, 9 at n+4 of a pair, and 3 have no neighbour (23014305, 23055364, 23110616). Non-GUID rows:
   14107 `11389` (97), 16393 country NULL (804), 2294259 DE NULL (216), 22183653 `DE308703297` (649), 22426277 (2).
2. `/v1/organizations/23014305`: Landkreis Göttingen, `7FA17C8B…`, national, 2 mentions.
3. `/v1/tenders?buyer=23014305`: one Tender, 623993 (VMware support, Landkreis Göttingen).
4. `/v1/tenders/623993`: Procedure-Buyer **and** Lot-AddInfo → 23014305, the same row. Lot-ReviewInfo → 23014306
   and Lot-ReviewOrg → 23014307, both `Vergabekammer Niedersachsen beim Nds. Ministerium …`, which is a twin.
   Lot-TenderReceipt → 22761159, the third row of a Göttingen triple from an earlier notice.

## Critic amendments

Adversarial critic, 2026-10-19. Inputs: the issue (whole), this file, census 2655 (decoded body, all samples),
`guid_census.rs` (the walk reads only mentions bound to GUID rows, `guid_census_mentions_sql`; the twin key is
`(notice, org_name_norm)` over those, with no country), `idgate.rs`, `project.rs` (`mentions_and_dropped`,
`normalise_identifier_with` :11806, the guard helpers :10574-10827), `canonical.rs` (the 234 `(None, None)` arm
:23545-23640, `RecordedMention::publishes` :15578, `dissolve_condemned` :41332), the three judges' notes, and 7 public
GETs (listed at the end).

**Verdict: adopt the decision with these amendments. A1-A3 are blocking: they fix correctness defects in the
monitor, in stock/gate parity, and in the KEEP self-check. Build 1's report must carry the fields that A2, A3, A5-A7
and A9 add, so the amendments land before Build 1 starts.**

What holds: 254, 255, 8378 and the 345 stable rows are spared. The stock is protected by KEEP plus the
<100-mention and 1-Tender tripwires. The gate is protected by KEEP, provided A3 and A4 hold. `publishes` (:15578)
compares name, country, raw and scheme and never the identifier, so the deploy itself moves nothing and the
dissolve's bindings survive refolds. The census definitions are read correctly for single-use (1 Tender, not 1
notice) and for cross (an earlier, lower-id row; the decision already calls the 55,783 and the 11 "id-order
figures"). One premise is misread: the census twin is not an upper bound for the gate (A3).

### A1 (blocking). The KEEP-addition trigger cannot fire

**Defect.** The census walks `organizations.identifier` only (`length(identifier) IN (32, 36)`). A refused mention
sits on an identifier-NULL row, and its value survives only in `organization_mentions.raw_identifier`, which has no
index. After Build 2, two cases are invisible to it:
- A stable pair born after the freeze that is always twinned mints no row at all.
- A dissolved value that recurs in a second Tender through a non-twin notice mints a fresh row with 1 Tender.

Neither is "a twinned row spanning several Tenders outside KEEP". So Risk 4's "the monthly census flags it" and the
KEEP-change rule cannot trigger.

**Amendment.** Add a refused-value recurrence arm to `dissolve-guid-twins` in dry mode. R3 runs it once, expecting
0, and the monthly monitor runs it after that.
- **Input:** the manifest's target ids, plus the identifier-NULL targets of twin groups in notices ingested after
  Build 2's deploy. The deploy-time max notice id is recorded in the wet-run report. The groups are re-derived from
  those notices' stored mentions by the same `guid_twin_refusals` fn pointer.
- **Walk:** the targets' mentions through `organization_mentions_org`.
- **Count:** for each refused v4 value, the distinct Tenders over its refused mentions plus the mentions of its own
  GUID row. That row is found by an `organizations_identifier_id` seek.
- **Output:** any value at 2 or more Tenders is a KEEP-add candidate, listed with its restore cohort (value →
  (notice, section)).

Rewrite Risk 4 and the KEEP "Changes" bullet to name this arm, not the census.

### A2 (blocking). The stock and the gate diverge on rows with a mention the gate would not refuse

**Defect.** `dissolve_condemned` works on whole orgs: it re-resolves every mention by (name, country). Under "Copies
that disagree":
- At or below 1%, whole rows are dissolved, which also moves TED or DÖE mentions that the gate would bind.
- Above 1%, the selection narrows, which keeps refused mentions on GUID rows that the gate would refuse.

Either way, a from-archive rebuild gives a different org layer for those rows. Durability is the gate's stated
purpose.

**Amendment.** The unit of the plan is the (notice_id, section_id) mention, not the row.
- **Every mention refused:** dissolve the whole row, as planned.
- **At least one mention not refused:** rehome exactly the refused mentions to their 234 target. Use a per-mention
  rebind inside the same id-set helper: parties and bid parties follow (mention_notice_id, mention_section_id), and
  winners follow the 309 origin tier. The row keeps its other mentions.
- Drop the 1% switch.
- **R0 adds a test:** a two-copy fixture (DÖE copy twinned, TED copy naming the body once). The stock plan's
  post-state must equal a fresh projection of both copies under the gate.
- **R1 reports** the partial rows and their mentions.

### A3 (blocking). The planner must see what the gate sees, with the gate's constants

**(i) Inputs.** The census twin is computed only over mentions bound to GUID rows. The gate sees every ORG mention of
the notice through `gated_identifier`.
- The planner builds each `TwinView` from all ORG mentions of a candidate notice: an `organization_mentions` PK prefix
  on notice_id with `section_id >= 'ORG-' AND section_id < 'ORG.'` (the `NHS_BODY_PARTY_MENTIONS_SQL` idiom). Each
  mention passes `gated_identifier(raw_identifier, scheme, country)`.
- R1 reports separately the refused mentions that are bound to a row that is not uuid_v4, by bound kind:
  identifier-NULL, other identified, or merge survivor.
- The "≤ 11,258" bar applies only to rows whose every twin partner is bound to a GUID row. The rest is reported and
  read in R2, not compared to the census.

**(ii) Merged values.** KEEP also takes every v4 value in `organization_merged_identifiers`. A merge already linked
that value, and its holder is the binding the resolver serves.

**(iii) Constants.**
- In Build 2 the job loads the compiled `include_str!` KEEP and the compiled `GUID_TWIN_UNSAFE_NAMES`, the same
  statics the gate reads.
- A wet run with a non-empty `extra_unsafe_names` is refused.
- Build 1's KEEP self-check is a tautology: one walk computes both sides. Re-run it in Build 2 against the compiled
  file, in a dry re-plan after deploy and again in the wet re-plan. A value of (twin ∧ ≥ 2 Tenders) ∪ stable that is
  missing from the file aborts the run, and the report lists it as a KEEP-add commit.

### A4. Normalise the KEEP lookup, and test it through the real path

**Defect.** A KEEP miss on SBH 3866/3868, GMH 22040636/37 or Nürnberg 2529/2530 refuses a stable pair in every twin
notice of a rebuild or a new fold. The stock tripwires do not cover the gate.

**Amendment.**
- Match on the undashed upper-case 32-hex value only, ignoring country. A v4 collision across countries is
  negligible, and ignoring the country can only spare more values. Build the set once, in a `OnceLock`.
- **R0, SBH test:** run it through `mentions_and_dropped` on a parsed fixture whose BT-501 values are published
  dashed lower-case (the `fb197f94-7578-…` shape), not through a hand-built `TwinView` holding stored-form values.
- **R0, second test:** 255's value published under another country is still kept.

### A5. R2 is under-powered, and its stated bound is wrong

**Defect.** Only the 30 random rows are a sample. 0 W in 30 bounds the error rate at about 10% (3/30), which is up to
about 1,100 of 11,258 rows. The buyer stratum (10 rows) bounds buyers at about 26%. The top-30 targets are a
concentration check, not a sample, so "0 in 60 → about 5%" does not hold.

**Amendment.**
- The random stratum becomes 60: buyer 25, review_or_info 25, other or tenderer 10. Same salted formula, salt 530002.
  Bar: 0 W and N ≤ 3 in the 60.
- The top-30 and top-20 targets are read separately, with a bar of 0 W.
- On any W, the fresh re-read is 60 rows at salt 530003.
- Rewrite the bound sentence so it applies to the random 60 only.

### A6. Screen every planned row for homonyms (the fallback-row question)

**Defect.** The 234 target is keyed only on `(org_name_norm, register country)`, so two bodies with one exact name in
one country share it. Two GETs show the shape:
- `Stadt Neustadt` DE identifier-NULL row 2371794 (18 mentions), beside 10 distinctly named Neustadt towns;
- `Freistaat Bayern` DE identifier-NULL row (203 mentions), covering many state bodies.

Random rows cannot find a homonym that is rare.

**Amendment.**
- **Cities.** The dry plan reads each planned mention's city: the `notice_texts` PK `(notice_id, section_id,
  'BT-513-Organization-Company')`, first ordinal, folded. It also reads each target's city set over the target's
  newest 50 mentions, via an `organization_mentions_org` reverse walk and then the same PK seek.
- **Flags:**
  - (a) a planned row whose city is absent from a non-empty target city set;
  - (b) a target whose existing mentions show 2 or more distinct cities.
- **R1** reports both counts.
- **R2** reads every flagged target. If there are more than 45, it reads the top 30 by planned mentions plus 15 at
  salt 530002. Bar: 0 W. A W name goes into `GUID_TWIN_UNSAFE_NAMES`.
- A target that already holds two bodies scores W, even if the folded mention itself is right, because the fold
  extends a wrong merge.

### A7. Role-incoherent twins (a buyer and a court under one name; a buyer and its department)

**Defect.** The twin proves only that one notice printed one name twice under two values.
- When the buyer's department prints the buyer's name, folding into the buyer's 234 row is what the notice says. It
  is one legal entity; SBH, GMH and the Nürnberg pairs are this shape and are kept by KEEP.
- When the second section is a different body printed under the buyer's name (a Lot-ReviewOrg or a winner), the fold
  attaches a review-body or winner role to the buyer's row.

**Amendment.**
- **R1** reports the refused groups by the role sets of their sections: `tender_version_parties` by
  (mention_notice_id, mention_section_id), and winners by result rows. It lists every group that mixes:
  - a buyer-family role with Lot-ReviewOrg, or
  - a tenderer or winner role with any role that is not a winner.
- **R2** reads all of them. If there are more than 45, it reads the top 30 plus 15 salted.
- **On any W:** before Build 2, `guid_twin_refusals` gains a role-coherence guard under which such a group binds. Both
  the gate and the planner compute it from the notice's own OPT-300 and OPT-301 role references.

### A8. Gaps in the name guard

**(i) Legal forms.** Legal-form tokens count as content words and defeat the generic list. `Stadtwerke GmbH` folds to
`stadtwerke gmbh`, which is not `stadtwerke`, and it has 2 content words. Evaluate `content_words` and both generic
lists on the folded words with legal-form tokens removed: gmbh, mbh, ggmbh, ag, kg, kgaa, se, ug, ev, aor, co, ohg,
gbr, sa, sarl, sas.

**(ii) Umbrella names.** Seed `GUID_TWIN_UNSAFE_NAMES` with:
- polity umbrellas: `<Freistaat|Land|Bundesland|Kanton|Canton> <one word>`, `Bundesrepublik Deutschland`, `Freie und
  Hansestadt Hamburg`, `Freie Hansestadt Bremen`, `Republik Österreich`;
- person placeholders: `Natürliche Person`, `Natürliche Personen`, `Privatperson`. The GET found no GUID rows under
  these today, so this entry guards future notices.

**R0 tests:** a `Stadtwerke GmbH` twin binds, and a `Freistaat Bayern` twin binds.

### A9. Two R3 bars will fail falsely

**Defect.** "`twin.stable_rows` stays 70" and "`twin.rows` ≈ 179 + exclusions" assume that every twin partner of a
stable row is a KEEP value. The census twin needs 2 or more GUID rows, so a stable row twinned only with fresh
partners stops being a twin once those partners are dissolved.

**Amendment.**
- The dry plan computes the expected post-state: the stable twin rows, less those whose every twin partner is
  planned. It does the same for `twin.rows`. R3 compares against those figures.
- Add a per-row check: each of the 345 stable rows still carries its value, with mentions at or above its census-2655
  count. This replaces relying on the 311,552 sum alone.

### A10. Verify line 1 is not stable

**Defect.**
- "Reads 3" is an inference for 23055364 and 23110616; one GET covered only 23014305.
- New no-twin Göttingen GUID rows minted after the wet run raise the count, which is correct under (c).

**Amendment.** Done when the set of Landkreis Göttingen GUID row ids equals the kept list the wet run records, plus
rows with an id above the wet run's max org id. Print ids (`[... | .id]`), not a count. Lines 2-3 are unchanged.

### A11. Wording of the R1 country bar

The 234 arm keys the target on `register_jurisdiction(mention.country)` (canonical.rs, the `(None, None)` arm). The
group key does the same, because `normalise_identifier_with` maps the country first. As written, "every target has
identifier NULL in the mention's country" fails on an RE, GP or MQ mention that lands on an FR row.

Reword: target identifier NULL and `target.country = register_jurisdiction(mention.country)`.

### A12. Smaller items

- **Stale stamps.** `dissolve_condemned` stamps winner Tenders epoch-stale (`refold_tenders`), and it can skip an
  org whole. "No refold needed" therefore holds only for 434 refolds. R3 checks that `skipped` = 0, and that the
  stamped Tenders were re-projected by the normal project drain before the census re-run.
- **300 answer.** "75,548 distinct values over 75,555 rows" is 312's figure from 2026-08-30. R1 reports distinct
  values over rows for today's 81,323-row walk, so that "merge-inert" rests on a current number.

### Critic GETs (7 of 15, at least 2 s apart)

1. `/v1/organizations?name_prefix=Natürliche Person`: 4 rows, none GUID-keyed; DE identifier-NULL 3950487 (10).
2. `/v1/organizations?name_prefix=Freistaat Bayern`: an exact `Freistaat Bayern` DE identifier-NULL row (203
   mentions); GUID rows exist only under the longer `vertreten durch …` names.
3. `/v1/organizations?name_prefix=Stadt Neustadt`: `Stadt Neustadt` DE identifier-NULL 2371794 (18), beside rows
   for Aisch, Donau, Waldnaab, Rbge., Weinstraße, Coburg, Holstein, Kulm and Hessen; no exact-name GUID rows.
4. `/v1/organizations?country=DE&cursor=22759000`: 123 GUID rows. The pair and triple names are specific (Leibniz
   Universität, Medizinische Hochschule Hannover, Vergabekammer Rheinland ×6, Landkreis Göttingen ×4); no generic
   pair names.
5. `/v1/organizations/21987077`: Sprinkenhof GmbH, 1,282 mentions (stable; a KEEP pair with 10966, not a fresh
   partner).
6. `/v1/organizations/22021294`: Stadt Nürnberg Hochbauamt, 250 mentions (stable; a KEEP pair with 1085).
7. `/v1/organizations?country=DE&cursor=23050000`: 104 GUID rows, 20 names with 2 or more rows, all specific
   (Vergabekammer variants, Regierung von Niederbayern, Kreis Coesfeld Abteilung 30).
