# 546 — legacy TED serves the buyer's and the winner's address NUTS as the tender's `country`, so `?country=` over-matches

Status: ready-for-agent — **unit 4 RUNNING 2026-10-24** on ad8e305 (deploy #2): probes counted 4,488,813 field carriers (refold-fields 2760) and 4,490,549 ted-export-r209 notices (refold 2762); wet **refold 2764 (profile r209, expect 4,490,549) + project 2765**, posted 12:41:56Z, upper bound ~20:45Z. A banner says countries/places are being re-derived. NEXT when 2765 ends: read its summary (corrected count), run the Verify (8432519 → ['FRK22']; 20297187 no longer matches ?country=DE), then retire the caveat (RETIRED "issue-546 refold") in the next code deploy. Unit 3 was deployed in 512b297. Was: ready-for-agent — unit 3 BUILT and GATED 2026-10-19 (wf_ce324a9f-c14: built, reviewed, 6 findings fixed; gate green at 42cc601, 1432 s), HELD OFF MAIN as `.scratch/tender-db/546-triage/u3-42cc601.patch` (`git apply` clean on da9b0de): issue 551 B6's flip deploy must carry no other projection change, so unit 3 lands and deploys only after the flip and its U8 verification. Then re-apply the patch, re-gate, deploy, and run unit 4 (cohort count + refold). The patch also carries the CHANGELOG entries for 541 and 571 and issue 540's MODIFIED_ADDRESS reason string. Was: ready-for-agent — unit 1 measured 2026-10-19 (.scratch/tender-db/546-triage/notes.txt, verify_notes.txt). r208 has no address NUTS: 2013 (W13, n=5,000) and the 43 r208 notices of 2019 in W19 carry none. In r209 (W17 2017 / W19 2019 / W22 2022), 32.2% / 32.9% / 23.6% of notices carry a buyer or winner code that adds a `?country=` match the performance place does not give. 0.9% / 1.1% / 0.7% match a wrong country, and about 42% / 42% / 31% serve a longer list. 1.5% / 0.9% / 0% are address-only: 27 of these 30 are F12/F13 design contests, and the other 3 are F14 corrigenda. Every ORG-section TED-NUTS code is restated by CA_CE_NUTS ∪ TENDERER_NUTS. Decision D1 (unit 2, recorded below): drop buyer, winner and other-address NUTS from canonical `place`, with no fallback on empty, and keep them on the notice layer. Next: unit 3 builds it. Then unit 4 runs the dry-run cohort count and a refold of about 4.5M r209 notices (about 4.5 h); the Verify should then read ['FRK22']. Was: needs-triage — filed 2026-10-15 from issue 532's unit-1 decision (D7), which confirmed the precedent and left
it out of 532's scope. Next: measure how many legacy tenders serve a buyer/winner-only NUTS, then decide.
Kind: API semantics (the `country` field and `?country=` on legacy TED)
Relates to:
- 532 (country-only place; D7 records the interaction: a carried legacy buyer NUTS suppresses a later eForms
  `place_country` row, a miss, not a wrong value);
- 48 (`?country=` is a NUTS place-code prefix);
- 437 (FTS award-item region).

## Verify

    curl -s --max-time 20 "https://tenders.zebreus.click/v1/tenders/8432519" | python3 -c "import json,sys; print(json.load(sys.stdin)['country'])"

- **done**: `['FRK22']`, the performance NUTS only. The fix may also give a documented alternative, e.g. a separate
  buyer-region field.
- **open (2026-10-15)**: `['FRI12', 'FRK22']`. FRI12 is the buyer's (ORG-1 `TED-NUTS`, PROCEDURE `TED-CA_CE_NUTS`);
  only FRK22 is `TED-PERFORMANCE_NUTS`.

## What was found

`CLASSIFICATIONS` maps `TED-NUTS`, `TED-CA_CE_NUTS` and `TED-TENDERER_NUTS` to field `place`, scheme `nuts`
(crates/ingest/src/project.rs:361-366). This includes `TED-NUTS` inside ORG sections. r209/rules.rs:390 parses them
as Nuts. So in the legacy era a tender's `country` is the union of three things:
- the place of performance;
- the buyer's region;
- the winner's region.

eForms and DÖE serve only the place of performance. `country` therefore means different things in different eras.
`?country=FRI` finds 8432519 although its works are in FRK22. This is partly why legacy TED looks fuller than eForms
in the empty-`country` rate (532's sample: TED 3.6% vs eForms 15.5%).

The /docs `country` row (docs.rs:170) says "NUTS place codes" and does not mention buyer or winner addresses.

## Units

1. **Measure (S).** Over a bounded legacy window, count:
   - tenders whose `place` rows come only from buyer/winner fields;
   - tenders with both kinds of row;
   - tenders with performance only.

   This needs the field id per row. `tender_version_classifications` stores only `field='place'`, so read the
   notice-layer ids for the window.
2. **Decide.** Options:
   - Re-map the buyer/winner ids to a distinct field (e.g. `buyer_place`), which `?country=` stops matching unless the
     version has no performance place. That is the 532 D4 shape: fallback only on empty.
   - Keep the behaviour and document it per era.
   - Drop the buyer/winner rows.

   A re-map needs a legacy refold (about 7M tenders), so size it first.

## Triage (2026-10-19, workflow wf_2a43dfc2-d09: a measurer and an adversarial skeptic)

Files: `.scratch/tender-db/546-triage/`. The Status line above is the skeptic-corrected one.

### Measured

Unit 1 is done. I sampled four legacy notice-id windows, one per era, and classified every NUTS row that CLASSIFICATIONS maps to `place` (project.rs:352-366) in each notice.
- Performance rows: PERFORMANCE_NUTS, ORIGINAL_NUTS, CURRENT_NUTS, and TED-NUTS outside an ORG-n section. OBJECT_DESCR lands in LOT-n; LOCATION_NUTS and NOTICE_DATA land in PROCEDURE.
- Address rows: CA_CE_NUTS, TENDERER_NUTS, and TED-NUTS inside an ORG-n section, which is an ADDRESS_* `Rule::Org` block (r209/rules.rs:346, parse.rs:368/413).
- ORG-n roles come from the ORG pointer rows in notice_ids (c_* queries).
- An address code "adds" when it is neither equal to nor a prefix of any performance code. Then `?country=` gains a match that the performance place does not give.
- Served `country` is every scheme='nuts' row of the head version, tender and lots, whatever the field (read.rs:1288/2627). The seeds read the (scheme, code) index (read.rs:2788/2842).

How it was run:
- 12 of 12 /v1/sql requests. The first was refused with a 400 (group_concat is not allowed) and is kept as a_w19_groupconcat_refused.*. No 408.
- Every query is a single-table PK range seek; plans are in plans.txt.
- 5 public GETs, each ≥2.2 s apart.

The issue's Verify still reads open: tender 8432519 serves ['FRI12','FRK22'] today.
- Its notice 20298159 falls inside window W19 and is the positive control: LOT-1 TED-NUTS FRK22, PROCEDURE PERFORMANCE_NUTS FRK22, ORG-1 TED-NUTS FRI12, PROCEDURE CA_CE_NUTS FRI12.

Code findings that shape the fix:
- Both legacy profiles go through the r209 parser.
- R2.0.8 address blocks carry no NUTS. ORIGINAL_NUTS equals LOCATION_NUTS, the performance place (fixture f02-000333-2014).
- A prefixed id such as TED-DESCRIPTION_PROCUREMENT.NUTS is not mapped, because stem() returns the whole id. The F20 MOD place falls under this, but all 71 such notices also carry a mapped performance place, so nothing is lost there.
- Nothing other than CLASSIFICATIONS reads field `place`.
- has_destination's Classification arm is `canonical_name(CLASSIFICATIONS)` (project.rs:9362). So the build must keep the dropped ids registered as read and deliberately skipped, or 368's sieve will list them as unmapped.


### Numbers

Per notice. Perf = performance rows only; harmless = address codes that add no match; adds = address codes that add a match; addr-only = no performance row.

| window | era, profile | notices | perf only | harmless | adds | addr-only | no NUTS row |
|---|---|---|---|---|---|---|---|
| W13 17,300,001-17,305,000 | 2013, r208 | 5,000 | 3,680 | 0 | 0 | 0 | 1,320 |
| W17 19,394,601-19,395,768 | 2017-08, r209 | 1,168 | 0 | 775 | 376 (32.2%) | 17 (1.5%) | 0 |
| W19 20,297,001-20,298,464 | 2019-03, 1,421 r209 + 43 r208 | 1,464 | 19 | 939 | 468 (32.5%) | 13 (0.9%) | 25 |
| W22 22,470,601-22,472,192 | 2022-06, r209 | 1,592 | 0 | 1,216 | 376 (23.6%) | 0 | 0 |

- W13 (r208) has no address NUTS rows at all.

The "adds" broken down by the shortest prefix that matches only through an address code (L2 = a new country):

| window | L2 | L3 | L4 | L5 |
|---|---|---|---|---|
| W17 | 10 | 227 | 73 | 66 |
| W19 | 15 | 308 | 73 | 72 |
| W22 | 11 | 167 | 89 | 109 |

- 8432519 is an L3 case: FRI against FRK.
- `?country=` matches a wrong country in 0.9% / 1.0% / 0.7% of r209 notices (W17 / W19 / W22).
- A wrong region at NUTS 1-3 level is matched in about 23-32%.
- The served list grows at all in 41.6% / 42.2% / 30.7%.

Who adds the extra codes, W19 / W17:
- buyer 328 / 188;
- winner 188 / 238;
- other addresses (review body and similar) 27 / 32.

The ORG-section rows add nothing beyond the coded rows. In each notice, the TED-NUTS code set of every ORG section is contained in that notice's CA_CE_NUTS ∪ TENDERER_NUTS set:
- W19: buyer 1,420/1,420, winner 522/522, other 99/99;
- W17: 1,168/1,168, 562/562, 124/124;
- W22: 1,592/1,592 (roles not read).

Address-only notices are design contests: query d_w17_forms shows the cluster at 19,395,618-635 is F12 (TD D) and F13 (TD R), plus one F14. Those forms carry no performance-place field.

Sizing (estimate, not re-measured):
- r209 holds about 4.5M notices (the figure in prod-box-reads.md).
- 532's refold ran 3.29M notices in 3h19m, so a refold of ted-export-r209 takes about 4.5 h.
- With 30-42% of r209 notices changing their served list, expect on the order of 1.3-1.9M announced corrections.
- Late r208 files (2016-2019) were not sampled for CA_CE/TENDERER rows.

Raw outputs, SQL and the analysis are in /home/user/tender-db/.scratch/tender-db/546-triage/: notes.txt, analysis.json, redundancy.txt, served_growth.txt, f20_mod_place.txt, plans.txt, requests.log, api_requests.log, analyse.py, and the a_/b_/c_/d_ *.sql and *.out.json files.


### Decision (as proposed; amended by the skeptic points below where they conflict)

D1 (proposed for the owner to record). Legacy TED buyer, winner and other-address NUTS leave the canonical `place` field, with no fallback when a version is left empty. `country` and `?country=` mean the place of performance in every era. The rows stay on the notice layer: notice_classifications field_id TED-CA_CE_NUTS, TED-TENDERER_NUTS, and TED-NUTS in ORG-n sections. The docs point there.

Mechanics:
- Take TED-CA_CE_NUTS and TED-TENDERER_NUTS out of CLASSIFICATIONS.
- In NoticeState::read's classification arm, skip TED-NUTS when its section is an Organization section. This is section-aware, like F14_DATE_FIELD.
- Keep all three registered as read in has_destination.
- TED-NUTS in LOT-n or PROCEDURE, and PERFORMANCE/ORIGINAL/CURRENT_NUTS, stay mapped.

Why, on the evidence:
1. The over-match is common, not an edge case. A third of r209 notices gain a false region match, and about 1% gain a false country match. Option "keep and document per era" leaves `?country=` at region level wrong for 2016-2023, and clients cannot separate the rows. Rejected.
2. Re-mapping to a separate field such as `buyer_place` under scheme `nuts` is rejected. Every read predicate and the country seeds filter on scheme='nuts' alone, through the (scheme, code) index. A new nuts-scheme field would still be served unless the hot-path seed queries also filter on field. A different scheme spelling would bend what scheme means. Dropping the rows touches no read path.
3. Nothing unique is lost. The ORG-section codes are restated by the coded CA_CE/TENDERER rows in 100% of sampled notices, and the notice layer keeps every row.
4. No fallback on empty. A buyer region is not a place of performance, unlike 532's place_country, which is a coarser performance place. eForms and DÖE serve [] in the same situation. The only losers are design contests (F12/F13, 0-1.5% of r209 notices), and they have no performance field to read. In a chain, a later address-only notice no longer republishes the (place, nuts) key, so the earlier performance place carries. That is a gain over today's supersede clobber.
5. It removes 532 D7's interaction: a carried legacy buyer NUTS can no longer suppress an eForms place_country row.
6. It is reversible additively. A documented buyer-region field can be added later without breaking clients.

Accepted costs:
- 1-1.5% of r209 notices (design contests) serve [], or what an earlier version carried.
- The legacy empty-`country` rate rises by about 1 point in 2016-2023.
- About 1.3-1.9M corrections are announced by the refold.
- PROJECTION_EPOCH stays unbumped: this is a profile-scoped mapping fix (532 D8).


### Units (proposed)

- U1 Measure (S) — DONE 2026-10-19: four era windows, per-notice classes, the role and redundancy read, and the address-only form read. Raw data in .scratch/tender-db/546-triage/.
- U2 Decide (S) — record D1 and its reasoning in the issue: drop the buyer/winner/address NUTS from canonical `place`, no fallback, rows stay on the notice layer. Rejected: re-mapping to a separate nuts-scheme field (it would touch the scheme-only read and seed paths), and keep-and-document (a third of r209 notices over-match).
- U3 Build (M), project.rs plus docs in one commit. (a) Remove TED-CA_CE_NUTS and TED-TENDERER_NUTS from CLASSIFICATIONS. (b) In NoticeState::read's classification arm, skip TED-NUTS when the row's section kind is Organization (section-aware, F14_DATE_FIELD precedent). (c) Keep all three ids true in has_destination's Classification arm as read and deliberately skipped, so 368's sieve and data_quality do not list them. (d) Tests: synthetic 8432519 shape (PROCEDURE PERFORMANCE FRK22 + LOT TED-NUTS FRK22 + ORG TED-NUTS FRI12 + CA_CE FRI12) serves [FRK22]; f03-000988-2019 serves [UKG13] (unchanged); f02-000245-2019 serves [MT00] (was [MT, MT00]); an F12 design contest serves []; a legacy CN with buyer NUTS chained to an eForms country-only CAN now serves the place_country (532 D7 interaction gone); a later address-only notice no longer clobbers a carried PERFORMANCE place; r208 fixtures unchanged; project_golden diff reviewed (legacy r209 fixtures lose address codes only). (e) Docs: docs.rs `country` row (around :160-170), openapi `country` parameter and Tender.country, sql.rs CLASSIFICATION_FIELD_NOTE: place of performance only in every era; legacy buyer/winner NUTS are on notice_classifications (TED-CA_CE_NUTS / TED-TENDERER_NUTS); a refold caveat tagged `(issue-546 refold)` on the same surfaces, pinned by an assert_says docs test; a CHANGELOG entry. (f) Run ops/check.sh with GATE-EXIT read.
- U4 Refold and Verify (M). Deploy, then a dry-run cohort count: refold-fields TED-CA_CE_NUTS,TED-TENDERER_NUTS with tables=notice_classifications and an expect guard. This also catches any late r208 carriers; if the cohort equals ted-export-r209, use `refold profiles=ted-export-r209` instead. Then refold plus project, about 4.5M notices, about 4.5 h at 532's rate, about 1.3-1.9M announced corrections. Verify: 8432519 serves ['FRK22'] and a W19 L2 example (20297187: perf NO0, buyer DEB3J) no longer matches ?country=DE. Then retire the caveat (RETIRED "issue-546 refold").
- U5 Read-back (S): after the refold, re-measure the legacy empty-`country` rate against 532's sample (TED 3.6%; expect about +1 point in the r209 era, design contests) and re-run W19's class census on the stored versions (adds -> 0).


### Skeptic (sound)

- Checked and holds. analyse.py, re-run on the saved a_/b_/c_ outputs, gives an analysis.json identical to the stored one. The capped windows are handled correctly: a_w17, a_w19 and a_w22 hit LIMIT 10000, the last id is dropped, and the b_ denominators end on that id. No c_ read is capped. Every query is a PK range seek. The code citations hold: project.rs:352-366, :5547, :6965 and :9362, and read.rs:1288, :2627 and :2788, which are scheme-only. The cited fixtures exist. The measurement is the one the issue's unit 1 asks for: field id per row, read from notice_classifications.
- Address-only composition is misreported. My bounded read (v_addronly_forms.out.json) finds W19's 13 address-only notices are all F12. W17's 17 are 14 F12/F13 plus THREE F14 corrigenda (19394896, 19395635, 19395679), not 'plus one F14'. So 27 of 30 are design contests. For the F14 corrigenda the fix is a gain, not a cost: they stop clobbering the carried performance place. Correct the numbers text and the accepted-costs wording ('design contests serve []').
- The 'late r208 (2016-2019) not sampled' caveat is partly answered by the triager's own data. W19 contains all 43 r208 notices from 2019, ids 20298227-20298368. They are 18 perf-only and 25 with no NUTS row, with zero address rows. W19's 'perf only 19' is those 18 plus one r209 notice (20298225). State this. The U4 cohort count stays as the full-corpus check.
- Mixed denominators. The W19 percentages divide by 1,439 (notices with a mapped place, the 43 r208 included), but the table's notices column says 1,464. The 'of r209 notices' country rates also include r208. For r209 only: adds 468/1,421 = 32.9%, wrong country 15/1,421 = 1.1%. This is cosmetic but should be labelled.
- Internal inconsistency on D1. The proposal calls it 'proposed for the owner to record' and lists U2 'record D1' as an open unit. The status line says 'Decision D1 (recorded below)' and Next skips to unit 3. Pick one: record D1 now and mark U2 DONE, or make U2 the next step. ready-for-agent is right either way, since nothing waits on a person.
- The measurement is per notice, but served `country` is per tender (head version after supersede). The '1.3-1.9M corrections' figure multiplies notice-level fractions by about 4.5M r209 notices. Corrections are counted per tender, so label it an upper-bound order of magnitude. The issue itself said a legacy refold covers about 7M tenders; the U4 dry-run cohort count is what settles it.
- U3(d) test wording. drop_place_country_beside_nuts (project.rs:6945) still suppresses an eForms place_country beside a carried legacy PERFORMANCE nuts row. So the '532 D7 interaction gone' test must use a legacy CN with ONLY buyer/address NUTS (an F12/F14 shape), not 'a legacy CN with buyer NUTS'. Claim 5 of D1 should say it holds only for buyer-only carriers.
- Minor point on U4. refold-fields TED-CA_CE_NUTS,TED-TENDERER_NUTS selects by those two fields only, so a notice with ORG-section TED-NUTS and no CA_CE/TENDERER row would fall outside it. The sample shows 100% containment, and the ted-export-r209 profile fallback covers the rest. Keep the expect guard and the cohort-vs-profile comparison as written.

## Scheduling constraint (2026-10-19)

Unit 4's refold (~4.5M r209 notices) writes `country`/`place` change rows on a large share of legacy Tenders. Issue 551's B6
flip enumerates its id list from every tender change row after cursor 958787061 (B6 decision points 5 and 9), under a
5,000-id cap per run; a 546 drain before that flip would swamp the list. So **unit 4 waits until 551 B6's flip and its
U8 verification are done** (no earlier than the 10-21 post-chain gap). Unit 3 (the build) can land and deploy before then:
it changes only Tenders folded afterwards.



## Unit 3 — built, gated, held (2026-10-19)

- Built by wf_ce324a9f-c14 (implement, review lenses, a skeptic per finding; 6 findings fixed). Focused runs green (87 passed).
- `TED-CA_CE_NUTS` and `TED-TENDERER_NUTS` are no longer CLASSIFICATIONS on the canonical `place`; `TED-NUTS` inside an
  Organization section is skipped (`enclosing(&sections, ..., &[ORGANIZATION_KIND])`); the notice layer keeps all three.
  /docs, openapi (`country` parameter, `Tender.country`, `TenderDetail.classifications`) and the /v1/sql notes say so,
  each with the pre-refold caveat tagged `issue-546 refold` (retire through RETIRED after unit 4's Verify).
- Committed 42cc601 with the CHANGELOG entries for 541/571 and 540's reason string; `ops/check.sh` green, 1432 s, marker
  `gate-v2 42cc601`. Main was then rebuilt WITHOUT it (da9b0de = 638f624 + scratch only) and the commit saved as
  `546-triage/u3-42cc601.patch`, because the 551 B6 flip (U6/U7, ≥ 10-21) is a deploy whose exact-set drain must show
  path W's changes alone (D1/D2 tolerance 0): a deployed 546 would re-place every legacy tender that drain re-projects.
- After the flip and U8: `git apply` the patch (resolve any drift), re-gate, deploy, then unit 4.

**551 W-on drain protocol (2026-10-23, `551-lot-sum/b6/decision-w-on-drains.md`).** Unit 4 is class N at corpus scale (about
0.75–1.9M corrected Tenders; place/country only, no path-W input). It runs after 539's drain, after the batched U2 deploy and
its probe P1, and after 576 U3. It runs BEFORE 577 U3 (recorder's check R1). It needs ONE gap of about 10–12 h: job and
project ≥ 8 h at 550's measured rate, E1 1–2.5 h, then the reads. That gap may fall in U9 days 8–14 only if four conditions
hold: E1 plan-probes as a SEARCH; E1 passed its replay and ran with parity on ≥ 2 segments; the registry switch-off (E4) is in
place; and the day-7 read passed. Otherwise unit 4 runs after 11-04. Before the run: P0 serves as its code probe; take the dry
cohort count and a reviewed expect; run start_rule with the upper-bound minutes; register 579's 6 as order-only NOTEs; salt
`551-b6-drain-546-`; write a writer-lock line. The check is gd5 class N over (K, CA] after the project. Never cancel the
project: on overrun the chain queues behind it, and CA is read before the chain's first job. Expected: NEW = MOVED = LOST =
SILENT = 0. Any NEW is a STOP and is read as an enumeration miss (all if ≤ 30, otherwise a salted 30 under U5's bar). Place
moves on served legacy heads are NOTEs. Cost: about 150–380 statements. The fallback reader adds 375–950, which moves unit 4
to after 11-04 over two gaps. **Next step:** unit 3 is deployed (512b297), so next is unit 4 under this protocol in that slot.
