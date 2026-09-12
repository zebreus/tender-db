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

## The three guards are measured, and they cannot move anything (2026-09-10)

Consequence 2 recorded that "the three guards are still worth having, and are now cheap". Measured
against the corpus before building them, none of the three can change a single grouping:

**Same-source is VACUOUS.** Every `ojs:`-keyed tender is TED-only. Checked in four 200,000-tender
windows spanning the id range: `ted` accounts for 200,000 of 200,000 in each, and the count of
`ojs:`-keyed tenders whose notices span more than one source is **0**. The corpus has three sources
(ted 13.2M notices, doe 1.1M, fts 8,667), but the legacy OJS era is TED alone — DÖE and FTS are
eForms-era and carry UUID keys. A same-source guard on this closure refuses nothing.

*And it is worth being clear about the risk it would have carried if that had come out differently:*
the weekly report tracks TED↔DÖE merging as a GOOD outcome (section 4). A same-source guard that did
bite would have broken exactly that.

**Strictly-earlier is unmeasurable as built.** `plan_ojs_edge` is written symmetrically —
`(own, edge)` and `(edge, own)` — so the citation's DIRECTION is not recoverable at grouping time.
Without direction the guard can only refuse a same-instant pair, not a forward reference. Making it
real means making the edge table directed, which is a schema change to the projection's hottest write
path for a guard whose eForms twin measured 563 of 563 references already pointing backwards.

**Target-exists is deliberately waived** and stays waived — ADR-0011's allowance, kept so identity
survives a deepening backfill, and already qualified this morning by the phantom-may-not-NAME rule.

### So the guards are hygiene, and unit 5 is the lever

This is not a contradiction of consequence 2 — it decided the guards were worth having, not that they
would cure anything, and it named unit 2's kind gate as the cure. But the plan since then has read as
though the guards were the fix, and they are not.

**Unit 2's kind gate IS the cure and it is deployed and INERT**, by its own design note: it reads the
declared kind at PARSE time, so standing legacy notices — parsed before it existed — still carry the
PIN citations it would now refuse. Nothing re-derives that without re-parsing them.

**So unit 5, the legacy re-parse, is the only thing that moves the 83.** Everything else measured
today is instrumentation: the gauge measures, the bands are trustworthy, three discriminators failed,
and the guards are no-ops. The re-parse is the one action with a predicted, falsifiable effect —
re-run it and the spread bucket should shrink, and section 12 will say by how much without anyone
remembering to look.

## The re-parse lever, piloted (2026-09-10, jobs 1005/1006) — and it refused nothing

Two-package `reparse` on `ted-export-r209`, then the projection that folds what it re-queued:

```
1914 reparse: re-parsed 283 notices across 2 packages (128234 members walked,
              0 unmatched, 0 now failing and left untouched);
              stamped 2131375 tender(s) epoch-stale; 101 package(s) held back by the cap
1915 project: 283 notices → 241 tenders (0 islands), 730 versions; 241 tenders written;
              issue-364 previous-publication citations: 260 ADMITTED, 0 REFUSED
              (prior-information 0, buyer-profile 0, periodic-indicative 0, qualification-system 0)
```

**Unit 2's kind gate refused nothing.** Not one prior-information, buyer-profile, periodic-indicative
or qualification-system citation in 260. If that holds at scale, the re-parse is inert too, and this
issue has no remaining lever — the gauge measures, the guards are vacuous, the discriminators failed,
and the cure has nothing to cure.

**Do not read that as settled.** It is 283 notices from 2 of 103 packages, and the packages the cap
took first are not chosen to be representative. The weld this issue was filed about (2816628) spans
2010–2014 and is not in this sample. The honest next step is a wider pilot — 20 packages, still
bounded, still cheap — before concluding anything about 4.5M r209 notices.

**Operational note, because a pilot should not surprise the next person.** `stamp_stale_for_profiles`
scopes by PROFILE, not by the ids actually re-parsed, so re-parsing 283 notices stamped **2,131,375**
tenders epoch-stale. That is deliberate and documented at the call site — a stale stamp forces a
rewrite that recomputes identical content, while a missed one silently loses the re-parse — and the
projection immediately after folded only its 241, so nothing ran away. But it means **any r209
re-parse, however small, ages the whole r209 era**, and that is worth knowing before someone runs a
one-package probe expecting a one-package blast radius.

## What a `reparse` package actually costs (2026-09-10) — sized wrong by 150x

The 2-package pilot re-parsed **283** notices in **55 s**, and I sized a 20-package run from it at
~2,800 notices and ~9 minutes. The real 20-package run is at **817,967 notices and 92 minutes**, and
still going.

