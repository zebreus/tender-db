# 530 — some DE eForms platforms mint a fresh BT-501 UUID per notice, splitting one org into a row per notice (buyers, review chambers, winners)

Status: ready-for-agent — Build 1b (R2's fixes F1 names + F2 A7 role guard) BUILT (wf_c22ff347-fd5, 5 findings fixed), GATED (65b1e22, 1,357 s) and DEPLOYED 2026-10-19 (8e9222c); R1 RE-RUN PASSED (queue job 2660, 23 s, `530-triage/guid_twin_plan_2660.json`): 10,995 rows whole / 22,056 mentions, 0 partial, all bars and tripwires 0 (role-mix refused 0, rule/planner role disagreements 0), unsafe_list 20 groups / 42 mentions, role_mix 130 / 292; 3353580 and 6622773 no longer targets; 22752175-77 not planned. NEXT: the fresh R2 random 60 at salt 530003 (running, wf_ec8b905f-ca4; bar 0 W / N <= 3), then Build 2 (KEEP file, gate wiring, A1 recurrence arm, mint the 52 missing targets). Was: ready-for-agent — R2 FAILED 2026-10-19 (wf_5d7ad2e8-333, `530-triage/r2/r2-result.md`): random 60 = 60 R / 0 N / 0 W (pass), top 30 targets 30 R (pass), but 3 W among the flagged picks — targets 3353580 'Vergabekammer NRW' (already holds VK Rheinland + VK Westfalen) and 6622773 'Geschäftsstelle der Vergabekammer' (Thuringia's chamber, while the planned rows are VK Westfalen), and the AOK Niedersachsen group 24253392/26483158 whose Lot-ReviewOrg section carries VK Bund's phone/NUTS. NEXT: Build 1b (gate still unwired): add the two folded names (+ precautionary 'vergabekammer nordrhein-westfalen', 'vergabekammer bayern', 'geschäftsstelle der vergabekammern') to GUID_TWIN_UNSAFE_NAMES, 'geschäftsstelle' to the office nouns, and A7's role guard (a group mixing a buyer-side role — Procedure-Buyer, AddInfo, TenderReceipt, SProvider — with Lot-ReviewOrg binds), in the shared function; re-run R1 (expect ~11,045 rows, 3353580/6622773 no longer targets, 22752175-77 not planned); a fresh R2 random 60 at salt 530003 plus any new flagged/incoherent/office pick (0 W). Missing targets (52 keys / 312 rows / 636 mentions): MINT via the 234 path (parity with a rebuild). Was: ready-for-agent — unit 3 Build 1 BUILT, GATED (9488b30, 1,432 s), DEPLOYED (9488b30) and its dry plan R1 PASSED 2026-10-19 (queue job 2659, 22 s, `.scratch/tender-db/530-triage/guid_twin_plan_2659.json`): 11,103 GUID rows planned whole (22,324 mentions, 0 partial, 0 copy disagreements), KEEP 7,312 values, 0 tripwires (several-Tenders, stable, KEEP), every control as expected, 22759261/62 → 2294259, Göttingen 49 of 52 planned / 3 kept; post-state twin rows 11,437 → 329 (stable 70 → 69). Open for R2: 636 planned mentions (312 rows) have no identifier-NULL target (Build 2 decides: mint via the 234 path or keep); 41 city-flagged targets/keys, 30 buyer+ReviewOrg groups (A7), largest target 7615 Vergabekammer Rheinland +1,346. NEXT: R2 (random 60 stratified, top targets, every flagged and incoherent item; 0 W), then Build 2. Was: ready-for-agent — unit 2 DECIDED 2026-10-19 (530-triage/decision.md, with critic amendments A1-A12 adopted). Decision: pair-scoped (a) 530-T plus (c). The gate runs in mentions_and_dropped with a frozen, value-keyed KEEP_530 (7,313+ values, including merged v4 identifiers) and the name and role guards. The stock is a one-time per-mention rehome or dissolve of at most 11,258 twin rows spanning one Tender (Göttingen 22759261/62 → 2294259). (d), notice-scoped (a) and (b) are declined; 530-F (the FR 8-4-4-16 hex) goes to a new issue. 300 design:84-85 is upheld for the class and amended for the twin sub-class. NEXT: unit 3 Build 1, the pure guid_twin_refusals plus a dry dissolve-guid-twins job that reads every ORG mention of a notice, with the city screen, role composition, partial rows and the post-state twin expectations. Then R1, then R2 (random 60 + top targets + flagged reads, 0 W). Build 2 commits KEEP, wires the gate and adds the refused-value recurrence arm. 530-F filed as issue 575. Was: ready-for-agent — unit 1 MEASURED 2026-10-19: census job `guid-census` (built b81d9ce, gate green 1,466 s, deployed 0787596) ran as queue job 2655 in 12 s (`.scratch/tender-db/530-triage/guid_census_2655.json`). 81,323 v4-GUID org rows (510,037 mentions): 74,023 (91.0%) span ONE distinct tender; within-notice twin reaches 11,258 single-use rows (15.2%; notice-scoped 18,415, 24.9%); the cross-notice (name_norm, country) signal reaches 55,783 (75.4%) but would also reach 11 STABLE rows under 41 names; either signal 80.4% (14,530 single-use reached by neither). Stable population (254/255/8378 + 100+ mentions): 345 rows, 96.2% multi-tender. DE 42,040 + CH 27,837 + FR 3,597 single-use; platforms doe+ted dashed-lower 40,388, ted dashed-lower 28,285. NEXT: unit 2 decide (a)/(b)/(d)/(c) on these numbers. Was: ready-for-agent — filed 2026-10-14 from an external data review (verified: verifier + adversarial refute). Next: unit 1, the in-app census that sizes the single-use GUID class by platform.
Kind: data quality (organization identity, DE): identifier admission, split side
Relates to:
- 312 (RESOLVED 2026-08-30): measured the v4-GUID class for false merges (about 0) and for linking on a 306-row
  consortium sample, then declined the condemn and the `platform-guid` kind. That sample held no per-notice DE platform,
  and its reopen conditions name only GUID reuse across different bidders, so a split can never reopen it. This issue
  is the split side.
