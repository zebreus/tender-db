# 577 U3 slice 1: code diagnosis (why only the party tables "corrected")

Read-only (2026-10-24, ~20:00Z). No cargo, no git writes. Line numbers are at the deployed rev 6cb4408
(`git show 6cb4408:<path>`) unless marked otherwise. Box: 1 bounded `/v1/sql` statement from this line
(`code-r1.sql` / `code-r1.out`, 19:57Z, 107 rows). The sibling agent's `raw/sql-state.json` records 3 more, so the
workflow total was 4 of 8 at 19:57Z. GETs: 0. Archive extracts: 0.

## Verdict

**Root cause: the reparse job itself, not a parser change.** `Db::reparse_notice` → `clear_parsed`
(crates/store/src/lib.rs:2480-2488) runs

    DELETE FROM tender_version_parties     WHERE mention_notice_id = ?
    DELETE FROM tender_version_bid_parties WHERE mention_notice_id = ?

for every re-parsed notice. Job 2773 therefore deleted every party and bid-party row that cites one of the
218,876 de-1.x notices. It left every other leaf table alone. Project 2774's issue-495 compare then read the
stored side after that delete. Every (version, table) that had held at least one such row came up short against
the fresh fold, so it counted as "differs" and was rewritten. The fold put those rows back.

- This code is in the initial import (84feb9f). It is not in `git diff 512b297 6cb4408`.
- 495's own design names it: "the re-parse's party deletes (lib.rs:2369)" and "a re-parse that deleted parties
  gets them restored" (issues/495:217, :308).
- 539 run b was a refold, which never calls `clear_parsed`, so it corrected only 2,211.

**Class:** none of (a), (b), (c) or (d) as a change in content. To the compare it looks like (b): rows missing on
the STORED side only, removed by the reparse minutes before the fold re-added them. The expected change in served
party content against the state before 2773 is 0. The evidence for that is below. The compare counts cannot show it
either way, because the old rows were gone before the compare ran.

**577 U2 (406529a/bdd055d) is not the cause of the party corrections.** On de-1.x it moves 7 role-reference ids.
Each pair maps to the same `OPT-301-Lot-*` role. It moves no bid-party field, no section and no mention.

**Slices 2 and 3 will show the same thing.** Every reparse deletes the party and bid-party rows of the notices it
re-parses. So every stale Tender that holds such a row will "correct" those two tables whatever the parser does.
The 0-corrected bar cannot be met by any reparse slice as the code stands.

## 1. What "identical" means for these tables (issue 495 compare)

- **Unit.** One (tender, seq, leaf table), for each seq in the compared prefix of the stored chain.
  `CompareTally::version` is at crates/store/src/canonical.rs:1698.
- **Stored side.** `LeafTable::compare_select_sql` (:1516-1517) runs
  `SELECT rowid, <cols> FROM <table> WHERE tender_id = ? AND seq = ?` with no ORDER BY. `stored_version`
  (:24779) drains the rows and sorts them by rowid.
- **Fresh side.** The `Pending` rows `write_version` would insert, in emission order.
- **Equal** means the same row count and every value equal by position (`same_value`). On `tender_versions` the
  quiet `value_eur_cents` is skipped (:1721). So "identical" is a positional match of the whole ordered row
  list: a missing row, an extra row, any changed value or a different order each make the table differ.
- A table with no rows on either side is not counted. A differing `tender_versions` row rewrites the whole version
  (:1725).
- **A differing table** is deleted for that version and re-inserted. For rule L, `differing_rows` (:1790) takes the
  multiset difference both ways. A pure reorder announces every lot on both sides (:1757).
- **Verified Tender:** no table differs (:24836), no stored version was dropped uncompared, the head is unchanged
  and source and kind are the same.
- **Emission order is deterministic.**
  - `tender_version_parties`: the Tender-scope `BTreeSet<Fact>`, then each lot in `v.lots` order
    (`write_version` :42773 ff., `write_facts` :43024). That is sorted by (role, organization_id, notice_id,
    section_id) within each scope, and the lot order is the same one `tender_version_lots` uses (identical in
    2774).
  - `tender_version_bid_parties`: round, then bid, then `bid.parties` order (:42905).
  - So an order-only difference needs stored rows that some writer changed in place. 2774's pattern does not
    look like that (§3).

