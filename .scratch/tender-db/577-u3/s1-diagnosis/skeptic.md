# 577 U3 slice 1: adversarial review of code.md and data.md

Read-only, 2026-10-24 20:10–20:30Z. Code is read at the deployed rev 6cb4408 (`git show 6cb4408:<path>`), because the
worktree has other agents' uncommitted edits in `lib.rs`, `canonical.rs` and `project.rs`. No cargo, no git writes.

**Budget used by this line:** 1 bounded `/v1/sql` statement (`skeptic-r1.sql` / `.out`, 20:21Z, 0.64 s, 825 rows, not
truncated). That is an `IN`-list seek on tenders 1–150 through the `(tender_id, seq)` indexes, plus PK seeks on
`notices`. With it the workflow total is **7 of 8**. I deliberately left the last statement for the slice-2 before/after
read in §7. I also made 1 public GET (`skeptic-get1-tender-28.json`, 20:25Z) and no archive extracts.

## Verdict

- **Root cause: CONFIRMED.** Job 2773's `clear_parsed` deleted every party and bid-party row citing a re-parsed
  notice. Project 2774's compare then saw short stored tables and rewrote them. I re-derived the code path (§1).
  Two data tests over 150 Tenders could have refuted it, and both held (§2, §3).
- **Class: neither order-only nor content, as far as the compare can see.** The compare saw rows missing on the
  stored side, and it is blind to whatever content change hides behind that. I call this class R (restore).
- **Served content: no evidence of any change.**
  - **Counts are conserved.** In every rewritten de-1.x party table whose old row count can be recovered from a rowid
    gap, the old count equals the new count: 25 of 25. The same holds for the 6 bid-party tables that can be isolated.
    Earlier the evidence was 2 tables.
  - **The parse change only relabels party references.** 577 U2 changes nothing on the party path except a field-id
    label, and both labels fold to the same role (§1.4).
  - **What remains possible.** An equal-count substitution (same number of rows, a different role or org) cannot be
    ruled out by the data. The code rules it out for 577 U2's moves. Parse changes older than 512b297 were never
    examined (§4.1).
- **Slice 1 passes on the evidence available.** A 0-corrected bar cannot be measured on any reparse slice until the
  clear is narrowed (issue 591).

## 1. The key path, re-derived (file:line at 6cb4408)

### 1.1 The delete

| Step | Where | What happens |
|---|---|---|
| Walk | `crates/app/src/supervisor.rs` `run_reparse` → `crates/ingest/src/process.rs:705` | Calls `db.reparse_notice(..)` for each member. |
| Keep set | `crates/store/src/lib.rs:2642` `reparse_notice`, `:2665` | `keep` = the section ids the new parse creates. |
| Clear | `lib.rs:2667` → `clear_parsed` (`:2447`) | Loop at `:2480-2488` runs `DELETE FROM {table} WHERE mention_notice_id = ?` on `tender_version_parties` and `tender_version_bid_parties`. **Unconditional, and not scoped by `keep`.** |
| Mentions | `lib.rs:2517-2537` | Only the mentions of vanishing sections are deleted (issue 248). |

- The comment at `:2471-2474` names the served gap itself: "It does leave a window in which the Tender carries texts
  and amounts but no parties".
- The comment at `:2476` says "Only what the FK forces is deleted". That is out of date: the FK now forces only the
  rows of vanishing mention sections.
- `git diff 512b297 6cb4408 -- crates/store/src/lib.rs` does not touch this code.

### 1.2 The stamp and the restore

- **Stamp.** `supervisor.rs:19526` → `stamp_stale_for_profiles` (`canonical.rs:16353`). It stamps every Tender that has
  a version caused by a notice of these profiles (`JOIN tender_versions v ON v.caused_by_notice_id = n.id`).
- **Rewrite.** Stale Tenders skip the early return at `canonical.rs:24172-24180`, so their party rows are rewritten.
  Unstamped Tenders whose chain is unchanged early-return and write nothing. Those are the "766 verified unchanged".
- **Compare.**
  - The stored rows come from `compare_select_sql` (`canonical.rs:1516`), drained and sorted by rowid in
    `stored_version` (`:24779-24808`).
  - The test is `old.len() == new.len() && all(same_value)` position by position (`:1716-1719`).
  - A short stored side always differs. An order-only difference would also count as a difference.
- **495 expected this.** The design note at issues/495:217, :308 and :393 says "a re-parse that deleted parties gets them
  restored".

### 1.3 Mentions are untouched unless their facts change

- `resolve_mentions` keeps a recorded mention as long as `recorded.publishes(m)` (`canonical.rs:23094`).
- Otherwise it refreshes the mention in place, and the project summary then prints "N recorded mention(s) refreshed, M
  re-bound" (`supervisor.rs` around `:6136`).
- No de-1.x organization field moved under 577 U2. W7's `moves_bind.tsv` has no
  `Organizations`/`Company`/`NoticeResult`/`TenderingParty` row for eforms-de-1.x. So no mention should refresh.