- 300 (DONE 2026-09-29): `300-org-fuzzy-matching-design.md:84-85` calls the hex-hash class "merge-inert singletons, not
  junk to dissolve", a stance taken from the merge side. Its §10 (no merge of a provisional row into an identified row on
  name equality) explains most of Göttingen's spread and stays out of scope here.
- 365 (DONE 2026-09-13): the `national` catch-all that admits the value. It left GUIDs alone because of 312.
- 234 / 351 (CLOSED 2026-08-21 / DONE 2026-09-05): the (name_norm, country) provisional row a refused GUID falls to.
  That is what makes a refusal cheap (see "Why 312 did not catch it").
- 440 (RESOLVED-VERIFIED 2026-09-28): `repair-placeholder-orgs`, the dissolve path for a condemned class. Its class was
  OJ S notice numbers, not GUIDs.
- 473 (ready-for-agent, filed 2026-10-01): DB file numbers (class 5) and the wrong-identifier residue (classes 3/4). The
  external report mixed DB Netz and Stadt Köln into this class; they belong there, not here.
- 456 (ready-for-agent): an identifier binds a mention whatever its name says. Köln's `VIII2` binding Vergabekammer
  Rheinland mentions is its DE shape; not this issue.

## Verify

    curl -s -G https://tenders.zebreus.click/v1/organizations --data-urlencode "name_prefix=Landkreis Göttingen" --data-urlencode limit=200 | jq '[.items[] | select(.name == "Landkreis Göttingen" and ((.identifier // "") | test("^[0-9a-f]{12}4[0-9a-f]{19}$";"i")))] | length'; for id in 254 255; do curl -s https://tenders.zebreus.click/v1/organizations/$id | jq -c '[.id,.identifier,.mentions]'; done

- **done**: line 1 reads `0`, or the count unit 2 records as deliberately kept, with its reason. Lines 2-3 are the
  stable-platform control: both rows still GUID-keyed, mentions not lower than 24041 and 72723. If unit 2 declines, it
  rewrites this block and records why.
- **open (2026-10-14)**: `52`, then `[254,"FB197F94757846738A574642AE120532",24041]` and
  `[255,"0A9EA48008E44AB6BF12D722D0AD54B6",72723]`.

## Repro

    curl -s https://tenders.zebreus.click/v1/tenders/948285 | jq -c '[.parties[] | select(.mention_name == "Landkreis Göttingen") | [.role, .organization_id]]'