## 2. The mechanism, end to end

1. **2773, `reparse_notice`** (crates/store/src/lib.rs:2642).
   - `keep` = the section ids the new parse creates (:2665).
   - `clear_parsed(&conn, id, &keep)` (:2667) deletes the canonical party and bid-party rows by
     `mention_notice_id` (:2480-2488).
   - It deletes only the mentions whose section is going away (:2534). It deletes the value tables wholesale and
     the vanishing sections.
   - It then inserts the new parse and sets `projected = 0`.
   - The comment at :2476 says "Only what the FK forces is deleted". That stopped being true after issue 248
     started keeping the mentions of surviving sections: the FK now forces deleting only the party rows of
     *vanishing* mention sections.
2. **2773, after the walk:** `stamp_stale_for_profiles(&refs)` (crates/app/src/supervisor.rs:19526; canonical.rs:16353)
   stamps every Tender that has a version caused by a de-1.x notice: 126,812.
3. **2774:** each stale Tender takes the compare path.
   - In `tender_version_parties` and `tender_version_bid_parties`, the stored side has lost exactly the rows
     whose `mention_notice_id` is a de-1.x notice. The fresh side has them again.
   - So `old.len() != new.len()`, the table differs, and its lots go to rule L.
   - Every other leaf table, including `lot_results`, `bids` and `contracts` (which also reference the notice, but
     by `notice_id` and not through the FK, so `clear_parsed` leaves them), compares identical.
   - That is exactly the observed split: 13 tables at 100% identical, and only these two differ.
4. **Why the ratios differ.**
   - **Parties:** a version table differs iff it holds at least one row citing a de-1.x mention. TED-twin versions
     whose roles are all their own (sdk-1.x mentions) stay identical: 278,995 of 499,644.
   - **Bid parties:** rounds carry forward into later versions. A TED version after a DÖE award still holds the
     DÖE round's bid parties, so 186,010 of 221,469 differ.
   - **0 verified:** every de-1.x-caused version carries at least its own buyer row.

## 3. Evidence from the box (code-r1, tenders 2 and 26)

| tender/seq | caused by | parties table rows cite | party rowids | verdict implied |
|---|---|---|---|---|
| 2/1 | 26213590 de-1.1 | 26213590 (8 rows) | 809,774,782–789 | rewritten by 2774 |
| 2/2 | 23555356 sdk-1.7 | 23555356 (9) | 667,936,401–409 | untouched (identical) |
| 26/1 | 26198825 de-1.1 | 26198825 (11) | 809,774,790–800 | rewritten |
| 26/2 | 23491395 sdk-1.7 | 23491395 (12) | 667,941,060–071 | untouched |
| 26/3 | 26290799 de-1.1 | 26290799 (16) + **23491395 SProvider (1)** | 809,774,801–817, **incl. the sdk row 809,774,802** | rewritten as a whole |
| 26/4 | 23851800 sdk-1.7 | 23851800 (17) | 667,941,089–105 | untouched |

- Every table holding a de-1.x-cited row was rewritten whole. That includes the carried-forward TED row in 26/3,
  which proves the table was rewritten because the de-1.x rows were missing, not row by row.
- Every table citing only sdk-1.7 mentions kept its old rowids.
- **Bid parties.** 26/3 cites 26290799 only. 26/4 cites 26290799 (the DÖE round carried forward, bids
  6913014–18) and 23851800 (bids 136–140). Both are in one contiguous block (563,858,742–756), which fits one
  fresh write. Without the table's max rowid this cannot be dated on its own.
- **Mentions survived the reparse.** The `organization_mentions` rowids of 26213590, 26198825 and 26290799 are
  39,194,233–39,382,853. Those are old rows, not re-minted ones, and they carry the same organization_id as the
  party rows. That rules out (c) for these samples.
- **Lot ids.** The lot_id values (2; 133–137) are the same on the DÖE and TED versions of each Tender. That is
  consistent with no (d).

## 4. The 512b297 → 6cb4408 diff on the party path