### 1.4 577 U2 relabels and nothing more

- **Sections do not depend on the bind.** The section is chosen from the matched branches' node at `parse.rs:229-256`,
  before and independently of the bind at `parse.rs:260`. So sections are unchanged and no mention is deleted.
- **The bind change.** `Index::bind` (`index.rs:910`) changes only which `FieldInfo` labels the same element, value and
  section. On de-1.x the 7 moved role references go from `DE1-TenderingTerms-…-PartyIdentification-ID` to
  `DE1-ProcurementProjectLot-TenderingTerms-…`.
  - All 14 ids are `id-ref` in `crates/ingest/sdk/fields-de-1.x.json`, so `is_ref` is unchanged (`value.rs:88`).
  - Each pair maps to the same `OPT-301-Lot-*` (`project.rs:1536-1552`).
  - `de1_mark_reference` (`:11839`) marks both.
  - The role scope comes from the value's section (`raw_roles.push`, `project.rs:5880`).
  - The same fact therefore comes out of either label.
- **No bid-party field moved.** No `NoticeResult`/`TenderingParty` field moved on de-1.x, and `moves_extra.tsv` has no
  de-1.x row at all.

**Root-cause points that I tried and failed to refute:** code.md §1–§4 and data.md "What actually happened".

## 2. Test A: is a table rewritten if and only if it holds a de-1.x-cited row? (skeptic-r1)

If anything other than the delete had driven 2774's party corrections, some table without a de-1.x-cited row would
carry 2774 rowids, or some table with one would not.

Over tenders 1–150 (352 versions):

| table | holds a de-1.x-cited row | rewritten by 2774 | not rewritten |
|---|---|---|---|
| parties (2774 block 809,774,782–951) | 29 | **29** | 0 |
| parties | none (311) | **0** | 311 |
| bid parties (2774 block 563,858,742–802) | 26 | **26** | 0 |
| bid parties | none (107) | **0** | 107 |

- The split is exact, with no partly rewritten table.
- 29 of 29 de-1.x versions hold their own rows, which supports code.md's "0 verified".
- The 26 rewritten bid tables include 13 caused by TED (sdk) versions and 1 by a de-2.1 version. They carry DÖE rounds
  forward.

## 3. Test B: old row count vs new row count, recovered from rowid gaps (skeptic-r1)

- **Method.**
  - The 10-08 replay wrote party rows in strict `(tender_id, seq)` order. For example, tender 5 ends at 667,940,708,
    tender 7 starts at 667,940,719, and the 10 slots between are tender 6's 10 rows, since rewritten elsewhere.
  - Any run of rewritten versions between two intact anchor versions therefore left a gap equal to its old row count.
  - Runs that span a missing Tender id, or that contain a de-2.x version, are skipped. Slice-2 reparse 2776 is clearing
    de-2.x rows right now.
  - Script: `gaps.py`, run on `skeptic-r1.out`.