2026-10-14: `[["Lot-TenderReceipt",22183653],["Procedure-Buyer",22759261],["Lot-AddInfo",22759262],["Lot-TenderReceipt",22183653]]`.
The served tender binds the buyer to three rows. The award notice alone binds it to two (22759261, 22759262);
22183653 (VAT `DE308703297`) comes from the contract-notice copies 23941063 and 26355498.

    for id in 22759261 22759265 30971684 30971734 30971757 30532145 30532147 8378 30530641; do curl -s https://tenders.zebreus.click/v1/organizations/$id | jq -c '[.id,.name,.identifier,.mentions]'; done

2026-10-14: the Göttingen, Rommel, Südbayern and Bremen rows each read a different GUID with 1 or 2 mentions;
8378 (LEONHARD WEISS) reads `9A9357BC…` with 3,380; 30530641 (Vergabekammer des Bundes) reads `1F3F4C13…` with 6.

Bounded seek (optional, uses the `organization_mentions_org` index): `echo "SELECT notice_id, section_id, name, raw_identifier, scheme FROM organization_mentions WHERE organization_id = 22759261 LIMIT 10" | ssh root@zebreus.click /root/sq.sh`
gives 2 rows: notices 24262502 (TED 00640267-2024) and 26483825 (DÖE), both `ORG-0001`, raw
`037c1508-d945-4e73-9d6e-65099a79bec1`, scheme NULL.

## What was found

**The Göttingen award.** Tender 948285 (Landkreis Göttingen, Eichsfeld Gymnasium) has an award notice published by DÖE
(notice 26483825) and copied to TED (00640267-2024, notice 24262502). In that notice every ORG section carries its own
v4 UUID in BT-501, except DÖE's own ORG-0006:
- the buyer gets two GUIDs in one notice: Procedure-Buyer → 22759261, Lot-AddInfo → 22759262;
- Vergabekammer Niedersachsen gets two: 22759263, 22759264;
- the winner, Fenster- und Fassadenbau Rommel GmbH, gets 22759265.

Each GUID row holds 2 mentions: the TED and DÖE copies of one notice. Seeks on 6 more Göttingen GUID rows return one
TED/DÖE pair each. The same procedure's contract notice (copies 23941063 and 26355498) put the buyer's VAT
`DE 308 703 297` on ORG-0001/0002/0005 and bound to 22183653. So in this award the GUID is minted per ORG section per
notice; it identifies neither the organization nor the procedure.

**Across DE the class is mixed by platform.** Stable platform GUIDs carry large volumes and do real linking:
- 255 Vergabekammer des Bundes (`0A9EA480…`), 72,723 mentions;
- 254 DB InfraGO AG – Geschäftsbereich Fahrweg (Bukr 16) (`FB197F94…`), 24,041;
- 8378 LEONHARD WEISS (`9A9357BC…`), 3,380, recurring across three notices of 2026-09-22;
- 30530641 reuses GUID `1F3F4C13…` across TED 00575467-2026, 00638319-2026 and 00652885-2026.

Per-notice minting continues today:
- Regierung von Oberbayern - Vergabekammer Südbayern appears under three GUIDs in notices 46692092, 46692352 and
  46692398, minting 30971684, 30971734 and 30971757 (2 mentions each). Ids above 30.9M, so after 312's count. None of
  these notices names it twice, so there is no within-notice twin.
- Immobilien Bremen got a new GUID row from each of two DÖE-only notices of 2026-08-19 (28671222 → 30532145,
  28671223 → 30532147), 1 mention each.
- Rommel GmbH has 7 rows: 3 single-use GUIDs (22710180, 22759265, 22800552), 2 VATs (DE150374080 22 mentions,
  DE297822100 8), a DE provisional row (17) and a NULL-country provisional row (3).

**Scale (bounded samples only; no corpus count was taken).** Landkreis Göttingen has 57 exact-name rows; 52 are
GUID-keyed at 2 mentions each (104 of 1,869 mentions, 5.6%). `/v1/organizations?country=DE&limit=200` windows:

| cursor | GUID rows | single-use | notes |
|---|---|---|---|
| 22759000 | 123 / 200 | 113 at exactly 2 | Vergabekammer Rheinland ×6, Landkreis Göttingen ×4 in one window |
| 23050000 | 104 / 200 | 77 at exactly 2 | |
| 23400000 | 27 / 200 | not counted | mostly 4 or more mentions |
| 23700000 | 32 / 200 | not counted | mostly 4 or more mentions |
| 30000000 (ids 30529996–30533031, Aug 2026 notices) | 53 / 200 | 39 (3 at 1, 36 at 2) | 10 at 4, 4 at 6 |

