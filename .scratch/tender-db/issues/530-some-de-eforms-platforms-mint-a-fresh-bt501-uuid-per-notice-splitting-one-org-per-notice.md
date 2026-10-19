# 530 — some DE eForms platforms mint a fresh BT-501 UUID per notice, splitting one org into a row per notice (buyers, review chambers, winners)

Status: ready-for-agent — unit 1 MEASURED 2026-10-19: census job `guid-census` (built b81d9ce, gate green 1,466 s, deployed 0787596) ran as queue job 2655 in 12 s (`.scratch/tender-db/530-triage/guid_census_2655.json`). 81,323 v4-GUID org rows (510,037 mentions): 74,023 (91.0%) span ONE distinct tender; within-notice twin reaches 11,258 single-use rows (15.2%; notice-scoped 18,415, 24.9%); the cross-notice (name_norm, country) signal reaches 55,783 (75.4%) but would also reach 11 STABLE rows under 41 names; either signal 80.4% (14,530 single-use reached by neither). Stable population (254/255/8378 + 100+ mentions): 345 rows, 96.2% multi-tender. DE 42,040 + CH 27,837 + FR 3,597 single-use; platforms doe+ted dashed-lower 40,388, ted dashed-lower 28,285. NEXT: unit 2 decide (a)/(b)/(d)/(c) on these numbers. Was: ready-for-agent — filed 2026-10-14 from an external data review (verified: verifier + adversarial refute). Next: unit 1, the in-app census that sizes the single-use GUID class by platform.
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
