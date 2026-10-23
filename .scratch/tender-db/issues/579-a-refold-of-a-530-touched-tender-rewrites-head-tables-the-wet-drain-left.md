# 579 — a refold of a Tender issue 530's wet drain touched rewrites head tables the drain left in place

Status: ready-for-agent — TRIAGED (wf triage-579: measurer + 2 adversarial skeptics; 6 of 6 bounded /v1/sql reads, no
408, every one `truncated:false`; skeptic-corrected, `## Triage (2026-10-21, wf triage-579)` below). **Unit 1 DONE:** the
difference is row ORDER, not bindings. `Db::dissolve_org_mentions` re-inserts each repointed winner at a fresh rowid
(INSERT OR IGNORE + DELETE), and its in-place party / bid-party org-id UPDATE leaves a row where the old GUID id sorted; the
fold writes each group sorted by org id and the compare engine compares rowid-ordered rows by position, so the next refold
rewrites the table and announces its lots. Values, row sets and org ids already match what a fresh projection writes (by
code; not measured). Size: 9 of 530's 2,742 touched Tenders; 3 healed by lc01 (497154, 675459, 780251), **6 open**
(408007, 413501, 454861, 179248, 340956, 823340). Served: only the head `parties[]` order of 413501 (and its
`bids[].parties`) and 179248; whether 179248's v_awards representative buyer flips is unchecked (U3). Severity low.
NEXT: U2 pre-register the 6 (551 U9 notes, 546 U4 expectations); U3 one bounded read; U4 dissolve-path parity (stamp
touched Tenders stale + a wet-then-refold-rewrites-0-tables test) on 578 U2's gate and deploy as its own unit; U6 file the
merge-side follow-up (`repoint_org_references` leaves the same order drift). **No explicit refold of the 6 until 551
records an exemption from its W-on standing rule (which names "579's refold") or re-runs the port diff.** Was:
needs-triage — filed 2026-10-21 from issue 551 B6's L_changed run lc01 (project 2698). Next: unit 1, name the
leaf table(s) and columns that differ between 530's in-place rehome and a fresh projection, size it over 530's touched
set (2,742 Tenders, `551-lot-sum/b6/u6/l_changed.json` source S7), and decide between refolding that set and fixing the
wet path.
Kind: data quality (organization identity / rebuild parity): the stored state after a stock drain is not the fixed point
a refold reaches; medium severity until sized (no value moved on the three seen; what moved is head tables, likely party or
winner rows). Sized by the triage: low (row order only; 6 Tenders open, 2 of them with a served array-order difference)
Relates to:
- 530 (DONE 2026-10-21): wet `dissolve-guid-twins` 2672 + 2673 (10,995 GUID orgs dissolved, 22,056 mentions moved, 52 minted;
  change cursor 958904203 → 958919213; **0 Tenders stamped stale**, project 2674 projected 0 notices). Its Build 2 review
  asked for rebuild parity ("a from-archive rebuild under the gate produces exactly the org layer the wet path leaves").
- 578: the same family — `repair-placeholder-orgs` (440) lands a mention where a rebuild would not; 530 Build 2 factored
  440's body into the shared `Db::dissolve_org_mentions`.
- 551 B6 (path W flip): this surfaced as 3 non-W corrections in its L_changed run lc01; it changes no 551 verdict (no value
  moved), and any refold over 530's touched set must respect 551's standing rule (port-diff re-run before a W-input drain
  while W is on: org merges feed R1/R6).

## What was seen (2026-10-21, issue 551 B6 lc01)