- **Parties.** **25 of 25 runs that isolate a rewritten de-1.x table have old count = new count.**
  - Examples: 26/1 11=11, 26/3 17=17, 46/1 17=17, 97/3 8=8, 108/3 9=9, 114/3 6=6.
  - 3 more runs differ by +1, +1 and +2. Each also contains an sdk version rewritten at 807.83M by an earlier refold
    (36/1, 113/1, 129/1, probably 2769's touchpoint roles). The DÖE share of those gaps cannot be isolated.
    Read alongside the 25 clean runs, they do not point at 2774.
- **Bid parties.** 2 runs isolate rewritten tables: (48/1, 48/2, 50/1, 50/2) 6=6 and (64/3, 64/4) 3=3. The other 11
  runs are skipped because of slice 2.
- **What this rules out.** It rules out lost or duplicated rows, which is the regression class, at 15× the sample data.md
  had (25 tables against 2). It cannot rule out an equal-count substitution.

## 4. What I could not refute, and residual risks

1. **The investigators diffed from the wrong baseline.** They checked parser changes 512b297→6cb4408. A reparse
   applies every change since each notice was *last parsed*, which may be well before 512b297. I could not find when
   de-1.x was last re-parsed. Several things limit the risk:
   - All 13 other tables are identical, so no value-level parse change reached the fold.
   - Sections are node-driven.
   - Count conservation holds (25 of 25).
   - The XML matches 8/8 and 11/11 (data.md).
2. **No pre-2773 copy of the row contents exists.** "Net served change 0" is an inference: counts are conserved, 577 U2
   only relabels, and the restored rows match the XML. It is not a measurement. The only true before/after anywhere is
   tender 28's slice-2 rows from 19:57Z (§7).
3. **Not checked by me: data.md's "2774 reported 389 placeholder".** The 2774 summary I was given has no dropped-parties
   clause. 2738's "0 void, 389 placeholder" is confirmed in `551-lot-sum/b6/u9/jobs/w539-b_ca.json`. In any case this
   number describes what the party decider dropped. It says nothing about role references.
4. **The 766 "verified unchanged" were not checked by data.** 2738 ran the same 127,578 Tenders and wrote 126,813 with
   765 unchanged, so these are the same island-mates both times. They have no de-1.x-caused version, and party rows cite
   only notices in their own chain (one version per notice). So I see no lost rows there, but this rests on reasoning
   alone.

## 5. Corrections to the two reports

- **data.md: "The TED copies … were not rewritten".** This is true for party tables only (0 of 311 TED or other tables
  rewritten). TED-caused **bid-party** tables that carry a DÖE round *were* rewritten: 13 of the 26 rewritten bid
  tables in tenders 1–150. data.md shows this for 26/4, but the headline overstates it.
- **data.md "Where measurable, old = new" (2 tables).** This is now 25 of 25 tables. The 3 non-isolable runs are
  explained in §3.
- **code.md recommendation 1 (pass a slice when only the two party tables differ).** This is necessary but not
  sufficient. The compare is blind to party content on a reparse, so the bar should also require one of:
  - a before/after read of the party rows of a few sample Tenders;
  - the 591 fix, after which the compare covers party content again.
- **code.md and data.md, the size of the window.** Its reach is bigger than stated (§6.1).

## 6. Additional findings

1. **The slice-2 window, measured (live).** At 20:21Z, in tenders 1–150 (18 Tenders with de-2.x versions):
   - **12 of 32 de-2.x versions serve 0 party rows:** 16/1, 34/1, 54/1, 61/1, 69/1, 71/1, 77/1, 92/1, 98/1, 132/1,
     142/1 and 146/1.
   - **6 more have visible holes:** 16/3, 28/5, 34/3, 61/3, 69/5 and 110/3. 28/5 has 2 of its 6 rows left.
   - **TED heads lose the DÖE bid round.** At 20:25Z `GET /v1/tenders/28` still serves bid TEN-0001/26710678 with
     `"parties": []`, so 2777 had not yet restored tender 28.
   - **Likely worse for DÖE-only Tenders.** The sample has no DÖE-only Tender and no de-2.x head. For a DÖE-only Tender
     the *head* itself would serve no buyer during the window. That is inferred, not observed.
   - The banner should say this while a reparse is running.
2. **A sibling bug, inferred from code, outside slice 1.** The issue-404 moved-identity path in daily `process`
   (`lib.rs:1898-1969`) also calls `clear_parsed` (`:1946`) and sets `projected = 0`.
   - Neither it nor `process.rs:401` stamps the Tender stale. The notice id is kept, so the chain is unchanged and the
     next fold takes the early return (`canonical.rs:24172`).
   - A re-keyed notice's party and bid-party rows would therefore stay deleted, and its version's `publication_id`
     would stay stale.
   - There is no projection test for that path.
   - It is rare (2773 re-keyed 0; issue 404 was 281 notices).
   - The 591 narrowing would remove the party loss, though not the stale `publication_id`.
3. **The 591 fix is sound as proposed.** It deletes only the party and bid-party rows of vanishing mention sections,
   by `(mention_notice_id, mention_section_id)`.
   - That is the exact set the FK forces: `sections` in `clear_parsed` is already "mention sections not in `keep`".
   - Org rebinds of surviving mentions still reach the party rows, because `stamp_stale_for_profiles` forces the
     rewrite.
   - Its test should also cover a re-keyed (404) notice.

## 7. Cheap checks left for the orchestrator

- **Slice-2 before/after (the last SQL statement, 8 of 8).** Run it once 2777 has finished (check `GET /v1/tenders/28`
  first: bid 26710678 should have its tenderer back). Read tender 28 in the same shape as `raw/s2_parties.sql` and
  `raw/s3_bid_parties.sql` (one UNION ALL).
  - **Expected:** 28/5's parties are the 6 rows of data.md §6, with the same values in the same order and new rowids.
    The bid parties of 28/5 and 28/6 are the same, with new rowids. 28/6's parties keep rowids 667,941,139–144.
  - **Any other difference** is a real content change, and the first one measured anywhere.
- **2774's full summary** (from `/admin/jobs`, which `/v1/sql` cannot read):
  - There should be no "recorded mention(s) refreshed" clause and no "mention(s) retired" clause, meaning the org layer
    is untouched corpus-wide.
  - The dropped-parties clause should read 0 void / 389 placeholder, as in 2738.
  - Its touchpoint clause should be compared with 2738's.
  - None of this needs a box read.

## Files (all in `/home/user/tender-db/.scratch/tender-db/577-u3/s1-diagnosis/`)

- `skeptic-r1.sql`, `skeptic-r1.out`, `skeptic-sql-state.json`
- `skeptic-get1-tender-28.json`
- Analysis script: `/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/skeptic577/bin/gaps.py`