- **406529a / bdd055d (577 U2).**
  - What changed:
    - `Index::bind` (index.rs:910-916) now binds the first declared field over a patch field on de-1.x (D577-1 (a)).
    - parse.rs:260 calls it. It was `find_map` first-field at 512b297 parse.rs:256.
    - The top-level EXTRA insert became gap-only.
  - On de-1.x, the `/*/cac:TenderingTerms` → `[schemeName='Lot']/cac:TenderingTerms` mirror alias (index.rs:136,
    applied by the loop at :1361) used to make the PROCEDURE DE1 id win inside a lot.
  - The role-reference moves (W7 port, `551-lot-sum/b6/w7/moves_bind.tsv`) are exactly 7: AppealInformationParty,
    AppealReceiverParty, MediationParty, Fiscal/Employment/EnvironmentalLegislation IssuerParty and
    TenderRecipientParty. In each, `DE1-TenderingTerms-…-PartyIdentification-ID` becomes
    `DE1-ProcurementProjectLot-TenderingTerms-…-PartyIdentification-ID`.
  - Each pair has both ids in `DE1_FIELD_ALIASES` → the same `OPT-301-Lot-*` (project.rs:1519-1556, e.g. :1536).
    `normalise_de1_parse` (:11797) rewrites both before any read. `role_name` takes the suffix, and the scope comes
    from the value's section (`raw_roles.push`, :5880).
  - So the role, the scope (lot_id) and the target section are the same.
  - The other party-adjacent moves (EndpointID, PreSelectedParty name, UBL-ContractingPartyTypeCode →
    DE1-ContractingParty-ContractingPartyType-PartyTypeCode) are read nowhere in the fold. The last one is in the
    UBL_PARSE_ONLY ledger (project.rs:856).
  - Bid parties: `OPT-300-Tenderer`, `OPT-301-Tenderer-SubCont` and `OPT-310-Tender` (DE1 TenderingParty ids) are
    not moved.
  - Sections: parse.rs's section logic is untouched by the diff, so `keep` held every section and no mention was
    deleted. That fits the old mention rowids above.
  - On SDK-shaped inventories the moves are only BT-23, BT-531 and BT-772→BT-67(b), plus the 4 EXTRA overwrites.
    There is no OPT-30x. The W7 skeptic's relaxed-claim pass found the OPT-301 pairs "all guarded"
    (`w7/skeptic.md`:88).
- **Offline confirmation in the gate.** `project_golden` ingests `doe/eforms-de-1.1-cn-7d69b0f7.xml` (its lot
  carries TenderRecipientParty and AppealReceiverParty, which are moved elements) and `doe/eforms-de-1.2-can-799811c4.xml`
  (lot AppealReceiverParty, plus a TenderingParty/Tenderer bid party).
  - It digests `tender_version_parties` and `tender_version_bid_parties`, e.g. `4|1|Lot-ReviewOrg|11|9|ORG-7004|5`
    and `3|1|2|tenderer|14|10|ORG-0003`.
  - `project_apply.snapshot` is unchanged across 512b297..6cb4408, and bdd055d gated green. So the new parser folds
    those de-1.x shapes to byte-identical party rows.
- **cb19854 / 6ff7bed / eac7067 (576 U2/U5, 586 U2)** change fold-side touchpoint role handling
  (`touchpoint_decision`, `retarget`).
  - They apply only to notices with TouchPoint sections, and 2765/2769 already refolded those on deploy #2/#3 code.
  - Whatever their effect, it cannot explain this pattern: the clear makes every de-1.x-cited table differ
    regardless.
- **1fb7541 / 6e73ed0 / c83fec5 (579 U4, 578 U2, 583)** cover dissolve and merge resolution and stale stamping.
  They do not touch the reparse, the compare or the order in which parties are emitted.

## 5. Before/after for one sample notice: 26213590 (tender 2 v1)

- **Before** (stored parse, as the mirror alias bound it). The lot's three appeal and receipt refs carried the
  PROCEDURE ids `DE1-TenderingTerms-AppealTerms-AppealInformationParty-PartyIdentification-ID`, `…AppealReceiverParty…`
  and `DE1-TenderingTerms-TenderRecipientParty-PartyIdentification-ID`.