**The first two packages are not packages of r209 notices.** They walked 128,234 archive members and
matched 283 — so they are overwhelmingly other content, and their per-package cost says nothing about
the era. Packages further in are r209-dense.

| | notices | wall |
| --- | --- | --- |
| packages 1–2 | 283 | 55 s |
| packages 1–20 | **817,967** (at pkg 19/20) | **92 min** |
| r209 era, extrapolated (103 packages) | ~4.5 M | **~8 h** |

**So the era-wide re-parse is an overnight job, not an afternoon one.** That is the number to plan
unit 5 with, and it is worth having before someone enqueues 103 packages expecting the 47 minutes a
55-second sample implies.

*Recorded because it is the third time in one day that a small sample misled about scale here* — the
weld gauge's `>= 50` threshold, the `/v1/sql` window width, and now this. In all three the sample was
not merely small, it was **drawn from the cheap end**: the first packages, the first window, the
easiest rows. A sample taken from the front of an ordered corpus is not a random sample of it.

## A legacy re-parse over 500,000 notices forces a FULL corpus re-projection

The 20-package run finished at **879,331 notices in 5,693 s (95 min)**. The projection behind it did
not run incrementally:

```
[project] INCREMENTAL → FULL fallback BEFORE identity pass: 879331 un-projected legacy notices
          exceed the closure cap (500000) (issue 305); re-projecting the whole corpus
```

**That is the planning fact unit 5 actually needs**, and it is not in this issue anywhere. The legacy
closure's scoped-incremental path has a cap of **500,000** un-projected legacy notices; above it, the
run re-projects all 14.4 M notices. So the era-wide re-parse is not "8 hours of re-parsing" — it is
8 hours of re-parsing **plus a full corpus projection**, and the projection is the larger half.

Two ways to spend that, and the choice belongs to whoever runs unit 5:

- **Chunk under the cap.** Packages sized so each re-parse stays below 500,000 notices, each followed
  by a scoped incremental fold. More jobs, no full pass. From the measured density (~44,000 notices
  per package over packages 3–20), that is roughly **11 packages per chunk**, ~9 chunks for the era.
- **Take the full pass once.** Re-parse the whole era, accept one whole-corpus projection at the end.
  Fewer moving parts, one long window, and the rebuild path is well-trodden.

The run in flight took the second by accident — 20 packages was chosen to be a *pilot*, and it landed
79 % over a cap nobody had put in front of me. It is safe (the documented fallback, announced loudly,
serving continues over WAL) and it will produce the citation measurement this pilot was for. But the
next person sizing a legacy re-parse should size it against 500,000, not against packages.

## The representative rule, measured: 5,054 components were named by a phantom

The full projection this pilot triggered is the first corpus-scale run of unit 3's phantom rule
(`2c05c8d`, deployed in `d87b95d`). Its unconditional log line:

```
[project] group step union-load: 7.0s (11007709 nodes)
[project] group step representative: 5054 component(s) named by their earliest EXISTING
          notice instead of a phantom minimum (issue 364)
[project] group step legacy-update: 116.1s (11003671 legacy)
```

**5,054 legacy components carried an identity no notice in them ever published.** Their minimum OJS
number was an edge target nobody has ingested — a mistyped digit, a citation into a year the corpus
does not hold — and under the old union-to-min labelling that phantom named the whole component.
Tender 2816628, named by `2001/S 112-185105` when its eleven siblings write `2011/…`, is one of
these.

**So the rule is not a no-op, and this is the number that says so.** Out of 11,007,709 nodes it moved
5,054 names. That is small as a share and large as a count: 5,054 Tenders are now called what their
own earliest notice calls them, and they keep exactly the members they had — the rule changes names,
never membership, and ADR-0011's allowance for phantom LINKS is untouched.

Worth setting beside the rest of today's measurements on this issue, because it is the only one that
moved anything: the guards are vacuous, three discriminators failed, the kind gate refused 0 of 260
on its pilot cohort. The phantom rule is the single change measured to have an effect, and it was the
cheapest of them.

*Also visible in the same run, and consistent with issue 369's own census:* `group step refused-keys:
3 placeholder-shaped key(s) with >= 3 distinct buyer sets`. Three, as measured there.

## The pilot measured the WRONG PROFILE, and the eras say so

The full projection finished (job 1917, 18,812 s): **14,366,679 notices → 7,942,429 tenders (684,070
islands); 2,136,456 tenders written, 5,805,973 verified unchanged**. And the citation gate:

```
issue-364 previous-publication citations: 502402 admitted, 0 refused
(prior-information 0, buyer-profile 0, periodic-indicative 0,
 qualification-system 0, DPS 0, undeclared 0, unknown kind 0)