- `refold-lot-sum` lc01 (dry 2696 → wet 2697 → project 2698) refolded 489 L_changed Tenders under the flip rev e718684:
  481 verified, **8 corrected; tables 85,144 skipped / 8 rewritten; rows 72,768,313 skipped / 376 rewritten; 76
  correction rows** (lot 68, tender 8). Five corrections are path-W fires (head value columns only — those rewrite no
  table: stage 2's 3,100 corrections rewrote 0 tables).
- The other three — **497154, 675459, 780251** (TED; heads 2026-06-16 / 2025-07-22 / 2024-10-31; max-elected, no
  derivation) — moved **no** served tender column the PK snapshot holds (value, currency, EUR, derivation, seq,
  published_at, projection_epoch all identical before and after), yet the compare engine rewrote 8 head tables / 376 rows
  among them, so its rule L announced every head lot (34 / 6 / 8 lot `changed` rows) and one `tender changed` each.
- Their only change rows between cursor 958787061 and the run are 530's wet drain: `tender changed` at 01:29Z (497154),
  01:30Z (675459), 01:30Z ×2 and 01:32Z (780251). 530 rehomed their mentions in place and did not refold them.
- So the in-place rehome (parties / bid parties by `(mention_notice_id, mention_section_id)`, winners by the 309 origin
  tier, per 530's A2) leaves some head table different from what a fresh projection writes for the same notices.

## Open questions for unit 1

1. Which leaf tables and columns differ (the project's per-leaf tallies, `tables_rewritten_by_leaf`, are not in the job
   summary; a dry compare of one touched Tender or a fixture run of the wet path followed by a refold will show it).
2. Is the stored (pre-refold) state wrong as served — e.g. a party row on the GUID org id 530 deleted, a winner on the
   wrong target, a duplicate party row — or only a representation difference the compare engine counts (row order,
   rowids, a denormalised name)?
3. How many of 530's 2,742 touched Tenders carry it (lc01 refolded only those inside the lot-sum cohort: 3 of them)?
4. Fix: refold 530's touched set by `tender_ids` (stamps + project; after 551's port-diff re-run if W is on), and/or make
   the wet path write exactly what the projection writes (or stamp its touched Tenders stale so the project reconciles).

## Triage (2026-10-21, wf triage-579)

Run: a measurer and two adversarial skeptics; every skeptic correction below was checked against code or files before it
was recorded, and claims that could not be confirmed were dropped. Code is cited at HEAD e79114c. canonical.rs, read.rs,
guid_twins.rs and supervisor.rs are unchanged since e718684, the rev lc01 ran under. The measurer's first canonical.rs
numbers came from the pre-flip tree c27c8bf and ran about 30 lines low; they are corrected here. Box reads were taken on
2026-10-23. Nothing was built or run, and nothing on the box was written.

### Evidence

**Reads: 6 of the 6 allowed.** All were bounded `/v1/sql` reads: PK or `_version`-index seeks, IN lists of 200 or fewer,
LIMIT 500. There was no 408, and each read took 0.5–1.5 s including ssh. Every statement was plan-probed first on a local
copy of `v551/plan.db` with the prod deferred indexes added. Two turso traps were caught there before any read went out:
a `b.role = a.role` join term that built an ephemeral full-table index (fixed by wrapping the columns), and
`b.rowid IN (a.rowid-1, a.rowid+1)` returning 5 rows where SQLite returns 96 (replaced by two equality seeks). Each query
form gave identical results on turso and SQLite against synthetic DBs.

- **S7 rebuild.** S7 was rebuilt from the `551-lot-sum/b6/u6/lchanged/changes/*.json` tender rows with cursor in
  (958904203, 958919213]: 2,742 Tenders. It was split into the 18 campaign windows (2672's one window plus 2673's 17).
  Each window's removed, added, tender and organization-changed counts match the 2672/2673 manifests. A moved row carries
  one of its window's target ids (organization-changed plus added: 554 ids), so each arm checks only rows whose
  `organization_id` is in that window's target list.
- **The order test.** For parties and bid parties, each target row is compared with its rowid neighbours in the same
  (version, lot or bid, role) group against the fold's sort key. For winners, each target row is compared with the rest
  of its version on (lot-result rowid, org id).
- **R1** (100 random S7 Tenders; party, bid-party and winner arms): 0 rows.
- **R2** (200 more): 3,150 of 8,302 party rows sit on targets and no winner rows do. All three arms found 0 inversions.
  The any-cause neighbour inversions (PX/BX) were also 0, so stored data holds the fold's sort order and the test gives
  no false positives.
- **R3** (every Tender of campaign windows 2, 3 and 5, winners, against that window's targets):
  - Target winner rows: 408007 (3, rowids 1361071811-813), 413501 (15, 1361071814-828), 454861 (3, 1361071829-831),
    995207 (9, one of them at 1361071845), and the three lc01 Tenders, now at 1361272918-1361273023 from lc01's rewrite.
  - Inverted pairs: 408007 21 (seq 3-4, head 4), 413501 560 (seq 3-4, head 4), 454861 9 (seq 1-2, head 2). None for the
    lc01 three or for 995207: its moved row already sorts last.
- **R4** (windows 1-5, Tenders not already sampled; parties and bid parties, plus the 56 winner rows of 408007 and 454861
  as detail):
  - Example: 408007 seq 3, lot_result 16976493, target 1176914 (Capgemini Deutschland), lot-result rowid 1113646231. Its
    rowid is 1361071811 while its neighbours are 1233239419..428, so it is last in rowid order and sixth in fold order.
    is_buyer is NULL; the content is otherwise identical. 454861 shows the same pattern.
  - Party inversions: 413501 (2 + 10 rows, seq 3-4; targets include minted 31971420 `]init[ DCP LDA` and 1289810
    Deloitte Consulting), 179248 (2 rows, seq 1-2; target 13025902 "Kreis Coesfeld, Abteilung 30") and 340956 (2 rows,
    seq 1-2; minted target 31971450 NRW.URBAN).
  - Bid-party inversions: 413501 (15 rows; target 8808564, Deloitte GmbH WPG).
- **R5** (windows 6-18, unsampled, parties; 360–4,554 target rows per arm): 823340, 8 rows across seq 3-14 (targets
  8223129 SVLFG and 4193988 vdek, 2673 windows 5 and 8).
- **R6** (skeptic; PK seek on `tenders` for 10 ids): all `ted`, epoch 4, no derivation. Heads: 179248 = 2,
  340956 = 4, 408007 = 4, 413501 = 4, 454861 = 2, 823340 = 16, 995207 = 6, 497154 = 4, 675459 = 8, 780251 = 4.
- **Files.** In `/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/`:
  - `579/q/read1.sql`, `read2.sql`, `read3.sql`, `read_A.sql`, `read_B.sql` and `579/q/read1.out` .. `read5.out`;
  - the generators `579/mksql3.py`, `mksql5.py`, `mkw2.py`, `mkpb2.py`, plus `579/windows.json` and `reads_plan2.json`;
  - R6 in `579sk/r6.sql` and `r6.out`.
- **lc01 cross-check** (`551-lot-sum/b6/u7/jobs/lc01_wet.json`): all 9 were among its 5,000 named ids. It admitted 489,
  the lc01 three among them. The 6 were not admitted, and R3–R5, read after lc01, still show their pre-refold order.

### Mechanism

**Bindings already match.** `RecordedMention::publishes` (canonical.rs:15644, used at :23039) keeps a recorded binding
when name, country, raw_identifier and scheme agree. The 530-T gate only sets the normalised `identifier = None` and keeps
the raw value (project.rs:6106-6113). So a fresh projection binds each moved mention to the target the wet path wrote, and
530 decision.md:185 ("a refold moves nothing") holds for bindings. What it missed is that issue 495's compare engine
treats rowid order as part of stored state. `stored_version` sorts each leaf's rows by rowid (canonical.rs:24724-24751),
and `CompareTally::version` compares them by position (:1682, :1703-1706). When the rows match but in another order, it
rewrites the table and puts every row in `moved`, so every lot the table holds is announced (:1736-1742). ADR-0017 D1
counts order because PARTIES_SQL, BID_PARTIES_SQL and the stats read have no ORDER BY.

The shared dissolve body has two write shapes that leave the fold's fixed-point order:

1. **Winners, the main cause: 6 of the 9 Tenders, including all of lc01's three.** `dissolve_org_mentions` repoints a
   winner with `INSERT OR IGNORE … SELECT …, w.is_buyer` (canonical.rs:42082-42100) and then DELETEs the loser row
   (:42127-42138). The moved row takes the next rowid, at the end of the table. The fold pushes winners in
   round → lot_result → winner order (:42731), after `sort_unstable` and `dedup` per lot result (project.rs:9202).
   `repoint_org_references` (canonical.rs:7466) works differently: it DELETEs the keep's duplicates (:7500) and then
   UPDATEs winners in place (:7513), so the rowid survives. The dissolve body's comment at :42012-42014 calls its
   insert-then-delete "the repoint_org_references shape". That comment is false, and it is how the divergence got in.
   440 runs the same body (`Legacy440`, canonical.rs:41432).
2. **Parties and bid parties: 4 Tenders and 1 Tender.** The dissolve UPDATEs `organization_id` in place by mention key
   (canonical.rs:41983-41995 for parties, the bid-party UPDATE right after), so the rowid stays where the old GUID id
   sorted. The fold writes tender scope first and then each lot (:42633, :42662). Within a scope it writes the
   `BTreeSet<Fact>` in order (:42935), and `Fact::Party` derives Ord on {role, organization_id, notice_id, section_id}
   (enum :3220, variant :3238). BidParty derives Ord on (role, organization_id, section_id) (canonical.rs:7222) and is
   sorted at project.rs:9093. A target id that crosses a sibling's id therefore reorders the group. Low reused ids
   (1289810, 13025902) and minted ids above the 22.8–23.1M GUID band (31971420, 31971450) both cross.

**Ruled out (by code):**
- Dangling org ids: FKs are on, and the residue check refuses a window that leaves a reference.
- Duplicate rows: INSERT OR IGNORE dedups the way the fold's sort/dedup does.
- A winner on the wrong target: the origin tier picks the section the fold binds.
- is_buyer drift: the dup-collapse UPDATE mirrors `clear_collapsed_buyer_flags` (:7540).
- Origin columns: unchanged.
- The 530-T gate itself.
- Other leaf tables: the only org references are these three (ORPHAN_SWEEP_REFERENCE_SQL).

**Not measured:** row-set parity against a fresh projection. The reads tested order on target rows only. "Row set and org
ids match" rests on the code: ambiguous winners go to tier 5, and the origin and chain tiers pin one section or key.

**Why only windows 2, 3 and 5 for winners:** the manifests show winners moved only there (27 + 6 + 19, 8 of them dups).
Bid parties moved only in windows 1, 2, 3 and 5 (24 + 57 + 9 + 19 = 109).

**The same class exists outside the dissolve path.** `repoint_org_references` has 8 callers: provisional-merge, rule
merges, r3, nhs-body, e2-altid, rekey and nested-repair (canonical.rs:25054, 26234, 28966, 32118, 33338, 34847, 38050,
40798). It UPDATEs parties, bid parties and winners in place, and none of those call sites stamps the touched Tenders
stale. Any merge whose keep id crosses a sibling's id leaves the same party, bid-party and multi-winner order drift, and
the same correction at the next refold. A fix inside `dissolve_org_mentions` does not close the class (U6).

### Served harm

No served value, row set, org id, name or is_buyer flag differs. Only row order differs, and only part of it reaches a
reader.

- **Winners: not visible.** `results_of` (store read.rs:3588; query at :3626-3630) is planned through
  `sqlite_autoindex_tender_version_result_winners_1` on turso 0.7.2 (plan-probed by the measurer and a skeptic), so
  winners come out in PK order (lot_result_id, organization_id) whatever their rowids. The winner difference exists only
  for the compare engine.
- **Parties and bid parties: visible at the head only.** PARTIES_SQL and BID_PARTIES_SQL (read.rs:3313, :3325) seek the
  (tender_id, seq) `_version` indexes, which return rowid order. The bid-party index is deferred (canonical.rs:18013).
  `tender_detail` reads only `tenders.current_seq` (read.rs:3403-3404; `results_of(conn, id, tender.seq)` at :3525). No
  `/v1` route serves a non-head version (v1/mod.rs:239-251), and `/v1/sql` promises no order. So:
  - **413501** (head 4): `parties[]` and `bids[].parties` list the moved party where its old GUID id sorted;
  - **179248** (head 2): the same for `parties[]`;
  - **340956** (inverted at seq 1-2 only, head 4) and **823340** (seq 3-14 only, head 16): nothing served.
  - For 413501 and 179248 the refold's correction is a real served change, so announcing it is right. The elements are
    identical, and the derived is_buyer flag does not depend on order.
- **v_awards' representative buyer: unchecked (U3).** `buyer_organization_id` and `buyer_name` are each a scalar
  `… p.role LIKE '%uyer%' … LIMIT 1` over the head's parties (canonical.rs:1288-1302). They rely on (tender_id, seq, rowid)
  order, so a buyer-family inversion between different org ids that includes the head's first buyer row changes the
  served buyer, and a refold flips it back.
  - 413501: its moved parties are consultancies (`]init[ DCP LDA`, Deloitte Consulting, Deloitte WPG), so a buyer-family
    group is unlikely. This is inferred from names, not measured.
  - 179248: its target is a public body (Kreis Coesfeld, Abteilung 30), so it plausibly is buyer-family.
  - It matters only if 179248 has award rows. `v_tender_buyers` (:1267-1277) lists every buyer, so it is order-only.
    No other served read picks a party by rowid; the read.rs buyer filters are EXISTS.
- **Operational cost: the main one.** The next refold of each open Tender, for any reason, reports it corrected. For
  winners and non-head parties that is a change no reader sees. It adds noise to the change feed and non-W corrections to
  551's U9 watch and any later L_changed run.

### Size

**9 of 530's 2,742 touched Tenders (0.33%).** 3 are healed by lc01 (497154, 675459, 780251). Their 6 winner tables fit
lc01's 8 rewritten tables; the other 2 are unattributed, because the job summary carries no per-leaf tallies. 6 are open,
and all 6 are outside the lot-sum cohort.

Rule L adds the head lots only when a head table differs (`count_compare`, canonical.rs:24764-24800; `head_differs` is
set only when `is_head`, :1733-1735). It always adds every lot named by a differing table's rows at any version.
Version-level party rows (lot_id NULL) name no lot.

| Tender | Head | Differing tables | Expected at its next refold |
|---|---|---|---|
| 408007 | 4 | winners, seq 3-4 | corrected; 1 `tender changed` + the head lots |
| 454861 | 2 | winners, seq 1-2 | corrected; 1 `tender changed` + the head lots |
| 413501 | 4 | winners, parties, bid parties, seq 3-4 | corrected; 1 `tender changed` + the head lots |
| 179248 | 2 | parties, seq 1-2 | corrected; 1 `tender changed` + the head lots |
| 340956 | 4 | parties, seq 1-2 only | corrected; 1 `tender changed` + only the lots its reordered party tables name (possibly none) |
| 823340 | 16 | parties, seq 3-14 only | corrected; 1 `tender changed` + only the lots its reordered party tables name (possibly none) |

The total is about 26 tables or fewer: 6 winners, up to 18 parties and 2 bid parties. **"No value moves" holds only if no
other deployed but undrained projection change reaches them.** 546 U3 (legacy TED place) was deployed on 2026-10-23
~19:00Z (512b297, 551:1572), and its U4 refold is held. All 6 are `ted`, so a chain holding an r209 notice also moves
served place and country at that refold.

995207's moved winner already sits last in fold order, so it would verify.

**Coverage:**
- parties: complete, every S7 Tender on target rows;
- bid parties: windows 1, 2, 3 and 5 (the only ones that moved bid parties) plus the 300 sampled;
- winners: windows 2, 3 and 5 (the only ones that moved winners).

It is a lower bound only where a deleted row leaves a rowid gap between neighbours inside a sort group. **Severity: low.**

### Decision

1. **Close unit 1 with this finding:** representation only, 530's bindings are correct, and 6 Tenders remain open.
2. **Do not refold all of S7.** 2,736 of its Tenders would verify, and the run would be a large compare while W is on,
   all for 6 Tenders.
3. **Heal the 6 by their next natural refold, pre-registered (U2).** No explicit refold now. No tender-id refold exists
   outside `refold-lot-sum` (supervisor.rs:4107-4111), whose finder admits only cohort Tenders, and 551's standing rule
   names this refold (Interactions).
4. **Fix the dissolve path by construction: option (d).** After its writes, every window pushes the Tenders it touched
   into `refold`, the path tier 5 already uses (guid_twins.rs:3122-3127: requeue the notices, `stamp_tenders_stale`). The
   drain's own `project` then reaches the fold's fixed point, and the compare writes nothing for Tenders that verify. A
   test pins it: a fixture wet dissolve, then a refold with the compare on, rewrites 0 tables.
   - (d) alone is complete for the dissolve path.
   - (c), repointing winners in place, only trims the drain's own winner corrections and still reorders multi-winner lot
     results when ids cross. It is optional.
   - The false comment at canonical.rs:42012-42014 is corrected either way.
5. **Option (e), comparing winners in PK order (the order they are served in), is deferred to U6's follow-up** as an
   ADR-0017 D1 question. It is not needed to close 579.

Rejected:
- Registering the 6 in 539's drain expectations: that drain is DÖE-only (repair-notice-instants plus the DÖE R3 refold)
  and all 6 are TED.
- Folding (d) into D578-1: (d) changes what every 440 run writes, because each run would then refold its touched Tenders.
  It is a separate decision and a separate unit, even though it shares 578 U2's gate and deploy.

### Units

- **U1: measure and decide (S).** Status: DONE (this section). Blocked by: nothing.
- **U2: pre-register the 6 (S, docs only).** Status: ready-for-agent. Blocked by: nothing.
  - Add the per-Tender table above to 551's U9 notes as expected non-W, order-only corrections, so a refold that reaches
    them is not read as W drift.
  - Add the same table to 546 U4's refold expectations, which may be their natural heal, with 546's place and country
    delta allowed unless U3 shows no r209 notice in their chains.
- **U3: one bounded read (S).** Status: ready-for-agent. Blocked by: a fresh read budget, and not on a U9 check morning.
  - Read the head parties of 179248 (seq 2) and 413501 (seq 4) by `_version` seek: rowid, role, lot_id,
    organization_id, mention_notice_id, mention_section_id.
  - Count `tender_version_lot_results` at those heads.
  - Find the profiles of the 6 chains' notices (tender_versions → notices), to learn whether any is r209.
  - Plan-probe locally first. Record whether v_awards' buyer for 179248 flips at a refold, and whether 546 U3 reaches
    the 6.
- **U4: dissolve-path parity (S-M).** Status: ready-for-agent. Blocked by: nothing for code. Its deploy rides 578 U2's:
  539's drain finished, and 551 C7 re-gates both twins on the deployed base. It is its own commit, not part of D578-1.
  - In guid_twins.rs step 4, push every touched Tender into `refold` (requeue its notices and `stamp_tenders_stale`).
  - Do the same for the parties and bid-parties Tenders, not just the winner Tenders.
  - Cover partial rehomes, `only.is_some()`, which returns early at canonical.rs:42139-42144. 578 U4's rehome of
    30920766 goes through that path.
  - For 440 (placeholder_dissolve), stamp in the caller the same way.
  - Correct the false comment at :42012-42014.
  - Tests: a fixture wet dissolve (530 Resolver and 440) followed by a refold with the compare on gives
    `tables_rewritten = 0`. No such assertion exists today; guid_twins.rs has no `tables_rewritten` or
    `tenders_verified`.
  - Gate: `ops/check.sh` (it enables `server`); read the `GATE-EXIT=` value.
- **U5: heal the 6 (S).** Status: waiting. Blocked by: U2.
  - The heal is their next natural refold, read against U2's table.
  - An explicit 6-Tender refold additionally needs a tender-id refold kind (none exists) and either a recorded 551
    amendment exempting it from the standing rule, or the port diff re-run first.
- **U6: file the merge-side follow-up (S, docs only).** Status: ready-for-agent. Blocked by: nothing. Announce the next
  free number first.
  - Shape: "an in-place org repoint (`repoint_org_references`, 8 merge kinds) leaves party, bid-party and multi-winner
    order the fold would not write, and stamps nothing."
  - Options: stamp the merge's touched Tenders stale, or the (e) PK-order winner compare under ADR-0017 D1 while
    accepting party reorders as real served changes.
  - Note for 551's U9 watch: order-only corrections can follow any merge, not just these 6 Tenders.
- **Closure:** 579 is DONE when U2 is recorded, U3 is read, U4 is deployed with its test green, and U6's issue exists.
  The 6 then heal through U5 at their own pace, tracked by U2's registrations.

### Interactions

- **551 B6 (path W on): the standing rule.** Rule 12 (551:1239-1242) lists winners and org merges among path-W inputs.
  The U8 PASS / U9 bullet (551:1554-1555) names **"579's refold"** explicitly: no lot-key or W-input drain until the port
  diff is re-run on the served lot-sum-awards set.
  - The measurer's argument that the rule is "not triggered" is a proposed exemption, not the rule. It rests on the 6
    being outside the lot-sum cohort: lc01 named all 6 and admitted none, and refold-lot-sum's cohort is a superset of
    path W's reach. The refold also moves no org id.
  - Until 551 records that amendment, any explicit refold of the 6 waits for the port-diff re-run.
  - U4's deploy moves nothing by itself. Every later dissolve run, though, is an org-identity drain under the rule
    already: a 440 run, 578 U4's rehome, or a re-run of dissolve-guid-twins. With U4 in place, that drain's own
    `project` refolds its touched Tenders too.
  - U9 noise is negligible: the 6 cannot produce lot-sum-awards heads or band rows.
- **578.** Same function and same gate and deploy. U4 lands beside 578 U2 (canonical.rs ~41198-42251; no textual
  conflict with D578-1's lookup edit beyond the shared body) but as its own unit and commit. 578 U4's rehome of 30920766
  goes through the `only` path, so U4 must cover it. 440 has the same winner re-insert shape, so its future runs also
  land at the refold fixed point once U4 ships.
- **546.** U3 is deployed and U4 is held. A refold of the 6 applies 546 U3 where their chains hold an r209 notice (U3
  here). 546 U4's refold may be the natural heal, and U2 pre-registers them there.
- **539.** Not involved: its drain is DÖE-only.
- **530 (DONE).** Its bindings are confirmed. Its Build 2 review asked for rebuild parity, and U4's test is that check.

### What the skeptics corrected (each checked before it was recorded)

- **Line numbers.** The canonical.rs citations were from c27c8bf, about 30 lines low. They are re-cited at HEAD here.
  The read.rs, project.rs and supervisor.rs numbers were right.
- **Served set: 4 Tenders became 2.** REST serves only the head version, so 340956 and 823340 (non-head inversions)
  serve nothing different. 413501 and 179248 do, and for them the correction is a real served change.
- **"/v1/sql promises no order" is wrong for v_awards.** Its representative buyer is a rowid-order `LIMIT 1`. It is
  unchecked for 179248, which goes to U3.
- **The rule L expectation was too uniform.** Head lots come only from head differences, and every lot of a differing
  table comes at any version. 340956 and 823340 expect few or no lot rows.
- **"551's rule is not triggered" contradicts 551's own text,** which names 579's refold. Recorded as needing an
  amendment or the port diff.
- **"No value moved" was unproven for a refold now,** because 546 U3 is deployed and undrained and all 6 are TED.
- **"Only the dissolve path has this shape" holds for the winner re-insert only.** Merges leave the party and
  multi-winner drift too. U6 files it.
- **The 539 registration is unneeded,** because its drain is DÖE-only. Dropped.
- **(d) alone is complete for the dissolve path; (c) is optional.** (d) is its own unit and is not folded into D578-1.
- **(d) must cover partial rehomes** (the `only.is_some()` early return) and must stamp party and bid-party Tenders as
  well as winner Tenders.
- **The dissolve body's comment at :42012-42014 misnames its shape.**
- **Row-set parity is reasoned from code, not measured,** and is stated so. **lc01's 2 unattributed tables** remain a
  stated limit.

**551 W-on drain protocol (2026-10-23, `551-lot-sum/b6/decision-w-on-drains.md`).** 579 is NOT a drain (amendment W1):
Decisions 2-3 stand, so nothing refolds S7 and nothing explicitly refolds the 6. U2 registers the 6 as order-only `tender
changed` NOTEs in 551's gd5 check and in 546 U4's expectations. U4 rides the batched deploy (576 U2 + 578 U2 + …) as its own
commit, with an entry in 551's code-drift ledger. It is not a fold change, so P1 covers it. Once U4 is in, every later
dissolve run refolds its touched Tenders; such runs are class I under 551's C9' and wait until after 11-04. The orderings "579
before 576/577" and "S7 ∩ DÖE before 539" are dropped. The exemption the Status asked for is now in 551's C9': the 6 heal
naturally inside any drain's gd5. An explicit refold of them would be class W, small, with the discharge "outside the cohort
(lc01 admitted none), and the cohort SQL reads no org id", predicting 0 NEW and 0 served moves. No such refold is scheduled.
**Next step:** unchanged (U2, U3, U4). U3's read stays off U9 check mornings and inside the 150/h budget.