Single-use means 2 or fewer mentions, or better one distinct tender: DÖE-only and TED-only notices give 1 mention, so
"exactly 2" undercounts. 312's 45,974 DE v4-GUID rows is a 2026-08-30 baseline; per-notice rows have been minted since,
so it bounds nothing today.

**Why 312 did not catch it.**
- It measured false merges (rows per distinct value, about 1.0). A split also leaves one row per key, so that
  measure cannot see it.
- It measured linking (93% of 306 GUID orgs span more than one notice, mean 15.3 mentions) on a consortium sample that
  held no per-notice DE platform. Counting notices also counts each DÖE+TED double copy as two.
- Its cost argument ("every future mention minting its own provisional row") was already out of date when written.
  234 closed on 2026-08-21 and routes an identifier-less mention to the one (name_norm, country) provisional row, in the
  resolver's `identifier IS NULL` branch (`canonical.rs:20652`). A refused GUID therefore costs a one-time split
  between the standing GUID row and new mentions, not per-mention fragmentation.

**Code path (e2cbbb8, unchanged at 0776903).**
- `crates/ingest/src/project.rs:9796`: the `national()` catch-all; `:9814-9816` gates it only through `idgate::condemns`.
- `crates/ingest/src/idgate.rs:453-464`: `condemns` leaves out `uuid_v4` (:479) on purpose, citing 312.
- `crates/store/src/canonical.rs:20034`: binds on the exact (country, kind, value) triple, so each fresh UUID mints a row.
- The API serves these rows as `identifier_kind: "national"`, against CONTEXT.md:39 ("one profile per real-world
  entity, not per notice mention"); `crates/app/src/v1/docs.rs:880` gives no warning about per-notice platform keys.

Side note: Stadt Köln's `T0221974921104` (2 mentions) is a phone number with a 13-digit body, outside
`phone_shaped`'s 9–12 window (`idgate.rs:240-244`). Size it with the other phone values before anyone widens the window.

**What the external review got wrong.** The counts reproduce; the mechanism and the impact do not.
- Not all German eForms notices carry a random BT-501 ID. Some platforms mint one per ORG section per notice; others
  (254, 255, 8378) publish stable keys.
- DB Netz AG's 65 file-number rows are Deutsche Bahn's own procurement file numbers (473 class 5), not BT-501 GUIDs.
- Stadt Köln's `HRB25580` (raw `HRB 255 80`, the same on 10 notices) and `VIII2` (raw `VIII-2` in ORG-7005) are stable
  wrong keys from the 365 catch-all (473's residue). `VIII2` also binds 2 Vergabekammer Rheinland mentions to a
  "Stadt Köln" row: a wrong merge, the opposite of a split (456's shape).
- "Lookups miss most of the history" is wrong for DB Netz AG (provisional 1197942 holds 15,844 of 16,461 mentions, 96%)
  and Stadt Köln (1178521 holds 244 of 300, 81%). For Göttingen it holds (the largest row has 43%), but mostly because
  of 300 §10: provisional 16393 (802) and 2294259 (216) never merge into VAT row 22183653 (648) on name alone.

The first-pass draft was wrong too, fixed above: "three rows per notice" (two; three per served tender), "concentrated
in some id bands" (still minted), "45,974 is an upper bound" (a dated baseline).

## Proposed units

**1. Measure (M).** A dry in-app census arm, not an agent prod read: walk GUID-keyed orgs in id batches, read mentions
through `organization_mentions_org` and notices by primary key, write nothing. Wrap the new `Spec` arm in
`Box::pin(async move { ... }).await` (CLAUDE.md). Report by country, role (buyer, review/info, tenderer) and publisher
or platform:
- the share of GUID rows whose mentions span one distinct tender (count tenders or procedure keys, never notices);
- the within-notice twin signal: notices where two ORG sections share a `name_norm` under different GUIDs, and the
  share of single-use rows it reaches (the Südbayern and Bremen shapes have no twin);
- the cross-notice signal: rows whose (name_norm, country) already holds a different GUID row with 2 or fewer mentions,
  and the share of single-use rows it reaches;
- how often that cross-notice signal also hits a stable-platform bidder (a name on both kinds of platform);
- 312's "93% span more than one notice" re-taken on the distinct-tender basis, with stable platforms (254, 255, 8378)
  sized as their own population.
Record the numbers in this file with the job's queue id.

**2. Decide (S)**, recording the decision here:
- (a) Refuse a GUID when the notice itself proves it is not an entity key: one name under two GUIDs in one notice. A
  category argument, like idgate's `notice_number`. Unit 1 says how much of the class it misses.
- (b) Refuse by publisher or era, if unit 1 shows a clean boundary.
- (d) Refuse a new GUID when its (name_norm, country) already sits on a different GUID row with 2 or fewer mentions.
  Weigh unit 1's collision count: it can fire against a stable-platform bidder whose name also has a single-use row.
- (c) Decline, with the measured cost written down and a sentence in `docs.rs:880`.

Constraints for any choice:
- 254, 255 and 8378 stay bound (the Verify control).
- A wrong refusal falls back to the 234 name-scoped row, the identifier-less baseline. For a stable-GUID org the cost
  is a one-time split between the standing row and new mentions.
- For Göttingen, a refusal folds the 52 rows into the DE provisional row 2294259, not the VAT row 22183653. Merging into
  the VAT row stays out of scope under 300 §10.
- Answer 300 design:84-85 ("merge-inert singletons, not junk to dissolve") explicitly in the decision.

**3. Build and dissolve (M).**
- Add the rule at the gate, beside `condemns`.
- Pin it with tests in `crates/ingest/src/idgate.rs` and a resolver test:
  - the Göttingen within-notice twin refuses;
  - the Südbayern three-notice shape (no twin) refuses or binds, as unit 2 decides;
  - the DB-platform shape (one GUID recurring across notices) still binds.
- Dissolve the standing rows through 440's `repair-placeholder-orgs`: dry, then wet, then the fold. A refold alone may
  not move them: 522 unit 1 found a recorded mention is kept while it publishes the same raw identifier
  (`RecordedMention::publishes`, issue 434).
- Then run the Verify above.


## Unit 1 — measured (2026-10-19, queue job 2655)

- Job: `{"kind":"guid-census","dry_run":true}` on rev 0787596 (default stable_mentions 100, samples 30); 3,198 windows to org id
  31,970,152; 301,045 32/36-char candidates, 81,323 `idgate::uuid_v4` rows, 10,548 nibble-only values left out. Report and
  summary: `530-triage/guid_census_2655.json`, `job_2655_summary.txt`.
- (1) **91.0%** of GUID rows (74,023 / 81,323) span one distinct tender; 7,300 span several.
- (2) Within-notice twin (one notice, one name_norm, ≥ 2 GUID rows): 23,474 notices, 25,438 groups, 11,437 rows, reaching
  11,258 single-use (15.2%) and 70 stable; notice-scoped (every GUID row of a twin notice): 19,697 rows, 18,415 single-use (24.9%),
  114 stable.
- (3) Cross-notice ((name_norm, country) already on an earlier GUID row with ≤ 2 mentions): 57,353 rows under 4,481 names, 55,783
  single-use (75.4%). Either signal 56,280 (76.0%), neither 17,743; with the twin notice-scoped, either 59,493 (80.4%), neither
  14,530.
- (4) Cross names that also hold a stable row: 41 of 4,481; 7,658 rows under them (7,051 single-use); **11 stable rows reached**
  (a rule on this signal would have refused them at mint time).
- (5) 312's basis re-taken: all rows 59.0% multi-notice but 9.0% multi-tender (the TED/DÖE twin copies made 312's "93% span
  more than one notice" read as reuse); stable (345 rows, 311,552 mentions) 100% / 96.2%; the rest 58.8% / 8.6%.
- Breakdowns: DE 42,040 / 47,436 single-use, CH 27,837 / 29,625, FR 3,597 / 3,606, AT 126 / 176. Roles (single-use/rows): buyer
  30,567 / 31,182, tenderer_winner 13,230 / 18,131, review_or_info 27,005 / 28,789, other 3,221 / 3,221 (27,337 mixed). Platforms:
  doe+ted dashed-lower 40,388 / 45,839, ted dashed-lower 28,285 / 30,106, ted other 3,511, doe dashed-lower 1,834.
- Canaries: 254 (24,219 mentions, 4,463 tenders), 255 (73,039; 9,342), 8378 (3,382; 120) stable; Göttingen 22759261/62/65 single-use
  (twin, twin+cross, twin-notice+cross); Südbayern 30971684/734/757 and Bremen 30532145/147 single-use, cross only (no twin), as the
  issue predicted.
- **Census audit (2026-10-19 14:5xZ, hourly AUDIT):** the report's canary mention counts equal the live `/v1/organizations/{id}`
  values: 254 → 24,219, 255 → 73,039, 22759261 → 2, 30971684 → 2 (identifiers identical).


## Unit 2 — decided (2026-10-19, panel wf_14c1e8ae-b39: precision / coverage / operations judges, synthesis, critic)

Full record: `530-triage/decision.md` (judges' notes `judge_*.md`; critic amendments A1–A12 appended and ADOPTED, three of them
blocking for the build: A1 a refused-value recurrence arm, A2 the plan's unit is the (notice, section) mention, A3 the planner
reads every ORG mention of a notice through `gated_identifier` with the gate's constants).
- **Decision: pair-scoped (a) "530-T" + (c).** Precision and operations chose it; coverage dissented with (d) + notice-scoped (a)
  and its numbers are kept as the cost and the reopen conditions. Within one notice copy, ORG sections whose BT-501 is `uuid_v4`,
  grouped by (country, org_name_norm(name)); a group with two or more distinct values refuses every section's identifier (the
  mention takes the identifier-NULL 234 path, raw_identifier kept), except values on a frozen, value-keyed KEEP_530 (stable rows,
  twin rows spanning several Tenders, merged v4 identifiers: 7,313+ values) and names that are not safe as a fallback key
  (fewer than 2 content words after legal forms, a generic body name or polity umbrella: Vergabekammer, Tiefbauamt, Freistaat X,
  …) or role-incoherent groups (A7).
- **Declined:** (d) the cross-notice name rule (state- and order-dependent, consumes its own trigger rows, would have refused 11
  stable and 1,570 multi-Tender rows, folds generic buyer names); notice-scoped (a) (reaches 114 stable rows incl. 254/255); (b)
  (no clean axis; its one clean cell → issue 575).
- **Expected:** at most 11,258 single-use rows rehomed/dissolved (DE 11,237), ~22k mentions, 0 stable and 0 multi-Tender rows by
  construction; Göttingen 49 of 52 GUID rows fold into 2294259 (the DE identifier-NULL row, never the VAT row 22183653);
  ~59.3k single-use rows stay split under (c) with the cost recorded (CH 27,837; the DE no-twin shape; most current minting).
- **300 design §84-85:** upheld for the class ("merge-inert" stands; 7,300 multi-Tender rows and the 345 stable rows forbid a
  class condemn), amended for the twin sub-class proven inside its own notice.
- **Next:** unit 3 Build 1 — pure `guid_twin_refusals` + a dry `dissolve-guid-twins` planner reading every ORG mention of a
  notice, with the city screen (A6), role composition (A7), partial rows (A2) and post-state twin expectations (A9); then R1 (dry
  plan bars), R2 (random 60 stratified + top targets + every flagged row, 0 W), Build 2 (KEEP committed, gate wired, recurrence
  arm), wet, R3.


## Unit 3 Build 1 — built, gated, deployed; R1 PASSED (2026-10-19)

- Build (wf_13467940-fea: implement, three lenses, a skeptic per finding, 13 findings fixed): `ingest::project::guid_twin`
  (pure 530-T rule; production ingest unchanged, pinned) + `store::guid_twins` and the dry job `dissolve-guid-twins` (report
  `guid-twin-plan`). Commit 9488b30, gate green 1,432 s, deployed 9488b30 (projection-neutral: no fold input changed).
- **R1 (queue job 2659, dry, 22 s): PASS.** 81,323 GUID rows (81,313 distinct values) to org id 31,970,152; 208,655 candidate
  notices, 742,388 ORG mentions read. KEEP 7,312 values (multi_tender 6,967, stable 13, both 332). Rule 530-T: 25,436 twin
  groups, 8,855 refused, 22,324 mentions refused. **Plan: 11,103 rows whole (≤ 11,258 bar), 0 partial**, 0 refused mentions off
  GUID rows (a lower bound by construction), 0 copy disagreements. Guard exclusions: content_words 1,718 groups / 3,492 mentions;
  KEEP 14,863 / 29,744 (KEEP spares 29,784 mentions on 169 rows). Tripwires 0; not-planned controls all not planned;
  22759261/62 planned whole → 2294259; target-country mismatches 0; KEEP self-check missing 0.
- Targets: 568 keys, 516 with an identifier-NULL row; **636 planned mentions of 312 rows have none** (Build 2 must choose: let
  the 234 path mint the fallback row, or keep those rows). Largest target 7615 "Vergabekammer Rheinland" (29,617 today, +1,346
  from 673 rows).
- Screens: city — 11,251 planned mentions with a city, 66 absent from the target's cities (63 rows, 17 targets), 21 multi-city
  targets, 13 keys with 2+ planned cities (1,064 mentions, 524 rows), 41 flagged; roles — 30 buyer+ReviewOrg groups (A7), 0
  tenderer-mixed. Post-state: census twin rows 11,437 → 329 (stable 70 → 69, several-Tenders 179 → 174).
- R2 picks in the report: random 60 (salt 530002, stratified), top 30 targets, 41 city-flagged, 30 incoherent, 8 office
  two-word keys.
- **Target audit (2026-10-19 17:5xZ, hourly AUDIT):** the plan's Göttingen target 2294259 is `Landkreis Göttingen`, identifier
  NULL, country DE, 216 mentions (the 234 fallback row the decision names); the VAT row 22183653 (`DE308703297`, 649 mentions)
  is not a target (300 §10).


## R2 — read (2026-10-19, wf_5d7ad2e8-333): FAILED on 3 W; fixes decided

- Random 60 (salt 530002; buyer 25 / review_or_info 25 / other or tenderer 10): **60 R, 0 N, 0 W** — pass (one-sided 95 % bound
  4.9 %). Three of these are R only if their missing target is minted (23053387, 22856837, 22895106).
- Top 30 targets: **30 R** — pass. Office two-word keys: 7 R, **1 W** (3353580, the same as city item 9).
- City-flagged 41: 39 R, **2 W** — 3353580 `Vergabekammer NRW` (16 rows / 32 mentions planned, all Münster; the target already
  mixes VK Rheinland Köln and VK Westfalen Münster) and 6622773 `Geschäftsstelle der Vergabekammer` (5 rows / 10 mentions; the
  target is the Thuringian chamber in Weimar, the planned rows are VK Westfalen printing the same office name).
- Role-incoherent 30: 28 R, **2 W** — one group in two copies: AOK Niedersachsen 24253392 / DÖE 26483158 (rows 22752175-77 →
  7861): ORG-0003 (Lot-ReviewOrg) prints the AOK's name/street/e-mail but VK Bund's phone +49 22894990 and NUTS DEA22. In the
  other 28 the buyer printed itself into the review slot (UK Essen, JVA Rheinbach, H-BRS, Kerpen, RBB, IKK, KWA, GSI, HWK).
- Every W confirmed by a skeptic. Tally: 530 R2 FAILS. Random 60 is 60R/0N/0W (pass) and top 30 is 30R/0W (pass). City-flagged is 39R/2W, incoherent 28R/2W and office 7R/1W, all failing. There are 3 W findings: Vergabekammer NRW (3353580), Geschäftsstelle der Vergabekammer (6622773) and the AOK Niedersachsen ReviewOrg group (24253392/26483158). Fix: two unsafe names plus the A7 role guard in Build 1b, then re-run R1 and read a fresh 60 at salt 530003. The 312 rows without a target should be minted. Written to .scratch/tender-db/530-triage/r2/r2-result.md and r2-result.json.
- **Fixes (decided, before Build 2):** F1 names — add exact folded names `vergabekammer nrw` and `geschäftsstelle der
  vergabekammer` (+ precautionary `vergabekammer nordrhein-westfalen`, `vergabekammer bayern`, `geschäftsstelle der
  vergabekammern`) to GUID_TWIN_UNSAFE_NAMES; a list, not a `Vergabekammer <Land>` pattern (that would refuse right folds into
  single-chamber Länder); `geschäftsstelle` joins the office nouns. F2 — A7 as written: a group mixing a buyer-side role
  (Procedure-Buyer, AddInfo, TenderReceipt, SProvider) with Lot-ReviewOrg BINDS, computed from the notice's own role references
  in the shared function (~100 notice-copy groups; R1 listed only the 30 with Procedure-Buyer). F3 — Build 1b (gate still
  unwired), gate, deploy, re-run R1 (expect ~11,045 rows / ~22,204 mentions), then a fresh R2 random 60 at salt 530003 (excluding
  the 60 read) plus any new flagged/incoherent/office pick, bar 0 W / N ≤ 3.
- **Missing targets: mint** (52 keys, 312 rows → 52 new identifier-NULL rows, 636 mentions): the Build-2 gate never reads the
  database and would mint the 234 row on any rebuild, so keeping the GUID rows would break rebuild parity (A2); R3 checks the
  new rows equal the re-plan's missing keys, one per key.

## Build 1b — built, gated, deployed; R1 re-run PASSED (2026-10-19)

- Build (wf_c22ff347-fd5: implement, three lenses — decision / parity-safety / tests — a skeptic per finding, 5 findings fixed):
  commit 65b1e22, gate green 1,357 s, deployed 8e9222c (dry-only; production ingest unchanged, pinned).
  - F1: `GUID_TWIN_UNSAFE_NAMES` + "Vergabekammer NRW", "Geschäftsstelle der Vergabekammer" (R2's W1/W2) and, precautionary,
    "Vergabekammer Nordrhein-Westfalen", "Vergabekammer Bayern", "Geschäftsstelle der Vergabekammern" — a list; single-chamber
    names (Rheinland, Westfalen, Niedersachsen, Südbayern, …) pinned safe. `geschaftsstelle` (folded) joins the office nouns.
  - F2: `TwinBind::RoleMix` — a group whose sections, by the copy's OWN OPT-300/OPT-301 references, play Lot-/Part-ReviewOrg
    beside a buyer-side role (Buyer, SProvider, AddInfo, TenderReceipt, any scope) binds (`buyer_review_org`); A7's second
    clause (a Tenderer role beside any other) binds too (`tenderer_mixed`; R1 counted 0). One mapping (`guid_twin_role_ref`:
    `role_name` + the DE-1.x aliases, an unflagged DE1- ref counted as a reference as the projection does) serves the future
    gate (`guid_twin_section_roles`) and the planner (`notice_ids` PK prefix, twin copies only). Parity of role maps and
    decisions tested on the AOK shape (24253392 / 26483158).
  - Report: guard `role_mix`, `tripwire_role_mix` (refused groups whose PLANNER-read roles are incoherent) and
    `role_ref_disagree` (rule vs planner roles), both in `pass`; R2's random draw at salt 530003 skips R2's 60 (`GUID_TWIN_R2_READ`).
    22752175-77 added to the not-planned controls.
- **R1 re-run (queue job 2660, dry, 23 s): PASS.** Same walk (81,323 GUID rows, 208,655 candidate notices, 742,388 ORG mentions,
  KEEP 7,312). Rule 530-T: 25,436 twin groups, 8,735 refused (was 8,855), 22,056 mentions refused (was 22,324). **Plan: 10,995
  rows whole (was 11,103), 0 partial**, 0 copy disagreements. Guard exclusions: unsafe_list 20 / 42 (was 0), role_mix 130 / 292
  (new; references read for 23,472 twin copies, 486,540 rows, 121,235 (section, role) pairs, 7.9 s), content_words 1,718 / 3,492,
  keep 14,833 / 29,684. Party roles of refused groups: 0 buyer+ReviewOrg, 0 tenderer-mixed (was 30). Tripwires 0, role-mix
  refused 0, rule/planner role disagreements 0; every control as expected (22752175-77 not planned; 22759261/62 → 2294259);
  Göttingen 49 planned / 3 kept. Targets 554 keys (502 found); **636 planned mentions of 312 rows still have no identifier-NULL
  target** (mint in Build 2, as decided). City screen: 61 absent (58 rows, 16 targets), 36 flagged. Post-state twin rows
  11,437 → 437 (stable 70 → 69). 3353580 and 6622773 are no longer targets of planned rows.
- R2 picks of plan 2660 vs plan 2659: the top 30, the 36 city-flagged and the 7 office keys hold NO item R2 did not already read
  (its two W targets dropped out); the incoherent list is empty. Only the random 60 is fresh (salt 530003, 59 planned rows R2
  read skipped).