```

**Half a million citations, not one refusable.** The admit list is three entries
(`CONTRACT_NOTICE`, `ORIGINAL_NOTICE`, `THIS_PROCEDURE`) and the refuse lists are disjoint from it, so
the gate is not a catch-all — every one of those 502,402 genuinely declared a same-procedure kind.

**The explanation is the era, and it is the third instance of one mistake.** The 19 % figure this issue
records was *"238 of 3,037, measured over four February-2013 archive days"*. Those days are **r208**:

| profile | earliest | latest | notices |
| --- | --- | --- | --- |
| `ted-export-r208` | **2010-03-10** | 2024-06-28 | 2,699,213 |
| `ted-export-r209` | **2015-12-05** | 2024-06-28 | 4,490,549 |

**r209 does not reach February 2013 at all.** I re-parsed r209 — and I chose it because it is the
LARGER profile (4.49 M against 2.70 M), which is exactly the wrong reason. The welding measurement,
and this issue's own exhibits, are r208: 4228069 is `ojs:2010-001662`, the adjacent pair 4459994/5
are `ojs:2011-010241`/`-010242`, and 2816628's typo'd name is `2001/S 112-185105`. Every one predates
r209's first notice.

So the night's zero is **not** evidence that the kind gate is inert. It is evidence that **r209 carries
no shared-publication citations**, which is a real and useful finding about r209, and says nothing
about the era the issue is about.

**Unit 5 targets `ted-export-r208`.** That is the correction, and it is worth the 5-hour projection it
cost to find.

*The mistake, named because it is now three for three today:* the weld gauge's `>= 50` threshold, the
`/v1/sql` window width, the reparse package cost — and now the profile. Each time the sample was
chosen for convenience (the round number, the first window, the first packages, the biggest profile)
rather than for containing the thing being measured. **Pick the sample that contains the phenomenon,
then check it does, before spending anything on it.**

## The r208 probe, and why unit 5 is a REPAIR rather than a measurement

One r208 package (job 1918): **62,834 members walked, 1 notice re-parsed**, 19 s; 160 packages held
back; `{"after": 24}` to continue. So r208's front is sparse exactly as r209's was, and its 161
packages are one more thing not to extrapolate from the first of.

**But the measurement this was reaching for does not need a re-parse at all.** Two things were
conflated last night, and separating them is the point of this entry:

1. **Does the corpus publish shared-publication citations?** Already answered, and by this issue:
   19 % of 3,037 citations over four February-2013 archive days. That is a property of the archive
   XML. Re-parsing cannot tell us anything the archive read did not.
2. **Does the gate refuse them correctly?** A code question, and it is **already under test** —
   `crates/ingest/tests/project.rs`: a refused citation "contributes no adjacency key at all" with
   `refused() == 1`, an undeclared citation "is refused and COUNTED", and a same-procedure case gives
   `refused() == 0`.

So **unit 5's re-parse is a repair, not an experiment.** Its job is to make standing legacy rows carry
the kind rows the gate needs, so the grouping they already have gets re-derived under the rule. It
has no finding to deliver and should not be run to produce one.

**And its price is now known, which is what last night actually bought:**

| | |
| --- | --- |
| r208 packages | **161** |
| r209 packages | 103 |
| cost of 20 r209 packages | 879,331 notices, 95 min |
| the projection that followed | **14.4 M notices, 7.94 M tenders, 18,812 s (5.2 h)** |
| trigger | >500,000 un-projected legacy notices (issue 305) |

Run it when a repair is wanted and an overnight window is available, chunked under 500,000 notices if
the full pass is not, and against **r208** — the era that holds this issue's exhibits. Not before.

**What last night established, in order:** the phantom rule renames 5,054 components (the only change
measured to move anything), r209 carries no refusable citations in 502,402 of them, the three guards
are vacuous, and the gate is correct by test. That is enough to leave this issue with a clear next
action and no open question that another job would answer.

## The rename, verified on the tender this issue was FILED about (2026-09-11)

The phantom rule's 5,054 renames include the exhibit. Read off prod after the full projection:

| | before | after |
| --- | --- | --- |
| tender id | 2816628 | **7972470** (old id now a clean `404 no such tender`) |
| procedure key | `ojs:2001-185105` — the **typo's** year, a publication the corpus does not hold | **`ojs:2011-052696`** — a real notice, `052696-2011`, published 2011-02-16 |
| versions | 2,983 | **2,983** |
| span | 2011→2018 | 2011-02-16 → 2018-11-09 |

**Both halves of the claim hold.** The component is now named by a publication that exists, and its
membership is byte-for-byte the same 2,983 versions — the rule changes what a component is called and
nothing else. The old id retiring into a 404 is `retire_regrouped_tenders` doing its job; the notices
moved, they were not lost.

**The weld itself is untouched, and that was never this rule's job.** 7972470 still fuses a 2011
heating-and-plumbing installation with seven more years of unrelated procurements. What changed is
that it no longer claims to be a 2001 publication nobody ever made.

*Method note:* the first probe of 2816628 printed `procedure_key: None, versions: 0` and read as a
silently emptied tender. It was a `404` with a perfectly clear error body, and the script printed
`.get()` defaults over it — the same "an error body is not data" mistake `docs/agents/prod-box-reads.md`
warns about, made an hour after I added a section to that file about it. Check the status, then the
body, then the number.


## Unit 5 STARTED (2026-09-12 21:52 CEST) — the r208 re-parse, chunked, with one fold at the end

The plan, from what this issue already measured plus one coupling read off `supervisor.rs` tonight:

- **`reparse` enqueues a `project` behind itself unless `reclaim_only: true`.** So the era can be
  re-parsed in chunks with no fold per chunk, and folded once.
- **A fold per chunk would be the wrong shape twice over.** `reparse` stamps tenders epoch-stale by
  PROFILE (the runbook's "not a one-package blast radius"), so every chunk's fold would rewrite all
  ~2.7 M r208 tenders; and above 500,000 un-projected notices the fold falls back to the full pass
  anyway (issue 305). One full pass at the end (~5.2 h measured 2026-09-10) is cheaper than nine
  r208-wide incremental ones.
- **The daily tick's `project` (~09:30) will fold whatever has accumulated** — above 500,000 that is
  the full pass, 5.2 h, mid-campaign. Accepted: it costs one extra fold and makes the first half of
  the repair visible early; it blocks nothing that matters on a Sunday.
- **Chunk size 10 packages, one per hourly firing while the box is otherwise idle**, `after` carried
  from each run's continuation. Density is the unknown — r208's front is sparse (package 1: 62,834
  members, 1 notice) and r209 ran ~44,000 notices per package further in — so the first chunks size
  the rest. 161 packages ≈ 16 chunks if none is skipped; the weekly tick (Sunday 03:10) and the daily
  ingest interleave between chunks, delayed by at most one chunk.

**Chunk 1: job 1332**, `{"kind":"reparse","profiles":["ted-export-r208"],"packages":10,"after":24,
"reclaim_only":true}`, submitted 21:52 CEST. Its `after`, notice count and wall go here, then the next.

### Chunks 1–5 (2026-09-12 21:52 → 23:0x CEST)

| chunk | job | packages | `after` in → out | notices re-parsed | members walked | wall |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 1332 | 10 | 24 → 36 | 2,571 | 663,587 | 75 s |
| 2 | 1333 | 20 | 36 → 56 | 7,713 | 1,244,654 | 134 s |
| 3 | 1334 | 20 | 56 → 76 | 8,573 | 1,086,637 | 161 s |
| 4 | 1335 | 20 | 76 → 96 | 36,666 | 1,016,902 | 435 s |
| 5 | 1336 | 20 | 96 → 116 | 203,089 | 890,400 | 1,812 s |
| 6 | 1337 | 20 | 116 → 136 | 685,456 | 766,735 | 5,471 s |
| 7 | 1338 | 20 | 136 → ? | running (submitted 01:33 CEST) | | |

Every chunk stamps the same **1,455,097** r208 tenders epoch-stale — the profile-wide stamp the
runbook warns about, which is why the fold is taken once at the end rather than per chunk. The front
is sparse and the density climbs with depth (2.5k → 36.7k → 203k notices per chunk); chunk 5 reached
the r209-like density (~10k notices per package, 30 min per chunk), and 70 packages remain after it —
about three more chunks of the expensive kind. Chunk size stays 20; the weekly window (03:10 CEST) is left clear.

Running total after chunk 6: **944,068 notices re-parsed over 111 of 161 packages** — already past the
500,000 cap, so the next `project` (the daily tick's, ~09:30 CEST, or an explicit one) takes the full
pass. Chunk 6 was the era's dense middle: 685k notices in 91 min, ~34k per package. Chunk 7 runs into
the 03:10 weekly window by a few minutes at most; the weekly jobs queue behind it. 30 packages remain
after chunk 7 — two more chunks on Sunday morning, then the fold.
