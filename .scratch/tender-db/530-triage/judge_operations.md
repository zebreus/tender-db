# 530 unit 2 — operations judge (2026-10-19)

Lens: OPERATIONS FIRST. For each option I ask:
- how the gate builds it: a pure test of one notice, or a stateful lookup at mint time;
- how the standing rows are dissolved;
- how big the drain is, and whether it can be reversed;
- how it interacts with 456, 473 and 572;
- what it costs every week afterwards.

Inputs:
- issue 530, all of it;
- `guid_census_2655.json` (decoded body: breakdowns and all seven sample lists);
- `crates/store/src/guid_census.rs` (the definitions; signal code at :795-975);
- `crates/ingest/src/idgate.rs` (`condemns` :453, `uuid_v4` :479);
- `crates/ingest/src/project.rs`: `gated_identifier` :5314, the per-notice mention assembly in
  `NoticeState::mentions_and_void` :6030-6050, and the gate itself :11944-11957;
- `crates/store/src/canonical.rs`:
  - `repair_placeholder_orgs_batch` :41122 and `dissolve_condemned`;
  - `RecordedMention::publishes` :15578;
  - the resolver's `org_of` preload :15295 and the rebuild path's in-RAM `org_of` :21054;
  - `organizations_name_country` :17677;
  - the `organization removed` change row :41881;
- `crates/app/src/supervisor.rs`: the `RepairPlaceholderOrgs` arm :9797, `UNSTOPPABLE_KINDS` :5974 and
  `BACKFILL_BATCH` :196;
- issues 312, 440, 522, 434, 456, 473, 572, and 300 design :84-85 and §10.

I read `judge_precision.md` and `judge_coverage.md` after forming the lens below. I reuse their GET facts, cited as such.

I made 8 public GETs, listed at the end.

## Decision

**(a), pair-scoped, as a pure notice-local test at the gate, with a frozen keep-list and the existing name-safety
guard. One dissolve of the single-tender rows it reaches. (c) for everything else, with the cost written down.**
- **(d) is declined at the gate.** If it is ever reopened, the operational form it must take is stated below.
- **(b) is declined.** Its one clean cell becomes a new issue.

In one line: build only what can be decided from the notice alone, keep that decision reproducible bit-for-bit through
refold, 434 refresh and a from-archive rebuild, and leave nothing that needs a weekly job.

## How each option is built, operationally

### (a) within-notice twin: a pure test, if its exception list is frozen

- **Where it goes.** Every mention of a notice is assembled in one place, `NoticeState::mentions_and_void`, which maps
  `gated_identifier` over all mentions (project.rs ~6041). A group-by pass right after that map sees every ORG
  section of the notice and nothing else, so it is a pure function of the notice, like `condemns` is of one value.
  - It needs `store::org_name_norm`, the census's key and the 234 fallback key, so all three use one ruler.
  - It needs `idgate::uuid_v4` and a frozen value set.
  - No DB read, no store change, no resolver change.
  - The incremental fold, the 434 refresh and the from-archive rebuild all consume this projection, so they agree by
    construction.
- **As worded in the issue it is not a clean category.** 179 of the 11,437 twin rows span several Tenders, and 70
  of them are stable. These are pairs of stable per-role keys:
  - SBH 3866/3868;
  - GMH 22040636/37;
  - Stadt Nürnberg U-Bahnbauamt 2529 (338 mentions) and 2530 (328), both read today;
  - Universitätsklinikum Erlangen 21983972 (174) and 21983973 (72), read today. The second is below the stable bar
    but spans several Tenders.

  So the exemption has to reach below the 100-mention stable bar.