- **After.** `raw/get1-26213590-content.json` (the sibling agent's GET) shows LOT-0001 now carrying
  `DE1-ProcurementProjectLot-TenderingTerms-AppealTerms-AppealInformationParty-PartyIdentification-ID`,
  `…AppealReceiverParty…`, `…TenderRecipientParty-PartyIdentification-ID` (and `…TenderRecipientParty-EndpointID`).
  PROCEDURE keeps the `DE1-TenderingTerms-*` ids.
- Both spellings normalise to the same role, so the fold yields the same 8 rows. They match what is stored now
  (code-r1):

| scope | element (section) | old id → new id | role | target | org |
|---|---|---|---|---|---|
| Tender | ContractingParty (ND-ContractingParty#0) | unchanged | Procedure-Buyer | ORG-0001 | 22183653 |
| Tender | AppealInformationParty (PROCEDURE) | unchanged | Lot-ReviewInfo | ORG-0003 | 21338391 |
| Tender | AppealReceiverParty (PROCEDURE) | unchanged | Lot-ReviewOrg | ORG-0004 | 21338391 |
| Tender | TenderRecipientParty (PROCEDURE) | unchanged | Lot-TenderReceipt | ORG-0005 | 22183653 |
| lot 2 (LOT-0001) | AdditionalInformationParty | unchanged (lot-only id) | Lot-AddInfo | ORG-0002 | 22183653 |
| lot 2 | AppealInformationParty | DE1-TenderingTerms-… → DE1-ProcurementProjectLot-TenderingTerms-… | Lot-ReviewInfo | ORG-0003 | 21338391 |
| lot 2 | AppealReceiverParty | same move | Lot-ReviewOrg | ORG-0004 | 21338391 |
| lot 2 | TenderRecipientParty | same move | Lot-TenderReceipt | ORG-0005 | 22183653 |

Before = after: 8 rows, the same values in the same order. The TED twin version (2/2, 23555356) serves the same set
plus its eSender's Procedure-SProvider.

## 6. Slices 2 and 3, and what to do

- **Slice 2** (de-2.0/2.1, reparse 2776 running) and **slice 3** (sdk-1.x probe, 2778/2779) go through the same
  `clear_parsed`.
  - Expect project 2777 and 2779 to report about every stale Tender with a de-2.x or sdk-1.x party row as
    corrected, with differing tables only in `tender_version_parties` and `tender_version_bid_parties`.
  - Expect "lot changed" rows for those lots, which is feed noise for subscribers.
  - 577 U2 moves no party role reference on those inventories (§4), so this too is the clear's artifact.
- **Bar, re-read (proposed).** A slice passes when no leaf table other than these two differs, i.e. the
  identical/compared line shows 100% for every other table. Call the party-table corrections class R ("a re-parse
  restores the party rows it cleared").
  - The compare carries no evidence about party content. It has to be checked another way.
  - **Slice 3 has not started.** Before 2778 runs, snapshot the rows of 3–5 sample Tenders with sdk-1.x versions:
    one bounded statement over `tender_version_parties_version (tender_id, seq)` and
    `tender_version_bid_parties_version`, the same shape as code-r1.
  - After 2779, diff values and order. That is the only before/after content check available. Slices 1 and 2 have
    already lost their old rows.
- **Fix (S, for later reparses).**
  - In `clear_parsed`, delete party and bid-party rows only for mention sections that are vanishing:
    `DELETE … WHERE mention_notice_id = ? AND mention_section_id = ?` for each section in the existing `sections`
    list, using the `_mention_key` indexes (canonical.rs:18023).
  - Since issue 248 keeps the mentions of surviving sections, the FK no longer forces the wider delete. A reparse
    whose sections survive would then be compare-neutral, and its compare would really test the parser change.
  - Extend the test `a_reparse_clears_every_parsed_table` / the 495 restore test to pin this.
  - It cannot help slice 2, whose reparse is already deleting.
- **Loose end, not a finding.** "766 verified unchanged" are Tenders the change-set touched that were not stamped
  stale. `stamp_stale_for_profiles` stamps every Tender with a de-1.x-caused version, and a party row cites only
  its own Tender's notices, so none of the 766 can have lost rows. One bounded check could confirm it if wanted.
