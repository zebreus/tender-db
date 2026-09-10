# 364 — the legacy OJS closure is an unbounded transitive closure over unguarded edges: 2,983 versions and 127 buyers in one Tender

Status: ready-for-agent — UNITS 1-2 DONE 2026-09-07, GAUGE (re-cut unit 4) DONE 2026-09-10 `c0c2581` and the REPRESENTATIVE RULE (re-cut unit 3, the phantom half) DONE 2026-09-10 `2c05c8d`, both gates green and DEPLOYED (rev `7e023e4`). **THE GAUGE'S FIRST RUN REFUTED ITS OWN >=50 CALIBRATION — see the last section; the top of the listing is Slovenian JOINT PROCUREMENT, not welds, and the weld this issue was filed about does not make the top 40 at all.** Corrected in `831b8f9`; the next unit is the widest-single-version discriminator: the kind gate is built and gated (`7b7d513`, 914 passed) and LANDS INERT by design — see "Unit 2, built" for what that means for the repair. Units 3 (guards + representative), 4 (the plausibility gauge) and 5 (the legacy RE-PARSE, not a re-projection) remain. **UNIT 3's CALIBRATION IS CORRECTED 2026-09-10 and the class is now MEASURED — see the two sections at the end. The recorded `role='Procedure-Buyer'` predicate is blind to the legacy era, where this issue's own 127-buyer weld lives (tender 2816628 has 2,983 `buyer` rows and ZERO `Procedure-Buyer`); the corpus carries two buyer vocabularies and the predicate must be `role IN ('buyer','Procedure-Buyer')`. Measured corpus-wide: 102,840 tenders with ≥3 distinct buyers, 1,326 with ≥50 — six times the ≥200-version set `longest_chain` can see.** Was: ready-for-agent — UNIT 1 DECIDED 2026-09-07 (owner)
Kind: defect (identity / grouping) — correctness, the CONTEXT.md:112-113 invariant
Relates to: 92 (records chain 3,282 only as a fold-performance cost, not as a correctness
signal), ADR-0011 (the eForms edge's three guards, which this edge has none of), ADR-0003
(no heuristic merging), 34 (the same "every notice sharing it collapses into one Tender"
failure for a folder id), and the sibling key defect (`procedure-key-accepted-unchecked`)

## Observed

**Tender 2816628** — `SELECT id, source, procedure_key, current_title, current_seq FROM tenders WHERE id=2816628` → `ted`, `ojs:2001-185105`, "Heizung- und Wasserinstallation", current_seq **2983**. `SELECT tender_id, MIN(published_at), MAX(published_at), COUNT(*), COUNT(DISTINCT publication_id) FROM tender_versions WHERE tender_id=2816628` → 2,983 versions / 2,983 distinct publication ids, 1297814400 (2011-02-16) → 1541721600 (2018-11-09). `SELECT role, COUNT(DISTINCT organization_id) FROM tender_version_parties WHERE tender_id=2816628 GROUP BY role` → buyer = **127** distinct organizations, winner = 107; buyer countries DE, PL, FR mixed. The notice payloads confirm the three named specimens: notice 4405514 (`052696-2011`, seq 1) `TED-ORGANISATION` = "Vermögen und Bau Baden-Württemberg, Universitätsbauamt Heidelberg"; notice 12339370 (`185105-2011`, seq 4) is PL with `TED-ACTIVITY_OF_CONTRACTING_ENTITY = EXPLORATION_EXTRACTION_COAL_OTHER_SOLID_FUEL` and a 112-lot title "1)Dostawa drewna kopalnianego …"; notice 20099441 (`493180-2018`, seq 2983) `TED-OFFICIALNAME` = "Krankenhaus Reinbek St.-Adolf Stift GmbH"; notice 12420678 (`266302-2011`) is French (SERS / Tribunal de grande instance de Strasbourg). `curl /v1/tenders/2816628` serves one German-titled record carrying a EUR `estimated_value` plus ~100 PLN coal-mine lot values.

**Tender 4812955** — procedure_key `ojs:2012-239732`, current_seq **2119**, 1343347200 (2012-07-27) → 1706659200 (2024-01-31); buyer = **182** distinct organizations, winner = 257; `SELECT COUNT(DISTINCT value) FROM tender_version_texts WHERE tender_id=4812955 AND field='title'` → **1,711** (1,445 at procedure level). Buyers: DB Netz AG, DB ProjektBau, DB Station & Service, DB Regio, DB Energie and the TEI-N/TEI-M/TEI-W/TEI-SO/TEI3 units — plus "Alliance élevage Loir-et-Loire" (FR), riding in on one stray edge.

**Blast radius** — one full-table aggregate: total **7,929,584** tenders; **43,088** with ≥10 versions; **2,574** ≥50; **212** ≥200; **15** ≥1000; max 3,282.

**The hub, measured**: notices 17283790 / 17284506 / 17284981 / 17285150 / 17285196 / 17285468 each carry their OWN distinct predecessor in `TED-REF_NOTICE.NO_DOC_OJS` but ALSO cite `TED-NOTICE_NUMBER_OJ = "2012/S 123-203577"`, so all six union onto node (2012, 203577) and become one component.

**The name is a typo's, and the node does not exist**: notice 12426474 (publication `273139-2011`) carries `TED-NOTICE_NUMBER_OJ = "2001/S 112-185105"` while its eleven sibling notices in the same component write `"2011/S 112-185105"`. `SELECT id FROM notices WHERE publication_id IN ('185105-2001','00185105-2001')` returns **0 rows** — the corpus holds no such publication — yet node (2001, 185105) is created, is the component's MIN, and therefore names all 2,983 versions.

## Why, exactly

1. **Every OJS ref is an edge, regardless of what field it came from.** The plan builder collects every parsed id with `scheme == "ojs" && is_ref` as a chain edge, with no regard for its origin — `crates/ingest/src/project.rs:3313-3325` ("The chain edges: every `is_ref` OJS-scheme id the notice carries").
2. **`NOTICE_NUMBER_OJ` is mapped as a ref globally.** `crates/ingest/src/r209/rules.rs:223` maps it to `Rule::Id(IdKind::Ref)`, and the parent-context table in the same file — `crates/ingest/src/r209/rules.rs:145-148`, which exists precisely to disambiguate `NO_DOC_OJS` ("the notice's own OJS number everywhere except inside REF_NOTICE") — has no entry for it. So a corrected notice's number (F14), a modified award's (F20), and a prior-information / qualification-system number that dozens of separate procurements were called under all become the same undirected same-procedure edge.
3. **The closure has no guard and no cap.** `insert_plan_tx` writes the edges symmetrically (`crates/store/src/canonical.rs:7051-7065`) and `build_plan_groups` unions both endpoints in a `MinUnionFind` with no existence check, no direction check, no buyer check and no size cap — `crates/store/src/canonical.rs:7151-7160` — then labels the component from the MIN endpoint as `ojs:{year}-{number:06}` (`crates/store/src/canonical.rs:7169-7171`), which is how a phantom endpoint gets to name and weld a component.
4. **The contrast is in the same file.** The eForms ADR-0011 edge joins through `PREV_EDGE_JOIN_SQL` (`crates/store/src/canonical.rs:1060-1070`), which requires the target to EXIST (`JOIN notices n ON n.source = e.a_source AND n.publication_id = e.b_publication_id`), the same source, and `b.published_at < a.published_at`. The legacy closure has none of the three. ADR-0011 records the omission as intentional ("the legacy OJS closure … deliberately admits not-yet-ingested edge targets so identity is stable as backfill deepens") — but with no counterweight, one mistyped digit permanently renames and can permanently weld.
5. **Nobody could see it.** CONTEXT.md:112-113 promises "transitive edges — a missed link splits a Tender, never wrongly merges", and the only standing component-size signal is `longest_chain` = `SELECT MAX(current_seq) FROM tenders` (`crates/ingest/src/data_quality.rs:715`), surfaced as `tender_db_dq_longest_chain` (`crates/app/src/v1/metrics.rs:248-254`) and framed by issue 92 as a fold-performance tripwire firing at ≥4,000. At 3,282 the gauge reads green while 212 tenders each fuse hundreds of unrelated procurements. No check anywhere counts distinct buyers, distinct titles or year span per tender.
6. **A weld is irreversible in served data**: the representative is `MIN(encoded ojs key)`, so growth only ever absorbs more.

## Units

1. **Decide the edge-admission rule** (owner's call, first unit): give the legacy edge the three guards its eForms sibling has — target-exists, same-source, strictly-earlier. Target-exists conflicts with ADR-0011's deliberate allowance, so either retire that allowance, or keep it and require a phantom endpoint to be well-shaped AND refuse it as component representative. Record the decision and its reasoning here.
2. **Context-gate `NOTICE_NUMBER_OJ`** through the mechanism `crates/ingest/src/r209/rules.rs:145-148` already provides for `NO_DOC_OJS`: an edge only where it means a predecessor; the notice's own number elsewhere. Pin with a fixture of the six-notice hub (17283790 …) — it must yield six components, not one.
3. **Measure the invariant.** A component-plausibility gauge in the weekly DQ report: per tender, distinct buyer organizations, distinct procedure titles, year span, with the top N listed. This is the detector for this issue *and* for `procedure-key-accepted-unchecked`; build it once. Issue 92's `longest_chain` stays what it is.
   **Calibrated 2026-09-08 by issue 369's unit-1 census:** count buyers at `role='Procedure-Buyer'` only (counting every role inflates by bidders and review bodies — 82806 reads 8 orgs and 2 buyers), and set the alarm at **≥3 distinct buyers**: two buyers is the org layer's duplication noise floor, measured twice (82803 = "Stadt Osnabrück - FD Öffentliche Aufträge" + "… - Fachdienst Öffentliche Aufträge"; 82806 = the same Rostock company as orgs 22771062 and 30900729). Distinct *titles* is a weak signal on its own for the same reason — a correct 4-version tender carries 2 (DE+EN) and 82803 carries 7.
4. **Repair**: with the guards in place a re-projection re-derives components; re-measure the 212 ≥200-version tenders before/after and record. Expect a large change-feed burst (the 351 fold's shape).

## Done when

- the six-notice hub fixture splits into six components and a phantom-target edge cannot name a component;
- the plausibility gauge is in the weekly report, and 2816628 / 4812955 are either named in it or gone;
- the ≥200-version count is re-measured after re-projection and written here.

*One issue because:* the 2,983-version Heidelberg/coal-mine/Reinbek tender, the 2,119-version DB Netz tender, the phantom `ojs:2001-185105` key and the whole ≥10/≥50/≥200/≥1000 distribution are one edge rule feeding one unguarded union-find — the distribution is that mechanism's blast radius, and the missing gauge is why it stood.

## Decision (2026-09-07, owner): gate on the reference's declared KIND, which the payload already states

I read the raw R2.0.8 payloads of the welding notices out of the archive rather than reasoning
from the rules table, and the XML answers the question the code never asks. Every one of these
citations sits inside a `PREVIOUS_PUBLICATION_*` block whose preceding element DECLARES what the
reference is:

```
notice 12426474 (273139-2011, the phantom's source) and 17283790 (054353-2013):
  <PREVIOUS_PUBLICATION_NOTICE_F5>
    <PREVIOUS_NOTICE_BUYER_PROFILE_F5 CHOICE="PERIODIC_INDICATIVE_NOTICE"/>
    <NOTICE_NUMBER_OJ>2012/S 123-203577</NOTICE_NUMBER_OJ>

notice 17284506 (055142-2013), the SECOND citation in the same notice:
  <CNT_NOTICE_INFORMATION>
    <CNT_NOTICE_INFORMATION_S CHOICE="CONTRACT_NOTICE"/>
    <NOTICE_NUMBER_OJ>2012/S 206-339288</NOTICE_NUMBER_OJ>
```

So the two citations mean opposite things and the corpus says so:

- `CHOICE="CONTRACT_NOTICE"` — the award's own contract notice. **Same procedure.** A real edge.
- `CHOICE="PERIODIC_INDICATIVE_NOTICE"` — the periodic indicative notice / buyer profile the
  procurement was called under. **Many unrelated procurements cite one PIN.** Not an edge at all.

That is the welder, and it explains the shape of the damage better than the missing guards do:
the six-notice hub welds because six separate procurements cite one PIN, and the DB Netz tender
welds because twelve years of rail projects were called under shared indicative publications.
It also explains the phantom: the mistyped `2001/S 112-185105` is a PIN citation too, so under a
kind-aware rule that edge is never created, node (2001, 185105) never exists, and nothing
renames the component. One rule removes both symptoms.

**Decided:** the identity layer keys on a reference's KIND, not on the fact that a reference
exists. A `PREVIOUS_PUBLICATION` citation is a chain edge only where the payload declares a
same-procedure predecessor (`CONTRACT_NOTICE` and its siblings for prior contract/award
publications); a PIN, buyer-profile or qualification-system citation is recorded as notice-layer
detail and never joins components.

Three consequences for the plan:

1. **The parser must stop discarding the kind.** `Rule::Id(IdKind::Ref)` collapses every
   reference to one boolean (`is_ref`), and `Ident::read` then treats all of them alike
   (`crates/ingest/src/project.rs:3313-3325`). The kind lives in a `CHOICE` attribute on a
   sibling element, so the fix is a parse-time reading, not a post-hoc filter — the same shape as
   the `CONTEXT` override that already disambiguates `NO_DOC_OJS`
   (`crates/ingest/src/r209/rules.rs:145-148`), one level up: parent element AND its declared
   choice.
2. **The three guards are still worth having, and are now cheap.** With PIN citations gone, a
   surviving edge should also require same-source and strictly-earlier where the target exists.
   ADR-0011's deliberate allowance for not-yet-ingested targets is KEPT — identity stays stable
   as backfill deepens — but a phantom endpoint may no longer NAME a component: the
   representative is the earliest EXISTING notice. A phantom that later arrives and takes the
   representative role is an ADR-0003 absorption, which the pipeline already handles.
3. **The invariant gets a measurement.** CONTEXT.md:112-113 promises a missed link splits a
   Tender and never wrongly merges; nothing checked it, which is why 212 tenders each fusing
   hundreds of procurements read green for years. The component-plausibility gauge (distinct
   buyers, distinct titles, year span per tender) is the detector, and it is what will show
   whether the kind gate actually cured this class.

## Units (re-cut after the decision)

1. ~~Decide the edge-admission rule~~ — DONE, above.
2. **Read the kind at parse time.** Extend the r209/r208 rules so a `PREVIOUS_PUBLICATION`
   reference carries its declared choice, and admit only same-procedure kinds as chain edges.
   Pin with a fixture of the six-notice hub (17283790, 17284506, 17284981, 17285150, 17285196,
   17285468): six components, not one.

   **The census is done** (2026-09-07, four archive days of February 2013: 6,327 notices,
   3,037 citations):

   | declared kind | citations | share | same procedure? |
   |---|---|---|---|
   | `CONTRACT_NOTICE` | 2,214 | 72.9% | **yes** — the award's own contract notice |
   | `PRIOR_INFORMATION_NOTICE` | 481 | 15.8% | **no** — one PIN is cited by many procurements |
   | (no `CHOICE` within 260 bytes) | 238 | 7.8% | **unresolved — unit 2 must classify these** |
   | `NOTICE_BUYER_PROFILE` | 53 | 1.7% | no |
   | `PERIODIC_INDICATIVE_NOTICE` | 27 | 0.9% | no |
   | `SIMPLIFIED_CONTRACT_NOTICE_DPS` | 13 | 0.4% | **judgement** — a DPS round; see below |
   | `NOTICE_QUALIFICATION_SYSTEM` | 11 | 0.4% | no |

   So **~19% of legacy citations are the welding kind** — a PIN, buyer profile, periodic
   indicative or qualification-system publication that many unrelated procurements cite. That
   share, not the phantom node, is the scale behind 43,088 tenders at ≥10 versions.

   **Two sub-decisions, taken 2026-09-07 (owner), so unit 2 implements rather than re-litigates:**

   - **`SIMPLIFIED_CONTRACT_NOTICE_DPS` is NOT an edge.** A dynamic purchasing system is one
     system, not one procurement: each call-off has its own award and is its own procedure under
     CONTEXT.md's definition of a Tender, and the DPS establishment publication is a separate
     procedure again. This is the "some of these are legitimate DPS rounds" case the reviewer
     raised, and the answer is that a DPS round is legitimately its OWN tender.
   - **A citation whose kind is not declared is NOT an edge, and is counted** — defaulting to
     "edge" is the mistake this issue is about. But the undeclared 7.8% must be CLASSIFIED
     before they are refused wholesale: if a large share are corrigendum-to-original blocks
     (which ARE same-procedure by another spelling), refusing them would needlessly split
     Tenders. Unit 2 identifies the element shapes in real payloads, admits the ones that
     demonstrably declare a same-procedure predecessor, and refuses-and-counts the rest.

   Two things unit 2 must settle rather than assume. The **238 citations with no `CHOICE`
   nearby** are a different element shape (the F14/F20 corrigendum blocks are the likely
   candidates) and must be classified, not default-admitted — defaulting to "edge" is exactly
   the mistake this issue is about, so the default is now "not an edge, and counted". And
   **`SIMPLIFIED_CONTRACT_NOTICE_DPS`**: a dynamic purchasing system's simplified notices all
   cite the DPS's own publication, so they are one system but not one procurement — this is the
   "some of these are legitimate DPS rounds" case the reviewer flagged, and it wants its own
   decision with the numbers in front of it.
3. **Guards + representative rule** as in consequence 2, with ADR-0011 amended to record that a
   phantom may link but may not name.
4. **The component-plausibility gauge** in the weekly DQ report (also the detector for issue
   369), listing the worst N.
5. **Repair**: re-project so components re-derive; re-measure the ≥10/≥50/≥200/≥1000 distribution
   and the two named tenders before and after, and write both here. Expect a large change-feed
   burst (the issue-351 fold's shape).

## Unit 2, built (2026-09-07, `7b7d513`)

The walker reads the sibling `CHOICE` and records the declared kind beside each citation, so the
identity layer can ask what a reference IS instead of only that one exists. The admit set is an
ALLOW-list, so a kind nobody has classified never welds anything:

- **Admitted as a chain edge**: `CONTRACT_NOTICE`; `ORIGINAL_NOTICE` — the F14 corrigendum's
  `COMPLEMENTARY_INFO` original, fixture-verified (its value equals the notice's own
  `REF_NOTICE/NO_DOC_OJS`); `THIS_PROCEDURE` — R2.0.9's procedure-level slot, IV.2.1 "Previous
  publication concerning this procedure", and the F20's original award.
- **Refused, recorded as notice detail, counted**: the four shared-publication kinds, the DPS
  simplified notice, an undeclared slot, and any unrecognised spelling.
- **Not gated**: `REF_NOTICE/NO_DOC_OJS`, which declares no kind and is the reference TED itself
  curates per procedure — in all three fixtures carrying both, it names the same-procedure
  citation and never the PIN. That is also why refusing a form-level citation rarely orphans a
  notice.

**The undeclared 7.8% resolved into three shapes**, as the sub-decision required: the forms'
IV.3.2 "Other previous publications" slot (three real instances; no kind anywhere in the block
— refused and counted, and almost certainly the bulk of the 238), the F14 corrigendum block
(no `CHOICE`, but its POSITION declares it, fixture-proven — admitted, so a real share of
undeclared citations were rescued rather than lost), and R2.0.9's procedure slot. Two shapes could
not be settled from committed bytes and say so in the code: the R2.0.9 procedure slot is admitted
on the repo's own measured research (it agrees with the coded reference in 709 of 711 files of the
2019 package) because refusing it would have split the whole era from its contract notices, and
the defence form's slot is classified by block name.

**It lands inert, and that is the important operational fact.** The kind lives in the PARSED
layer, so a citation with no recorded kind keeps its pre-364 grouping. Nothing regroups on
deploy; no flag day; `PROJECTION_EPOCH` unchanged, because re-folding a stored chain produces
identical output. The consequence for the repair: **unit 5 is a legacy-era RE-PARSE followed by a
re-projection, not a re-projection alone**, and the cure arrives era by era as that re-parse
deepens. A per-kind tally rides the durable project job row, so the effect is observable while it
happens rather than asserted at the end.

## Unit 3's calibration is WRONG for this issue's own corpus — the corpus has two buyer roles (2026-09-10)

Unit 3 above records, from issue 369's unit-1 census: *"count buyers at `role='Procedure-Buyer'` only
(counting every role inflates by bidders and review bodies …)"*. The exclusion reasoning is right.
**The role name is not, and following it would have shipped a detector blind to exactly the era this
issue is about.**

`tender_version_parties` carries **two** buyer vocabularies:

| role | where |
| --- | --- |
| `Procedure-Buyer` | eForms era |
| `buyer` | legacy (r208/r209) era |

And this issue's flagship weld is legacy. **Tender 2816628** — 2,983 versions, the 127 buyers named at
the top of this issue:

| role | rows | distinct orgs |
| --- | --- | --- |
| AWARD_AND_CONTRACT_VALUE | 15,287 | 1,986 |
| **`buyer`** | **2,983** | **127** |
| winner | 908 | 107 |
| LODGING_INFORMATION_FOR_SERVICE | 2,983 | 103 |
| APPEAL_PROCEDURE_BODY_RESPONSIBLE | 2,983 | 100 |

**Zero `Procedure-Buyer` rows.** The gauge as calibrated would have reported *0 distinct buyers* for
the tender the issue was filed about, and read green.

**How the calibration went wrong, since that is the transferable part:** it was derived from issue
369's census, which examined tenders 82803 and 82806 — both eForms. The exclusion rule ("not Tenderer,
not ReviewOrg") generalises correctly from that sample; the role NAME does not, because the sample
contained only one of the corpus's two eras. **A vocabulary calibrated on one era is a rule about that
era**, and this corpus has never had one vocabulary.

Note also what the legacy roles look like: `AWARD_AND_CONTRACT_VALUE`,
`LODGING_INFORMATION_FOR_SERVICE`, `APPEAL_PROCEDURE_BODY_RESPONSIBLE` — raw TED field names, i.e.
issue 368's unmapped-vocabulary subject showing up in the party layer. `buyer` and `winner` are the
two that were mapped. So the legacy party roles are mostly unmapped, and any rule that names roles
must say which era's names it means.

**Corrected predicate: `role IN ('buyer','Procedure-Buyer')`.**

## The measurement unit 3 was missing (corpus-wide, both roles, 2026-09-10)

Counted across ALL versions rather than the head only — a weld shows along the version chain, and it
avoids a per-row join to `current_seq` that put a 500k-id window over the `/v1/sql` 10 s cap (narrowed
to 100k windows; the over-cap query was NOT retried, per `docs/agents/prod-box-reads.md`).

| distinct buyer orgs per tender | tenders |
| --- | --- |
| **≥3** | **102,840** |
| ≥5 | 32,497 |
| ≥10 | 13,297 |
| **≥50** | **1,326** |

**This is a broader signal than the version count, which is the point of unit 3.** The issue's own
blast radius reads 43,088 tenders at ≥10 versions, 2,574 at ≥50, 212 at ≥200 — but **1,326 tenders
carry 50 or more distinct buyers**, six times the ≥200-version set. `longest_chain` cannot see them.

**What the numbers do NOT say, stated so the gauge is not over-sold:**

- **≥3 buyers is an upper bound on welds, not a count of them.** Joint procurement is real and legal,
  and the org layer's own duplication (issues 329/351) can render one authority as two or three rows.
  The issue's ≥3 alarm was calibrated against that noise floor on eForms samples; against 102,840
  corpus-wide it will need a listing to be actionable, not just a count.
- **≥50 is where the reading gets safe.** No joint procurement has fifty buyers, and org duplication
  does not multiply by fifty. That is 1,326 tenders that are almost certainly welded, and it is the
  number to open the listing on.

**Not built this firing, deliberately.** The measurement and the vocabulary correction are what unit 3
needed before it could be built correctly; building it first would have hard-coded the wrong role.

## The gauge, built (2026-09-10, `c0c2581`)

Section 12 of the weekly DQ report, `weld_candidates` + `weld_bands` in
`crates/ingest/src/data_quality.rs`. Registered whole-corpus beside the sentinel sweeps, since the
`HAVING` is per tender and the top-N ordering could not be merged across windows without keeping
every window's tail.

- **Predicate** `p.role IN ('buyer', 'Procedure-Buyer')` over `tender_version_parties`, all versions,
  `COUNT(DISTINCT organization_id) >= 3`. The both-roles requirement has its own test, negative-checked
  by dropping `'buyer'` and watching it fail.
- **Bands above the listing** at 3/5/10/50, uncapped, so the 40-row listing cap can never be read as
  the population's size. A full listing says so on its own line.
- **UNMEASURED renders as UNMEASURED, not 0.** For a detector those two claims are opposite, and the
  test holds that open.
- The render carries the calibration caveats from the section above, so a reader of the report gets
  the upper-bound reading without having to find this issue.

**Not the repair.** The gauge measures; unit 3's guards and unit 5's re-projection are what would move
the numbers. The bands are the before-picture those units will be measured against.

**Remaining:** unit 3 (guards + representative rule, ADR-0011 amendment), unit 5 (legacy re-parse and
re-projection), and unit 2's DPS-round decision.

## Unit 3, built (2026-09-10): a phantom may link but may not name

`build_plan_groups` labelled each legacy component with its union-find root, and the closure is
union-TO-MIN, so the label was the minimum OJS number over **all** nodes — including the
not-yet-ingested edge targets ADR-0011 deliberately admits. One mistyped digit in one citation could
therefore hand a Tender an identity no notice in it ever published, and change it again if that number
was later ingested under other circumstances.

The label is now the minimum over the nodes that **exist** (`named_by`, built from every legacy
notice's own `ojs_self`). Three things are deliberately unchanged:

- **The allowance itself.** A phantom endpoint still joins the component, so identity still survives a
  deepening backfill — that was ADR-0011's reason and it is still good.
- **Membership.** This decides what a component is *called*, never who is in it. No Tender gains or
  loses a notice from this commit.
- **The ordinary case.** A component whose minimum is a real notice keeps the name it had, and a second
  test holds that open — without it, a rule that always took the second-lowest node would pass.

Negative-checked: reverting the pick to the raw root makes the phantom test fail. A per-run log line
counts the components named by a non-root, including the zero, so a rule that stops firing is
distinguishable from one that finds nothing.

**Same-source and strictly-earlier are NOT built here.** Consequence 2 asked for all three guards; this
commit is the representative half. The other two need the edge's target resolved at plan time, which is
the shape unit 2's kind gate already established but for a different question — worth its own unit
rather than being smuggled in beside a labelling change.

**Inert on standing rows, by construction.** Only a projection that re-groups a component relabels it.
The daily incremental relabels a legacy component only when it touches one (and then completely, since
a legacy delta expands to its whole OJS component before grouping); everything else waits for unit 5's
rebuild. So expect the ≥50-buyer band to move in steps, not at once.

## Pending on the box (2026-09-10 ~14:00 CEST)

- **Deployed:** `1fd72fa` — the gauge (section 12). The representative rule (`2c05c8d`, tightened by
  `353431a`) and the timing correction (`bbed912`) are pushed but **not deployed**: the weekly
  `data-quality` job (1003) started at ~13:05 and runs ~4 h (32 windows at ~460 s each, historically
  416–493 s, so this run is normal). A deploy restarts the service and re-runs the job from the top,
  which would throw away hours — wait for it.
- **Then:** deploy, and read section 12's first real numbers out of the stored report. The weld
  queries run in the whole-corpus phase at the END of the job, so nothing about their cost is known
  until it lands; that is what the doc comment now says instead of borrowing job 816's figure.

## The gauge's FIRST RUN refuted the gauge's own calibration (2026-09-10, job 1912)

`data-quality` job 1912: 5,247 s total, 0 labels unmeasured. Windows took 4,599 s; the whole-corpus
phase took **646 s against ~128 s before these two queries existed**, so **the weld pair cost ~518 s
(8.6 min)**. That is inside the prior recorded before the run (~15 min for one pass; above ~30 min for
the two would have meant the planner took the full-scan plan). **It took the good plan. No index on
`role` is needed.**

**The bands reproduce the census exactly** — 102,840 / 32,497 / 13,297 / 1,326 at ≥3/5/10/50, from a
windowed `/v1/sql` census and an in-process whole-corpus query, two instruments with nothing in common
but the predicate. That agreement is worth more than either number alone.

**And then the listing refuted the sentence this issue wrote into the render.** It said ">= 50 is
where the reading is safe — no joint procurement has fifty buyers". The top of the very first listing:

| tender | buyers | versions | what it is |
| --- | --- | --- | --- |
| 331647 | **505** | **1** | `Dobava električne energije …` — 505 `Procedure-Buyer` rows, ONE version |
| 7966979 | 234 | 1 | **`Skupno javno naročilo za nakup goriva …`** |
| 386418 | 228 | 1 | **`Skupno javno naročilo za nakup goriva …`** |

`Skupno javno naročilo` is Slovenian for **joint public procurement**. These are not welds. They are
exactly what the caveat said could not exist at that scale, and they sit at the TOP of the listing.

**Note what the weld this issue was FILED about does here: nothing.** Tender 2816628 has 127 buyers,
and the top-40 cutoff is 210. It does not make the listing at all. A detector whose worst-40 excludes
its own motivating case is measuring something else.

**The shape, not the count, is the discriminator.** A joint procurement names all its buyers in ONE
notice; a weld accumulates them across versions:

| | buyers | versions | buyers/version |
| --- | --- | --- | --- |
| 331647 (joint) | 505 | 1 | **505.0** |
| 2816628 (the weld) | 127 | 2,983 | **0.04** |

Four orders of magnitude apart on a ratio the listing already had both halves of. `buyers/versions`
now renders as a `per-ver` column, free, and the render says plainly that it is a hint rather than a
test — a weld whose versions each named many buyers would also score high.

### Next unit (new): the widest SINGLE version

The honest discriminator is `MAX over seq of COUNT(DISTINCT organization_id)` per tender, compared
against the overall distinct count. Equal ⇒ one notice named them all ⇒ joint procurement. Overall
much larger ⇒ they accumulated ⇒ weld. Not built, and the reason is cost: it is a second grouped pass
over `tender_version_parties` (~8 min at the rate just measured), and it cannot be folded into the
existing query because `COUNT(DISTINCT …)` across versions is not derivable from per-version counts.
Worth paying once the `per-ver` column has been read for a week and shown whether the cheap hint is
already enough.

**The lesson, for the third time on this issue.** Unit 3's calibration was eForms-only and would have
blinded the gauge to the legacy era. Unit 4's timing figure was borrowed from a run that predated the
queries. This threshold was an assertion in the shape of a measurement. All three were caught, two of
them before shipping and this one by the first data it met — but the pattern is the same, and the fix
each time was to state what was measured and what was not.

## The >=50 band, split (2026-09-10, issue 377 unit 1) — the weld class is 83, not 1,326

Every tender with >= 50 distinct buyers, classified by key type and by whether its buyers are
concentrated in single notices or spread across them:

| key | concentrated (>= 10 buyers/version) | mixed (1–10) | **spread (< 1/version)** |
| --- | --- | --- | --- |
| **`ojs:` — this issue's mechanism** | 872 | 54 | **83** |
| other (eForms BT-04) | 283 | 30 | 4 |

**1,155 of 1,326 are concentrated**, i.e. many buyers named in ONE notice, which is joint procurement
rather than fusion. So the ">= 50 is where the reading is safe" sentence this issue wrote into the
render was wrong about seven entries in eight, and the section now says so.

**This issue's actual candidate set is the 83 spread legacy tenders**, not the 1,326 the band
suggested. Confirmed examples inside it: 4228069 (`ojs:2010-001662`, 354 buyers over 928 versions,
2010–2014, all legacy era) and the adjacent pair 4459994/4459995 (`ojs:2011-010241` / `-010242`,
274/272 buyers, consecutive tender ids, one Lithuanian lab equipment and one Slovak office furniture —
the mechanism firing on neighbouring notices).

**But 83 is a candidate count, not a weld count, and issue 377 found out why.** Reading the eForms
side's four spread tenders, two name themselves a **Dynamic Purchasing System**. A DPS runs rounds
over years and admits buyers over time, so its buyers accumulate across notices — the same shape as a
weld. This issue's own unit 2 predicted exactly that ("some of these are legitimate DPS rounds"). So
some unknown fraction of the 83 is legitimate, and **nobody has read them**.

That hand-read is the next unit worth doing here, ahead of the widest-single-version pass: it is 83
rows, it decides whether the repair has a target at all, and the country-spread signal that convicted
377's one real weld (341 Swiss buyers under a Finnish title) is cheap to compute for all 83.

## The 83 read (2026-09-10) — and THREE discriminators have now failed

All 83 spread legacy candidates, classified by how their buyers' countries distribute over **all**
buyers (not just the ones carrying a country):

| | tenders |
| --- | --- |
| one country >= 80 % of all buyers — DPS / national framework shape | **57** |
| cross-border (top country < 80 %) — weld shape | **8** |
| unjudgeable: fewer than half the buyers carry a country at all | **18** |

The 8 cross-border: 4204255 (159 buyers, BG 121), 4211417 (146, GB 89 — under an **Austrian**
hospital title, `LKH Univ.-Klinikum Graz`), 4017457 (111, DE 86), 3919485, 4235691, 4184397, 4011075,
4037790.

### The result that matters is negative

**Tender 2816628 — the weld THIS ISSUE WAS FILED ABOUT — lands in the "single country, legitimate"
bucket.** Its buyers are PL 103, DE 14, ES 3, SE 1: 81 % Polish, which is exactly what a Polish
national framework looks like. The country signal does not convict it.

That is the third discriminator to fail today, all three tried in order and all three measured:

| signal | fails because |
| --- | --- |
| **buyer count** (the >= 50 band) | 1,155 of 1,326 name their buyers in ONE notice — joint procurement, not fusion |
| **buyers per version** (issue 377) | a Dynamic Purchasing System accumulates buyers across notices too, identically |
| **country spread** | a weld confined to one country is indistinguishable from that country's framework — 2816628 |

**So no aggregate over the buyer set separates a weld from a legitimate multi-buyer arrangement.**
What actually convicted the two known welds was neither: 430681 by a Finnish title over Swiss buyers,
2816628 by knowing the mechanism has no guards. Both are semantic, and neither generalises to a
threshold.

### What this changes about this issue's plan

**Stop looking for a detector that decides.** The gauge's job is to MEASURE, and it does that well —
three instruments now agree on the bands, and the spread/concentrated split is real and useful. It
was never going to adjudicate individual tenders, and the two units that assumed it would (the
widest-single-version pass, and 377's candidate rule) are both worth less than they looked this
morning.

**The guards are the work.** The legacy closure admits an edge with no target-exists, no same-source
and no strictly-earlier check — consequence 2 of this issue's own decision, still unbuilt. Land those,
re-project, and read the spread bucket before and after. That is a falsifiable prediction about a
number this gauge already produces, which is worth more than any threshold argued from examples.

**The 18 unjudgeable are a second finding.** Fewer than half their buyers carry a country, which is
the organization layer's coverage gap (issues 355/357/358) showing through a different window. Worth
a line on those issues: the country campaign's residue is large enough to blind an unrelated
detector.

*Method note, recorded because two earlier cuts of this same measurement produced WRONG numbers:*
the first computed the dominant share over named countries only (a tender with 59 of 62 buyers
country-less read as "67 % dominant, cross-border" off a denominator of 3); the second could not tell
a failed query from an empty one, so transient failures under 83 back-to-back reads were recorded as
"0 countries" — 6006174 came back 0/0 and is actually SI 105, MT 1. Both were caught by sampling rows
by hand and finding them impossible. The third cut retries and separates the buckets, and reports 0
failures.