- **Two ways to exempt them:**
  - **Bound key** (the precision judge's (i): a value already keyed to a row keeps binding). This moves the decision
    into the store resolver (`org_of`), so ingest would have to flag twins and store would have to decide. It also
    depends on state:
    - In a from-archive rebuild, ids are re-minted from 1 and `org_of` starts empty. A pair that is twinned in every
      notice never mints, and one twinned only sometimes mints at its first solo sighting.
    - So the rebuild's org layer would differ from the live one, depending on order.
  - **A frozen keep-list (chosen).** It is value-keyed, so it survives renumbering. It is committed as data, like
    issue 513's frozen `nhs-ods.tsv` through `include_str!`, and is no more than 454 values. It keeps the rule pure
    and the rebuild identical. Its only cost is a stable pair born after the freeze: its twin mentions go to the
    name's 234 row, the identifier-less baseline. The bound on that is the 36 single-use twin rows first seen in 2026.
- **Notice-scoped (a) is rejected.** Its premise, that the notice proves the platform mints per section, is false
  by measurement:
  - 254 and 255 sit in twin notices, so notices mix stable and fresh keys;
  - it would need a keep-list of about 1,282 multi-tender rows plus 345 stable rows;
  - it would still refuse every stable key that a mixed notice carries before the freeze.

### (b) by publisher or era: nothing clean to build

- **The measured axes are mixed:**
  - doe+ted dashed-lower holds 40,388 single-use rows and 338 of the 345 stable rows;
  - ted dashed-lower holds 1,821 rows that span several Tenders (CH bidders: FR_Swisscom has 75 mentions over 53
    Tenders);
  - CH is per role (buyers mint per notice, bidders keep their keys);
  - era and profile are maturity-biased.
- **The publisher fields are not measured** (`esender_party` and `submission_url_host` are `not_measured`).
- **The one clean cell is a sampling accident, not a platform rule.** "ted other" is 3,511 FR rows, all single-use,
  raw 8-4-4-16. The evidence:
  - nibble-only 10,548 ≈ 3 × 3,511, the 3:1 ratio random hex gives;
  - the precision judge's specimen has a sequential prefix (`FD29FC2F…` beside `FD29FC01…`).

  A rule on it would refuse about 1/64 of that platform's keys and leave the rest (about 225k by the coverage judge's
  estimate) minting. It belongs in a new issue on the raw 8-4-4-16 shape across the hex class (the coverage judge's
  F3), measured there.

### (d) cross-notice: a stateful lookup at mint time. Declined for six operational reasons

1. **It cannot live at the gate.** The gate (`normalise_identifier`, `condemns`) sees one value. (d) needs the
   resolver's mint branch:
   - a `(name_norm, country)` seek on `organizations_name_country`;
   - a mention COUNT for each GUID row under that key. Vergabekammer Westfalen alone has 185 or more.

   That runs inside the daily fold's write path, which is the hottest code in the system.
2. **Two implementations.** It needs one in the batch resolver, which has no full preload (issue 57), and one in the
   rebuild's in-RAM path. They must agree, and nothing pins that they do.
3. **Its outcome depends on order, and the census cannot size it.**
   - Org ids are not mint order. Row 39 is a 2026 row: a from-archive rebuild re-minted from 1.
   - So the census's "earlier = lower id" is not what a mint-time rule would see. The 55,783 reached and the 11
     stable rows are id-order figures; a mint-order replay would give different ones.
   - The ≤2-mention count also changes while the TED and DÖE copies land in different batches.
4. **It consumes its own evidence.** The dissolve removes the ≤2 rows that trigger it. To keep firing it needs a
   marker: either a new table, or a scan of the 234 row's mentions for a v4 `raw_identifier`, and there is no index
   on `raw_identifier` (3233 has 37,696 mentions). The anchors still mint, so the coverage plan itself needs a
   **recurring dissolve**. That is weekly upkeep with writes, and the worst of both worlds.
5. **Every recurring dissolve has a consumer-facing cost.** A dissolved id answers **404 "no such organization"**
   (read today on 30914328, 522's dissolve), and the dissolve writes `organization removed` change rows
   (canonical.rs :41881).
   - Once is acceptable (440, 510, 511 and 522 did it).
   - Every week, it makes "added, then removed within a week" the normal life of about 1.8k orgs a month.
   - The same holds for a stock-side periodic fold.
6. **Bigger drain, less reversible state.**
   - The coverage plan dissolves 59,493–63,974 rows, about 130k mentions. That is hours on the single writer and
     needs a quiet window, against (a)'s tens of minutes.
   - Its refusals can be found afterwards (v4 raw on identifier-NULL rows), but its decisions cannot be reproduced,
     because they depended on DB state at mint.

### (c) decline: operationally free, and it leaves the live shape split

This is the price of the decision, and it is written down below. The only build is one docs sentence.

## Rule wording, as the gate applies it

**530-T (ingest, `NoticeState::mentions_and_void`, immediately after `gated_identifier` is applied to the notice's
mentions and before the `extracted` check; beside `idgate::condemns`, not inside it).**

Per notice; a TED copy and a DÖE copy are each their own notice:

1. **Candidates:** mentions whose gated identifier has kind `national` and whose value satisfies `idgate::uuid_v4`.
2. **Groups:** group the candidates by `store::org_name_norm(name)` of the mention's published name. That is census
   job 2655's twin key, so its counts are exact upper bounds. A mention with an empty name joins no group.
3. **Twin:** a group holding **2 or more distinct** identifier values.
4. **Refusal:** in a twin group, set `identifier = None` on every candidate whose value is **not in `KEEP_530`**.
   - `raw_identifier` and `scheme` stay as published.
   - **Exception:** if the group's name is not name-safe, the whole group binds as before. Not name-safe means
     `content_words(guard_name_words(name)) < 2` or `generic_body_name(…)`: the existing pure helpers in
     project.rs, extended only with words the R2 read convicts.
5. **Everything else is untouched.** A GUID in no twin group binds or mints as today:
   - the Südbayern three-notice shape and Bremen **bind**;
   - a stable key recurring across notices (the DB platform, 8378) **binds**;
   - an SBH pair, both values in KEEP, binds;
   - a KEEP value twinned with a fresh one: only the fresh one is refused.

A refused mention resolves through the issue-234 identifier-NULL path: the `(name_norm, country)` row with identifier
NULL, else a fresh provisional row. It never lands on an identified row (300 §10).

**`KEEP_530`** is frozen, value-keyed, in `crates/ingest/sdk/guid-keep-530.tsv` through `include_str!`, with columns
value, org id at freeze, mentions, tenders and reason. It holds:
- the identifiers of the uuid_v4 rows that sit in a twin group and span ≥ 2 distinct Tenders (179 at job 2655);
- ∪ the stable population: 254, 255, 8378 and every uuid_v4 row with ≥ 100 mentions (345).

That is no more than 454 values. The dry plan emits the list at the build rev, the build commits it, and no job
ever edits it. A test pins 254, 255, 8378, 3866, 3868, 2529, 2530, 21983972 and 21983973 inside it.

**Stock dissolve (unit 3).** It is an id-set selector over 440's `dissolve_condemned` machinery. The value-predicate
entry `repair_placeholder_orgs_batch(condemns: fn(...))` cannot express a twin. A row is dissolved if and only if:
- its identifier satisfies uuid_v4;
- its value is not in KEEP_530;
- at least one of its mentions sits in a twin group that step 4 refuses;
- its mentions span exactly 1 distinct Tender (the `tender_versions.caused_by_notice_id` basis);
- it has fewer than 100 mentions.

The last two are tripwires: any planned row that fails them aborts the run. Whole rows are dissolved, which is
correct because they are single-tender.

Operational requirements on that job:
- **Box its `Spec` arm** (CLAUDE.md; 440's arm is already boxed).
- **Make it stoppable.** `repair-placeholder-orgs` sits in `UNSTOPPABLE_KINDS` because it takes "seconds", and this
  run takes tens of minutes. Issue 543's pairing test will demand a stop checkpoint between windows.
- **Window by target key, not by org-id range.** Issue 542's in-window name-variant re-derivation reads ALL mentions
  of every org it rebinds in each window. Org-id windows would re-derive a large chamber row once per window it
  appears in.
- **Write a restore manifest** in the job report: each dissolved org's id, country, value, every
  (notice_id, section_id) and the target.
- **The wet run re-plans** rather than trusting the dry list. Its count must fall within the dry count plus rows
  minted since.

A refold alone moves nothing: `RecordedMention::publishes` (issue 434) compares name, country, raw and scheme, and the
gate leaves all four unchanged. So the dissolve is required. After it, the same comparison keeps the new binding
stable.

## Expected effect (census 2655)

**The gate (forward).** It prevents pair-twin mints outside KEEP. At the 2026 rate that is about 36 rows a year
(`one_tender_twin` by first year: 2023 10, 2024 7,459, 2025 3,753, 2026 36). The Göttingen shape has nearly stopped.
- The gate's job is **durability**: a from-archive rebuild or a 434 refresh cannot re-mint what the dissolve removed.
- Deploying it moves no standing mention (`publishes`), so the deploy itself causes no churn.

**The stock.** The dissolve removes **at most 11,258 single-use rows (15.2% of 74,023)**, about 22.5k mentions
(about 2 per row, the TED and DÖE copies; the dry plan gives the exact sum).
- By country: DE 11,237, FR 18, AT 1, other 2.
- By role: review_or_info 8,262, buyer 2,774, other 183, tenderer 39.
- Guard exclusions come off the top.
- **0 rows spanning several Tenders and 0 stable rows** are touched. All 179 twinned multi-tender rows are in KEEP.

**Drain.** At 440's observed rate (job 1615: 425 orgs in under a minute, including a full scan), it takes 30 minutes
at most. The planner is the census walk, which took 12 s in job 2655. About 10k Tenders get a `changed` row and
about 11k orgs get `organization removed`, all once.

**Verify.**
- Line 1: all 52 Göttingen GUID rows read exactly 2 mentions today (GET: 52 rows, 104 mentions). They are the
  Procedure-Buyer + Lot-AddInfo twin of each award, so the expectation is **52 → 0**, folded into **2294259**
  (DE, identifier NULL, 216 mentions; GET by the precision judge, still listed today). They never go to VAT row
  22183653 (649) nor to the country-NULL row 16393 (804). Any unplanned Göttingen row is recorded with its reason.
- Lines 2-3 hold by construction. 254 reads 24,219 and 255 reads 73,039 today. Neither is pair-twin, and both are
  in KEEP.

**Left under (c):** **62,765 single-use rows (84.8%)**:
- CH 27,837 (0 twin);
- DE about 30,803 (the Südbayern and Bremen no-twin shape, the Westfalen runs);
- FR 3,579.

The live growth is about **1.8k single-use rows a month** (17,533 first seen in 2026 by 10-19; 13,085 of them
cross-only, 36 twin). This decision does not stop it. The docs sentence owns it.

**`docs.rs` sentence** (the organization-identifier note cited at :880 in e2cbbb8): "A v4 UUID in `identifier`
(`identifier_kind: "national"`) is a submission platform's record key, not a register number. Some German and Swiss
platforms mint a fresh one per notice, so one body can appear as many organizations with one or two mentions each;
aggregate such rows by name (issue 530)."

## Risks

1. **Coverage, by decision.** 84.8% of the single-use class and nearly all of 2026's minting stay split. The reopen
   conditions are below.
2. **The frozen KEEP.** A stable pair born after the freeze loses the mentions from its twin notices to the name's
   234 row. Bound: 36 single-use twin rows in 2026.
3. **The TED and DÖE copies can disagree** (one copy twinned, the other not). That splits one notice pair between a
   GUID row and the 234 row. The dry plan counts the rows that have a mention outside any twin group.
4. **Homonyms in the fallback.** A `(name_norm, country)` key can hold different bodies. The guard and R2 bound this.
   Twin names are specific in the samples (Stadt Landshut, Regierung von Niederbayern, Hamburger Energienetze).
5. **New code in a write path that 440 and 522 share.** It is a refactor of `repair_placeholder_orgs_batch`'s
   transaction tail into a helper that takes an id set. The existing `placeholder_dissolve.rs` tests must stay
   green.
6. **Identity becomes contextual.** A tool that rebuilds a mention's identity from its stored row alone
   (`gated_identifier`: the 513 nhs-body canary, 511's guard 3) sees a GUID where the fold refused one.
   - None of them reads DE GUID mentions today.
   - The gate's doc comment must say so.
   - Any future reader re-derives the twin from the notice's stored mentions (same notice, same name, two or more
     raw v4 values), as the census does.
7. **Consumers.** Dissolved ids answer 404, once, for about 11k orgs. That is the 440 and 510 precedent.

## Interactions

- **456** (an identifier binds whatever the name says). A refused GUID binds by name, so 530 only removes 456-shaped
  exposure. Run 456's measurement census after this drain, or exclude v4 rows; otherwise its counts move under it.
- **473** (DB file numbers, class 5; the Köln residue). Disjoint from uuid_v4. The id-set dissolve built here is the
  infrastructure 473 can reuse. Serialize the two drains on the single writer.
- **572** (an agent's identifier binds its principals). The dissolve is single-tender only, so agent rows, which span
  several Tenders by nature, are untouched; KUBUS 11107 and EKK plus 21986305 are stable, twinned and in KEEP. 572's
  stock stays intact for its own measurement.
- **Scheduling.** Put the wet run in a queue gap: not on the drain-free mornings (560), and after 551's flip. The gate
  can deploy before the wet run, because between them it only refuses new twin notices, consistently.

## Weekly upkeep

- **(a) with a frozen KEEP needs none.** The rule is pure, and the dissolve is one-time and converges.
- **Optional monitor:** re-run the 12-second `guid-census` monthly.
  - `one_tender_twin` should stay at about the guard's exclusions.
  - `twin.stable_rows` stays at 70 (the KEEP pairs are still twins).
  - The stable population stays at 345 rows and at least 311,552 mentions.
  - The 2026 cross-only growth is the figure to watch for a reopen.
- **By contrast, mint-time (d) as the coverage judge words it** needs:
  - a marker store;
  - a periodic dissolve of the anchors;
  - a weekly read for stable keys it refused silently (a v4 raw recurring across 2 or more Tenders on an
    identifier-NULL row).

## Reads that must pass before the build ships

- **R0, gate (ops/check.sh, tests in `idgate.rs` / `project.rs` plus one resolver test):**
  - a Göttingen-shaped twin refuses both values, and the resolver lands them on the DE identifier-NULL row, never on
    the VAT row;
  - the Südbayern three-notice shape (no twin) binds;
  - a DB-platform GUID recurring across notices binds;
  - an SBH-shaped KEEP pair binds;
  - a KEEP value with a fresh one refuses only the fresh one;
  - a generic-name twin binds;
  - a TED copy and a DÖE copy are each decided alone.
- **R1, dry plan (in-app, the same rev as the wet run, writes nothing). Exact bars; any miss means stop:**
  - planned ≤ 11,258 plus the twin rows minted above id 31,970,152;
  - **0** planned rows with ≥ 2 Tenders, ≥ 100 mentions, or a value in KEEP;
  - not planned: 254, 255, 8378, 3866, 3868, 2529, 2530, 21983972, 21983973, 21987063 and 30530641;
  - 22759261 and 22759262 planned, with target 2294259, and the planned share of the 52 Göttingen rows reported (52
    expected);
  - every target has identifier NULL, in the mention's country;
  - **KEEP self-check:** the walk's (twin ∧ ≥ 2 Tenders) ∪ stable set is contained in KEEP (no new unprotected pair
    since the freeze);
  - reported: windows by target key, the largest target's mentions, the dry runtime, guard exclusions by reason, and
    the rows with a mention outside any twin group.
- **R2, sample:** the 30 targets receiving the most planned mentions, plus 30 random planned rows (review 15, buyer
  10, other or tenderer 5). Score each R, N or W. **Bar: 0 W in 60, at most 3 N**; 0 in 60 puts the 95% upper
  bound near 5%. Any W extends the guard, re-runs R1, and needs a fresh 30 at 0 W.
- **R3, after the wet run:**
  - the Verify block: line 1 reads 0 or the residue recorded with its reason; 254 ≥ 24,219; 255 ≥ 73,039;
  - a guid-census re-run, against the expectations above;
  - the manifest's mention count equals the wet run's mentions moved, which makes it a complete restore path.

## Reversibility

- **The gate:** revert the commit and new notices bind again. Refused standing mentions stay where they are
  (`publishes`).
- **The stock:** every refused mention keeps `raw_identifier`, and the manifest lists value → (notice, section). The
  restore is an `apply-rehoming` cohort per value onto a re-minted (country, national, value) row. It is chunked,
  because each cohort is one commit. It is mechanical, not free.
- **Because the twin decision is a pure function of stored mentions,** the refused set can be recomputed exactly at
  any time, even without the manifest. A mint-time (d) cannot give that.

## 300 design:84-85 ("merge-inert singletons, not junk to dissolve"), answered

- **"Merge-inert": upheld.** `canonical_key` returns None for a v4 UUID (312 step 1a), and nothing here makes a GUID
  a merge key. `condemns` and `uuid_v4` are unchanged.
- **"Singletons": now measured, and it is the defect, not inertness.** 91.0% of the rows span one distinct Tender. A
  per-section key never merges, so its split never heals. 312's 93% figure counted the DÖE and TED copies (59.0% span
  several notices, 9.0% several Tenders).
- **"Not junk to dissolve": upheld for the class.** 7,300 rows span several Tenders, and 345 stable rows (311,552
  mentions, 96.2% spanning several Tenders) do real linking. There is no class condemn and no class dissolve.
- **Amended for a proven sub-class.** A row is dissolved when its own notice publishes one name under two values,
  the value is not on the frozen KEEP list, the name is safe as a fallback key, and the row never left one Tender.
  - It is dissolved as a per-notice key whose 234 row is the entity, not as junk.
  - The value survives on `raw_identifier`.
  - The rest of the v4 class stays a merge-inert singleton by decision (c), with its cost stated above.
  - The wider 273,982-row hex-hash population is not decided here.

## Reopen conditions for (d) or a successor

Reopen only in a form that is pure at the gate, or convergent at the stock, and measured first:
- **A mint-order replay of (d)** (an in-app dry job replaying GUID mints in notice-ingest order, not org-id order),
  showing 0 stable rows refused under an exemption, plus a fallback read at 0 W in 60.
- **Or a probation design:** a new GUID outside KEEP does not mint until its value recurs in a second Tender, and is
  then promoted from the kept `raw_identifier`. Measure first the promotion lag and the stable keys' first-tender
  share.

Either reopening must account for the 404 and change-feed cost of anything that dissolves on a schedule.

## GETs (8 of 15, at least 2 s apart)

1. `/v1/organizations?name_prefix=Landkreis Göttingen&limit=200`: 52 GUID rows, all 2 mentions (104). Non-GUID
   rows: 14107 `11389` (97), 16393 country NULL (804), 2294259 DE NULL (216), 22183653 `DE308703297` (649),
   22426277 `DE84ZZZ00000042204` (2).
2. `/v1/organizations/254`: `FB197F94…`, 24,219.
3. `/v1/organizations/255`: `0A9EA480…`, 73,039.
4. `/v1/organizations/2529`: Stadt Nürnberg, U-Bahnbauamt, `5359C289…`, 338.
5. `/v1/organizations/2530`: the same name, `5EC26812…`, 328 (a stable pair).
6. `/v1/organizations/21983972`: Universitätsklinikum Erlangen, `FD9B6FCE…`, 174.
7. `/v1/organizations/21983973`: the same name, `9CC94E71…`, 72. It spans several Tenders and is below the stable
   bar, so KEEP must reach below 100 mentions.
8. `/v1/organizations/30914328` (dissolved by 522): HTTP 404 `no such organization`. A dissolved id leaves no
   redirect.
