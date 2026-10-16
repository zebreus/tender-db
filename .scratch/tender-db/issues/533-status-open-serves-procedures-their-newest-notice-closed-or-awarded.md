# 533 — `status=open` serves procedures their newest notice closed or awarded: a no-winner result or UK12 termination inherits the contract notice's deadline, and DÖE sdk-0.1 award notices publish one of their own

Status: ready-for-agent — unit 2 DEPLOYED `a7acb4d` 2026-10-16 05:10 UTC (gate green at `c3c1a7c`, 1281 s; built wf_9ed3c6cf-ebc, two adversarial panels with 11 + 10 findings fixed, a read-only third look with 5 minor findings, follow-ups riding the next gate). The open head is refolded: dry 2534 counted 41,733 Tenders / 64,472 notices, wet 2535 stamped 41,733, and project 2536 wrote 41,733 (39,443 verified + 2,290 corrected, 7,269 correction rows). The **Verify reads done**: `[]`, `[]`, and the newest-notice description. All 14 exemplars read as decided. Next: the three field refolds (sizing 2537/2539/2541 queued), then unit 3 (the per-lot leg, measurement) and unit 4's remaining docs.
Kind: API semantics / data quality (read layer: `status` on `/v1/tenders` and `/v1/lots`, SSE and webhook filters; the fold's head deadline)
Relates to:
- 273 (RESOLVED): `status` is a range on the indexed head column `t.current_deadline`; a fix must stay one.
- 216 (RESOLVED): `current_deadline` and `sort=deadline` ("closes soon", paired with `status=open`); job 706 is the
  backfill precedent.
- 370 (DONE): carry-forward documented; its "deadline < published_at" marker cannot see a result published BEFORE
  the carried deadline.
- 389 / 422 / 424 (DONE): per-lot `status` reads the same carried dates, so these tenders' lots are open too.
- 366 / 474 (DONE): the deadline window, one home in `canonical::deadline_admitted`. It bounds typos, not results.
- 375 (DONE): a backfill must not carry a second head election.
- 257 (CLOSED): sdk-0.1 award notices keep an unstated decision NULL by design, so a `clos-nw` rule misses DÖE.
- 29 (VERIFIED): `project.rs:424` maps SDK01 `…TenderSubmissionDeadlinePeriod-EndDate` to `submission_deadline`,
  award notices included.
- 510 / 512 / 525 (done), 521 (ready-for-agent, low): the no-award markers that make a result read `clos-nw`;
  inputs to a fix, not this defect. 386 (done): FTS OCID welds, relevant only to unit 3's multi-round shapes.
- None covers a result or termination that closes a procedure before its deadline, and no board issue or earlier
  review (2026-09-07, 09-15, 09-27) records a decision against it.

## Verify

    curl -s "https://tenders.zebreus.click/v1/tenders?publication_id=00684240-2026&status=open&limit=1" "https://tenders.zebreus.click/v1/tenders?publication_id=25892804-1&status=open&limit=1" "https://tenders.zebreus.click/v1/openapi.json" | jq -c 'if .components then .components.parameters.status.description else [.items[].id] end'

It prints three lines: the carry-forward exemplar (8805808, publication 00684240-2026), the DÖE exemplar
(8831547), and the served `status` description. Lines 1–2 read unit 2's result and line 3 reads unit 4's. Unit 3 is
a measurement.
- **done**: `[]`, `[]`, and a description that names what closes a Tender besides its deadline. 8805808 is a one-shot
  open procedure (`Sanierung Neu- und Ausbau Stiegstrasse und Stiegweg - Ingenieurleistung`): its CAN 47360037 decides
  its one lot `clos-nw` (BT-144 `other`, 0 tenders, BT-300 `Begründung des Abbruchs: … Formfehler`), its window is 97
  days, and it declares no DPS, no `oth-*` procedure and no scheme subtype, so no exemption of the re-made 1(a) keeps
  it open. Its deadline, 2027-01-07, is the latest of any closing eForms one-shot, so `[]` proves the flag rather
  than a passed date: **read it before 2027-01-07**. Fallback 7940050 (publication 00598729-2026). (The first Verify
  line used 841082, which step 0a found declares `dps-list` on its head: a DPS, which K1 keeps open.)
- **done (2026-10-16 05:2x UTC, after `a7acb4d` and refold-open-head 2535/2536)**: `[]`, `[]`, and `"By submission deadline, and by what the newest notice says. …"`.
- **open (re-made decision, 2026-10-15)**: `[8805808]`, `[8831547]`, `"By submission deadline."` (2026-10-14 16:04
  UTC with the old line 1: `[841082]`, `[8831547]`, `"By submission deadline."`)

## Unit 2 deployed (2026-10-16)

- **Deploy.** `a7acb4d` (533 unit 2 `c3c1a7c` plus 550's issue commit) at 05:10 UTC. Gate green at `c3c1a7c`
  (151 suites, 1281 s). Health ok.
- **Open-head refold.** It ran first, so every daily queues behind it. That closes the SSE window the third look
  found.
  - Dry 2534: 41,733 Tenders with a head deadline after now; 64,472 notices.
  - Wet 2535: re-queued 64,472 and stamped 41,733, equal to the dry count.
  - `project` 2536: 41,733 Tenders written. Compare: 39,443 verified + 2,290 corrected = 41,733 (D7's completeness
    holds). 7,269 correction rows. 60.5 s of reads.
- **Exemplars** (`tenders.current_closed_by_notice`, a PK read):
  - closed (1): 38279, 66939, 6739006, 7422682, 8805808;
  - open (NULL): 403644, 445631, 336277, 634469, 179459, 841082, 913640, 7207004, 8483433.

  Each reads exactly as the re-made decision's exemplars say.
- **Verify:** `[]`, `[]`, and the served description "By submission deadline, and by what the newest notice says …".
- **Follow-ups riding the next gate** (the read-only third look, wf_71cd7d33-729, 5 minor findings, all confirmed):
  - the Dates caveat and the `Status::Closed` doc still describe the single-result shape;
  - "every lot" should be "every lot of kind `Lot`";
  - the `current_closed_by_notice` note's two self-contradicting sentences;
  - the docs test pins fewer phrases than its comment claims;
  - the post-deploy SSE window, now in the runbook with a NULL-flag fixture.

  Applied in the working tree; they gate with 524 v10.

## Repro

Public GETs, 2026-10-14 16:04 UTC:
- `/v1/lots?tender=403644&status=open&limit=5` gives `[1108460]`, so the lots surface is affected.
- `/v1/tenders?publication_id=00043118-2025&status=closed&limit=1` gives `[]`.

The mechanism-1 cohort read is a data-page read (gated per `prod-box-reads.md`; never the Verify line). Substitute
the current epoch for `<now>`. Plan read locally 2026-10-14: `SEARCH t USING INDEX tenders_current_deadline`, every
subquery a `(tender_id, seq)` or PK seek. It returned 178 rows at now=1791990321.

    echo "SELECT t.id, t.source, v.notice_subtype, v.publication_id, t.current_deadline FROM tenders t CROSS JOIN tender_versions v ON v.tender_id = t.id AND v.seq = t.current_seq WHERE t.current_deadline > <now> AND EXISTS (SELECT 1 FROM tender_version_lot_results r CROSS JOIN lot_results lr ON lr.id = r.lot_result_id WHERE r.tender_id = t.id AND r.seq = t.current_seq AND lr.notice_id = v.caused_by_notice_id) AND NOT EXISTS (SELECT 1 FROM tender_version_lot_results r WHERE r.tender_id = t.id AND r.seq = t.current_seq AND (r.decision IS NULL OR r.decision <> 'clos-nw')) AND NOT EXISTS (SELECT 1 FROM tender_version_lots l WHERE l.tender_id = t.id AND l.seq = t.current_seq AND l.kind = 'Lot' AND NOT EXISTS (SELECT 1 FROM tender_version_lot_results r WHERE r.tender_id = t.id AND r.seq = t.current_seq AND r.lot_id = l.lot_id)) ORDER BY t.current_deadline DESC LIMIT 500" | ssh root@zebreus.click /root/sq.sh

It does not see mechanism 2 (DÖE, decision NULL); those 141 need their own read.

## What was found

`status=open` is `t.current_deadline > now` (`crates/store/src/read.rs:1123-1133`, `StatusBy::Head`). The fold writes
`current_deadline` from `canonical::head_deadline` (`canonical.rs:3109`), which is the MAX admitted
`submission_deadline` over the head version's tender and lot facts. Two mechanisms put a live date on a finished
procedure:
1. **Carry-forward.** These result notices publish no deadline: TED eForms 29/30/32/33/E4, legacy TED results, FTS
   UK12 (procurement termination) and FTS award+contract. The head version carries forward every fact the newest
   notice is silent about (the /docs Dates caveat, 370). So it inherits the contract notice's deadline and stays
   open until that date passes.
2. **Own deadline (DÖE sdk-0.1).** A `can-standard` award notice publishes its own
   `TenderSubmissionDeadlinePeriod-EndDate`, and `project.rs:424` maps that to `submission_deadline`. The tender has a
   single version, headed by the award, with decision NULL (257).

The code assumes neither exists: `read.rs:1124` ("award notices … Closed … it cannot be bid on"), `Status::Closed`
(`read.rs:162-166`, "award notices do not" carry one) and `head_deadline`'s doc ("`None` … (award notices)").
`fts/checklist.rs:116` ignores OCDS `tender.status` because status "is derived from its notices and deadlines"; the
notices half is not implemented. The public contract, "By submission deadline." (`openapi.json:755`, `docs.rs:172`),
is met to the letter.

**Counts** (the verifier's bounded reads, 2026-10-14 15:05 UTC, now=1791990321; the refute re-ran the 178 by bucket
at 15:40 and read the 141; nobody re-measured 42,062, 441/353/88 or the 175):

| cohort | tenders |
| --- | --- |
| open (`current_deadline > now`, indexed count) | 42,062 |
| head version carries any lot_results row | 684 |
| … every lot decided (`selec-w`/`clos-nw`, no `open-nw`) | 441 |
| … … newest notice is a CN or tenderUpdate after an earlier result (a re-launch; inferred from head subtype only) | 88 |
| … … newest notice is the result itself | 353 |
| … … … all `clos-nw`, no winner (mechanism 1) | **178** |
| … … … at least one `selec-w` (unit 3) | 175 |
| DÖE single-version sdk-0.1 award head, decision NULL, its own deadline (mechanism 2) | **141** |
| unclassified: partial, `open-nw` or null-decision head results outside DÖE (684 − 441 − 141) | ~102 |

- **The 178 by source:** TED 29: 126, legacy TED: 12, 33: 7, 30: 4, 32: 2, E4: 1; FTS UK12: 18, FTS award+contract: 8.
- **The 178 by deadline, measured from now:**
  - 128 are ≤30 days out: they leave `open` within a month on their own, but they top "closes soon". 38279's
    result was published 2026-06-30, four months before its 2026-10-29 deadline, and it was served open throughout.
  - 16 are 30 days to a year out; 5 are open-ended schemes (445631 ДСП, 336277 DAS, 634469 and 913640 OH-Rabatt,
    179459 a non-exclusive Rahmenvertrag).
  - 34 are more than a year out: about 24 clearly open-ended and about 5 probably, by title (DPS/DAS, SGB V §130a(8)
    open-house, LOV/auktorisationssystem, qualification systems, an e-marketplace); 2 explicit cancellations; 2
    implausible deadlines (all four in the table below).
- **The 141 DÖE:** all seq 1, decision NULL, deadline 23 to 239 days after publication (median 184). They were
  published 2026-04 to 2026-10, so new ones keep arriving. The raw content was read for 2 of 2 (below). No open
  DÖE tender has a `clos-nw` or `selec-w` head result, so mechanism 1 is TED and FTS only.
- **The 175 with a winner:** 171 have the deadline more than 60 days after the award, and 112 more than a year out.
  That fits DPS, framework, dynamic-market (UK14/UK15), qualification-system or multi-round shapes, where `open` can
  be right. 458370 has 34 versions: LOT-0001 was re-tendered 2026-09-25/28 and awarded 2026-10-01.

**Exemplars** (public reads, 2026-10-14 16:04 UTC; all served by `status=open`):

| tender | head notice | result | deadline | role |
| --- | --- | --- | --- | --- |
| 841082 | TED 29 00043118-2025, after CNs of 2024-12-09 and 2024-12-31; restricted | 6 lots `clos-nw`, reason `other` | 2029-12-11 | plain cancellation, Verify line 1; deadline 5 years after a restricted CN |
| 8831547 | DÖE 25892804-1, 2026-10-05, single version ("Trockenbauarbeiten") | decision NULL | 2027-04-30, its own | mechanism 2, Verify line 2 |
| 1706547 | DÖE 25232038-1, 2026-04-15 (notice 26924865: `can-standard`, AwardDate 2026-04-15) | NULL | 2026-10-15, its own | leaves `open` tonight |
| 38279 | TED 29 00447924-2026, 2026-06-30 | `clos-nw`, `other` | 2026-10-29 | the review's example |
| 66939 | TED 29 00704410-2026, 6 versions | `clos-nw`, `chan-need` | 2026-10-30 | the review's example |
| 8576007 | FTS UK12 095413-2026, after UK4 088907-2026 | `clos-nw` | 2026-10-20 | termination |
| 6739006 | legacy TED 438170-2022, "Aufhebung des Open-House-Verfahrens …" | `clos-nw` | 2030-02-28 | explicit cancellation of a scheme |
| 7422682 | legacy TED 366281-2022, "CANCELLED Dynamic purchasing system …" | `clos-nw` | 2028-07-17 | explicit cancellation of a scheme |
| 7207004 | legacy TED 641439-2022, restricted | 2 results, `clos-nw` | 2029-07-31 (= duration_end) | implausible deadline |
| 403644 | TED 29 00059850-2025, Kalmar auktorisationssystem, `oth-single` | `clos-nw`, `other` | 2034-12-30 | **counter-example:** the date is the scheme's admission window, and `open` may be right |

**Surfaces affected:** `/v1/tenders?status=`, `/v1/lots?status=` (403644 gives lot 1108460), `sort=deadline` with
`status=open`, and SSE and webhook subscriptions, which share `Filter`.

### What the external review got wrong

- "3,660 open tenders": it was 42,062 on 2026-10-14. The 3,660 does not reproduce and was probably a subset.
- "98 have a result deciding every lot": it is 441, and that mixes two groups. 88 are re-launches, correctly open
  (inferred from the head subtype). For 353, the result itself is the newest notice.
- "33 closed with no winner": it is 178, or 206 counting re-launches. The 178 include 18 FTS UK12 terminations and
  8 FTS award+contract notices, so this is not TED only.
- "Finished procedures": this holds only in part. About 29 to 34 of the 178 are open-ended schemes, and most of the
  175 with a winner are DPS/framework shapes.
- It missed mechanism 2 entirely: the 141 DÖE award notices with their own deadlines.
- "Status only looks at the deadline" is accurate, and documented. The review missed why the deadline is there.

Nor re-file the first verification draft's errors: "a result notice publishes no deadline" (false for DÖE
sdk-0.1); "128 ≤30 days out = cancellations shortly before the deadline" (counted from now, not from the result);
403644 as a cancellation or a Verify exemplar.

## Decision (unit 1, 2026-10-15)

Decided by a judge panel (independent proposals from distinct angles, then a skeptical judge that re-checked every disputed code claim against the tree at f3d5d06).

**Verdict.**
- **1(a)** An all-`clos-nw` result closes a Tender when the head notice itself published it and it decides every Lot. The exception is a Tender that declares an open-ended scheme:
  - a DPS (BT-766, projected by sub-unit 2a);
  - procedure `oth-single` or `oth-mult`;
  - a chain that carries subtype `15` or `UK13`–`UK16`.
  UK12 gets no rule of its own. It closes through its own `cancelled` awards.
- **1(b)** On a DÖE sdk-0.1 `can-standard` notice, `TenderSubmissionDeadlinePeriod-EndDate` stops mapping to `submission_deadline` (`project.rs:424`). The 141 then have no head deadline, so today's range reads them Closed. No status rule is added for a NULL decision.
- **1(c)** Option B: an unindexed, fold-maintained `tenders.current_closed_by_notice`, checked on each row of the unchanged `current_deadline` range. The SSE `Scope::At` probe decides status per version. Lots get a per-lot term.
- The rule never reads a title, a deadline's distance from publication, `now`, or `tenders.source`.

Stale references in the issue body:
- `read.rs:1123-1133` is now `read.rs:1222-1233`.
- `read.rs:1124` is now `1223-1225`.
- `read.rs:162-166` is now `159-166`.
- `read.rs:1903` is `tenders_ordered_query` (`read.rs:2214`).
- `canonical.rs:3109` is now `3136`.
- `docs.rs:172` is now `crates/app/src/v1/docs.rs:175`.
- `openapi.json:755` is `crates/app/data/openapi.json:755`.

### Code claims re-checked where the proposals disagreed
- **SSE drops a fold-time close today.** `tenders_query` passes `StatusBy::Head("t.current_deadline")` for `Scope::At` too (`read.rs:2831`). The seq and seq−1 probes (`crates/app/src/v1/sse.rs:420-432`) therefore both read the head. A result that closes the Tender reads closed on both sides and hits `(false, false) => continue` (`sse.rs:465`). A versioned change row never produces `removed`.
- **Webhooks have no Filter.** `CreateRequest` is `{ url }` (`crates/app/src/v1/webhooks.rs:26-29`), and delivery sends every public change row (`crates/app/src/webhooks.rs:481-490`). The issue's "webhooks share `Filter`" is wrong. Unit 2's webhook check is: a public `tender`/`lot` change row exists for each flip.
- **A lot change row needs a fact or kind change.** A lot is `changed` only when its facts or kind differ (`canonical.rs:39549-39553`). A result that closes a lot is otherwise silent. A Tender is `changed` on facts, rounds or group_members (`39528-39535`).
- **Every head index is deferred.** `tenders_current_deadline` and `tenders_source_deadline` are in `DEFERRED_TENDER_INDEXES` (`canonical.rs:14970`, `15054`, `15105`).
- **The flag's residual costs nothing.** Every Tenders window already reads the `tenders` row, because each joins `tender_versions` on `t.current_seq` (`read.rs:2249`, `2852`, `2926`).
- **The lot seed must not change.** Bare `status=open`'s open-head lot seed is `SELECT t.id FROM tenders t WHERE t.current_deadline > ?` (`read.rs:3869-3884`), a covering read of `(current_deadline, id)`.
- **The DÖE date can be dropped at the mapping.**
  - `NoticeState::read` already knows `sdk01` (`project.rs:5171`). `first_code` exists (`project.rs:10192`), and `SDK01-NoticeTypeCode` is parsed (`project.rs:1103`).
  - The F14 precedent drops non-deadline dates at the mapping (`project.rs:428-466`).
  - Neither committed `can-standard` fixture carries the date. Both `cn-standard` fixtures do (`crates/ingest/tests/fixtures/doe/sdk-0.1-*.xml`).
  - `head_election_agreement.rs` counts `submission_deadline` selectors in store/app code only (`:580-596`). The ingest mapping is outside it, so the claim that dropping the date breaks the head election is false.
- **Supersession is per field key** (`canonical.rs:3117-3125`). A later notice's date therefore replaces the CN's.
- **Rounds carry their own notice's id** (`project.rs:5799-5800`, `7297`, and `caused_by_notice_id` at `6539`). A correction replaces the round it corrects (`project.rs:6509-6515`).
- **The stored layer can't feed the fold, which rules out R2.** `stored_version` returns raw leaf rows (`canonical.rs:21611`), and no loader rebuilds a `TenderVersion` from stored rows.
- **No `.rs` file reads BT-765/766.** BT-766 is a `code` field, so it lands in `notice_codes` (`eforms/sdk.rs:194-222`). It appears in Rust only in comments (`project.rs:1064`, `1067`). The per-dialect ids are:
  - eForms (and eForms-DE 2.x): `BT-766-Lot`, `BT-766-Part`;
  - eForms-DE 1.x: `DE1-ProcurementProjectLot-TenderingProcess-ContractingSystem-ContractingSystemTypeCode`;
  - DÖE: `SDK01-ProcurementProjectLot-TenderingProcess-ContractingSystem-ContractingSystemTypeCode`.
- **REST serves classifications by named scheme only** (cpv, nuts, procedure: `read.rs:2401-2418`). A new `dps` scheme changes no REST payload.
- **Legacy decisions can be inferred.** A legacy award block with no evidence at all reads `clos-nw` by inference (`project.rs:6990-6992`). Legacy procedure codes never map to `oth-*`.

### 1(a) The closing rule
Notation: the version prefix is `C = versions[..=n]` and the head is `H = C.last()`. Reading only `C` keeps ADR-0017 E5.

**Definitions:**
- **`own(H)`:** the lot results of the rounds whose `notice_id == H.caused_by_notice_id`.
- **`all(H)`:** the lot results of every round of H.
- **`covered(H)`:** every lot of H with `kind == "Lot"` has a result in `all(H)` with the same `lot_key`. In SQL that is the same `lot_id` (`result_lot`, `canonical.rs:39384-39395`). A version with no Lot-kind lot counts as covered. A Tender-scoped result (`lot_key` None) covers no lot. This is the Repro read's third clause, unchanged.
- **`exempt(C)`** holds when any of these holds:
  - **K1 (DPS):** a `Fact::Classification{scheme:"dps"}` (code `dps-list` or `dps-nlist`) is in `H.facts` or in any `H.lots[].facts`. Sub-unit 2a produces this fact.
  - **K2 (open procedure):** H's Tender-scope `procedure` classification (`project.rs:5256-5262`; SQL `scheme='procedure' AND lot_id IS NULL`, `read.rs:1296`) is `oth-single` or `oth-mult` (`project.rs:7732-7733`).
  - **E2 (scheme subtype):** some version in C has a trimmed `notice_subtype` of `15` (qualification system, `project.rs:1043-1044`) or `UK13`–`UK16` (dynamic market, `docs/research/uk-fts.md:119`).

**`closed_by_notice(C) = !exempt(C) && own(H) is non-empty && every r in all(H) has decision == Some("clos-nw") && covered(H)`**

- A NULL decision never counts as `clos-nw` (issue 257).
- Any `selec-w`, `open-nw` or NULL in any round keeps the Tender open. The 175 with a winner stay for unit 3.
- **Re-launch:** a head CN or tenderUpdate has an empty `own(H)`, so it reads open by construction. This covers the 88.
- **UK12:** `cancelled` and `unsuccessful` awards map to `clos-nw` (`fts/parse.rs:303-309`).
  - A UK12 that cancels every lot closes through the rule. Fixture 083256-2026 does: one lot, award `cancelled`, `relatedLots ["1"]`.
  - A UK12 that cancels some lots stays open.
  - A UK12 with no awards of its own stays open. That goes to unit 3.
- **Legacy TED:** all-`clos-nw` heads close, including the inferred `clos-nw` of an evidence-free award block. Step 0's legacy stop rule guards this.

**Lot rule** (per-lot `status`). Lot L of H is closed when either:
- the Tender is closed; or
- `!exempt(C)`, and `own(H)` has a `clos-nw` result for L, and every result for L in `all(H)` is `clos-nw`.

**Not rule inputs, and why:**
- **Title.** Never.
- **Deadline distance from publication.** It cannot separate 841082 (restricted, 5 years out) from a scheme, and 7207004's date is its duration end.
- **BT-765 framework.** A framework is concluded by one competition, so an all-`clos-nw` result means that competition failed.
- **BT-144 reason.** 841082 and 403644 both read `other`.
- **FTS `tender.status` and `tender.techniques`, BT-119-LotResult, and the legacy DPS/TERMINATION_DPS/PROCUREMENT_DISCONTINUED markers.** None of them is projected; they go to unit 3.

**Expected outcomes:**
- 841082, 38279 and 66939 close. 8576007 closes, or has already closed if its 2026-10-20 deadline passes before deploy.
- 6739006, 7422682 and 7207004 close, unless step 0's legacy stop rule fires.
- 403644 (`oth-single`) and its lot 1108460 stay open.
- 445631, 336277, 634469, 913640 and 179459 stay open only through K1, K2 or E2. Step 0 checks this.
- 8831547 and 1706547 close through 1(b).

### Sub-unit 2a: project the DPS declaration (BT-766 only)
- Add `contracting_system_dps(field_id, code) -> Option<&'static str>` in `project.rs`, beside `contract_nature` (`project.rs:7646`).
  - It trims and lowercases the code.
  - It reads four ids: `BT-766-Lot`, `BT-766-Part`, `DE1-ProcurementProjectLot-TenderingProcess-ContractingSystem-ContractingSystemTypeCode` and `SDK01-ProcurementProjectLot-TenderingProcess-ContractingSystem-ContractingSystemTypeCode`.
  - `dps-list` and `dps-nlist` return the code.
  - `none`, any `fa-*` code and unknown codes return nothing.
- Emit `Fact::Classification{field:"dps", scheme:"dps", code}` at the published Lot or Part scope. Supersession per key carries it.
- A later `none` emits nothing, so a declaration never clears. That errs toward open.
- Add the four ids to the `Channel::Code` destination predicate (`project.rs:7509-7520`).
- Add `dps` to the sandbox scheme and field notes (`crates/app/src/v1/sql.rs:1032-1039`).
- Do not project BT-765. It is not a rule input, and unit 3 reads it from `notice_codes`.

### 1(b): drop the award notice's date in the mapping
**The change.** At the Date arm of `NoticeState::read` (`project.rs:5397-5417`), when `sdk01` holds and `first_code(parsed, "SDK01-NoticeTypeCode")` is in `pub const SDK01_AWARD_NOTICE_TYPES: &[&str] = &["can-standard"]`, the SDK01 deadline field maps to `None`.
- `data_quality.rs:363` uses the same const in place of its literal.
- `cn-standard`, `pin-only` and any unlisted type keep today's mapping, visibly.
- The date stays in the notice layer, and `/v1/notices/{id}/content` still serves it.

**Why the mapping, and not `deadline_admitted`, `head_deadline` or the flag:**
1. **Only the mapping knows which notice a date came from.**
   - `Fact::Date` carries no provenance (`canonical.rs:3077`).
   - `deadline_admitted` sees only `(utc, published_at)` (`canonical.rs:4078`). The 474 window stays pure.
   - `head_deadline` sees merged facts (`canonical.rs:3136`).
2. **A flag would keep serving the wrong date.** Supersession is per key, so in a chain the award's date replaces the CN's real deadline. A flag would close the Tender but keep serving that date. Dropping it lets the CN's deadline carry forward.
3. **The date is not a submission deadline on any reading.** It sits 23 to 239 days after the award notice, on all 141. The committed award fixtures do not carry it.
4. **F14 precedent** (issue 385): "a missing correction is visible as a stale date, an invented one is not."
5. **One edit fixes every surface:** the served `submission_deadline`, `sort=deadline`, `deadline_after`/`_before`, the lot row and `status`. `current_deadline` becomes NULL, which is Closed under `read.rs:1226-1232`.

The raw reads are a precondition with a stop rule (step 0c), not a unit-4 caveat. NULL-decision award heads outside sdk-0.1 stay in the ~102 for unit 3.

### 1(c): Option B
**Column.** `tenders.current_closed_by_notice INTEGER`. It is 1 when `closed_by_notice` holds at the head and NULL otherwise, never 0 (the `is_buyer` convention). It has no index: one would offer the planner an `IS NULL` seek over ~7.9M rows.

**Head-scoped Tenders reads** (`StatusBy::Head` at `read.rs:2276`, `2831` (Page), `2901`):
- Open: `AND t.current_deadline > ? AND t.current_closed_by_notice IS NULL`
- Closed: `AND (t.current_deadline IS NULL OR t.current_deadline <= ? OR t.current_closed_by_notice IS NOT NULL)`

The open range still seeks `tenders_current_deadline`, or `tenders_source_deadline` under the source pin (`read.rs:2155-2159`). Closed is already an OR.

Unchanged:
- `sort=deadline`, `HeadOrder::Deadline` and `deadline_after`/`_before`, which stay on `t.current_deadline`;
- the source pin and isolation;
- the publication seed. Status rides `version_predicates` after the seed, so Verify line 1 sees the flag.

**SSE probe.** Add `StatusBy::Version`, used by `tenders_query` for `Scope::At` only.
- Open is an EXISTS on `tender_version_dates` at `d.seq = v.seq`, with `field='submission_deadline'`, `> ?`, and the date admitted by `deadline_admitted_sql` against `v.published_at`. That is ANDed with `NOT closed_by_notice_sql("t.id","v.seq")`.
- Closed is its negation.
- Build the deadline EXISTS through one helper shared with `StatusBy::Lot` (`read.rs:1249-1266`). Then `the_deadline_window_is_written_once` (`head_election_agreement.rs:580`, six selectors) stays at six. If a line is added anyway, change the count in the same commit.

**Lots.** `StatusBy::Lot` gains a `tender_closed` expression.
- Open is `<424 EXISTS> AND NOT (<tender_closed>) AND NOT <lot_closed_by_result_sql(tid, seq, lot)>`. Closed is its negation.
- `lots_stream_predicates` (`read.rs:3942`): `(SELECT tt.current_closed_by_notice FROM tenders tt WHERE tt.id = l.tender_id) IS NOT NULL`. This is the same PK seek its value bound already pays.
- `lots_query_previous` (`read.rs:3742`): `t.current_closed_by_notice IS NOT NULL` for Page, and `closed_by_notice_sql("t.id","v.seq")` for At.
- Never use `closed_by_notice_sql` in a Page read. Its coverage leg, evaluated per lot, is issue 115's quadratic shape.
- `lot_seed_predicates` is unchanged. It is still a superset, and it keeps bare `status=open`'s seed a covering read.
- **Perf bound.** On a fixture with 2,400 lots and 2,400 head results, `lots?tender=X&status=open` must stay under 2× its pre-change time. If it does not, ship without `lot_closed_by_result` (lots then close only with their Tender) and move the per-lot term to unit 3. No re-decision is needed.

**Why not Option A** (`current_open_until` with its own index):
- `sort=deadline`, the deadline bounds and the 503 source pin still need `current_deadline`. Option A would emit both ranges, and its new index would serve nothing.
- Its index is deferred, so until a `Reindex` a bare range over it walks 7.9M rows. That is the 273 DoS again.
- Its NULL default disagrees with the fold on every Tender with a deadline. That means either a 7.92M backfill (job 706's size) or a correction on every later refold. The flag's NULL default agrees wherever the rule does not fire.

### One home, a Rust form and an SQL form
In `canonical.rs`, beside `deadline_admitted` (`4078-4093`):
- Functions: `closed_by_notice(chain: &[TenderVersion]) -> bool`, `lot_closed_by_result(chain, lot_key) -> bool`, `closed_by_notice_sql(tid, seq) -> String` and `lot_closed_by_result_sql(tid, seq, lot) -> String`.
- Constants: `clos-nw`, `OPEN_SCHEME_PROCEDURES`, `OPEN_SCHEME_SUBTYPES` (`15`, `UK13`–`UK16`) and `DPS_CODES`.

The SQL forms read only the version's own rows, each by a `(tender_id, seq)`-prefixed or PK seek:
- `tender_versions`: `caused_by_notice_id`, plus `notice_subtype` for `seq <= v`;
- `tender_version_lot_results` joined to `lot_results`: `decision`, `lot_id`, `notice_id`;
- `tender_version_lots`: `kind`;
- `tender_version_classifications`: `procedure` with `lot_id IS NULL`, and `dps`.

**Agreement test** (the 474 pattern), through `Db::apply_tenders` over every branch fixture:
- per version, the SQL form equals the Rust form;
- per Tender, the stored flag equals the SQL form at `current_seq`;
- per lot, the two lot forms agree.

**Tripwire.** A `clos-nw` literal may appear in store/app code lines only in these helpers. Today the only one is the note at `crates/app/src/v1/sql.rs:1128`.

### Fold and change feed
- `StoredHead` (`canonical.rs:1527-1533`) gains `closed_by_notice`.
  - `fresh` fills it beside `head_deadline` (`canonical.rs:21328`).
  - Both `tender_identity` SELECTs read it.
  - `head_update` writes it (`canonical.rs:41279`; executed near `21350`).
  - `head_equal` then sees it (D1.5).
- `append_version_changes` (`canonical.rs:39506`) gets two more conditions:
  - Tender `changed` also when `closed_by_notice` differs between prev and v;
  - each lot present in both versions `changed` also when its lot closure differs.
- A later result flips the flag in the same transaction as `current_seq`, and SSE classifies it as `removed`. A later re-launch CN clears it, which reads as `added`.

### Rollout: ADR-0017 R3 over the open head
Read answers can change only where `current_deadline > now`, about 42k rows. A lot open on its own deadline implies that, because `head_deadline` is MAX over the version's tender and lot facts.

**Rejected routes:**
- **R2 (in-place backfill).** It is infeasible: no stored-rows→`TenderVersion` loader exists.
- **A job-706 SQL UPDATE.** It would be a second implementation of the rule, which is issue 375's hazard.

**`refold-open-head {dry_run}`** copies `refold-value-band`:
- Cohort: `Db::open_head_tender_ids(now)` = `SELECT id FROM tenders WHERE current_deadline > ?`, beside `value_band_tender_ids` (`canonical.rs:13735`).
- Wet run: `stamp_stale_for_tenders` (`13761`) and `requeue_tender_notices` (`13694`), then `project`.
- Register it at every site `refold-value-band` uses: request arm, the 543 table, dispatch, the `Box::pin` run arm, the heavy-kind list, and a test. `supervisor.rs` is being edited concurrently, so diff before staging.
- No `PROJECTION_EPOCH` bump. Queue it after the running 534 refold.
- D3's rules T and L emit the corrections, and SSE classifies them head against head−1.
- Completeness (D7 R3): the cohort's stale-0 residue is 0, and `verified + corrected == stamped`.

**Then two field refolds:**
- `refold-fields` over the SDK01 date id (dry count first, expect guard), so past-deadline DÖE awards stop serving the date.
- The 2a fill: `refold-fields` over the four contracting-system ids. This is heavy, so take a dry count first and use an expect guard. If it has not run when unit 2's reads pass, file it as its own issue. Until it has run, the sandbox note says `dps` rows are complete for the open head and for Tenders folded since the deploy.

### Unit 2 order
**0. Bounded reads before any code**, gated like the Repro read.

a. **Scheme table for the 178.** For each Tender, record:
   - the head procedure code;
   - every version's `notice_subtype`;
   - the BT-765/BT-766/DE1/SDK01 contracting-system codes on every version's `caused_by_notice_id` (`notice_codes`, PK prefix);
   - for legacy rows, the r209 DPS, TERMINATION_DPS and PROCUREMENT_DISCONTINUED markers;
   - a verdict.

   Stop rules:
   - **Stop and revisit 1(a)** if any of 403644, 445631, 336277, 634469, 913640 or 179459 would close, or if a legacy row that would close carries a DPS marker with no termination or discontinued marker.
   - **If 841082 declares `dps-*`,** it is a DPS. Replace Verify line 1 with a closing one-shot eForms row from the table.

b. **Per-lot population.** Count the open-head lots the per-lot term would close while their Tender stays open, and read 5 of them.

c. **sdk-0.1 award dates.** Read 3 to 5 public `/v1/notices/{id}/content` for sdk-0.1 `can-standard` notices that carry the date, including 25892804-1 and the one with the shortest distance. **Stop 1(b)** if any of them invites tenders or requests to participate until that date.

## Decision 1(a) re-made (2026-10-15, owner, after step 0a; panel wf_18caa669-8aa: two designers, an adversarial judge)

The step-0a table showed the decided exemptions close live schemes (open-house § 130a(8) agreements, an open-house
framework, a DPS declared only on its CN) and two legacy DPS rows. The judge re-ran both proposals' numbers on the table
and read the disputed notices; this adopts the judge's verdict. The core and the lot rule are unchanged; only `exempt`
changes:

    closed_by_notice(C) = !exempt(C) && own(H) non-empty && every r in all(H) is clos-nw && covered(H)
    exempt(C)           = K1(C) || K2(H) || E2(C) || (W(C) && !D(H))

- **K1 (DPS), read over the chain.** `Fact::Classification{scheme:"dps", code ∈ {dps-list, dps-nlist, dps}}` on H or any
  H lot. Facts supersede per key and lots carry by key, and 2a emits nothing for `none`, so H holds every earlier
  version's declaration: 445631 (`dps-list` on its CN, `none` on its result) stays open. Sources: sub-unit 2a's four
  BT-766/DE1/SDK01 ids as decided, PLUS the r209 marker `TED-DPS` as a Tender-scope fact with code `dps` (mirror
  `procedure_marker`; it is a `Rule::Marker` stored as `Integer(1)`, so its destination is **Channel::Integer**). Not
  projected: `TED-TERMINATION_DPS`, `TED-DPS_ADDITIONAL_PURCHASERS`. A declaration never clears.
- **K2, E2:** as decided.
- **W (admission window over a year) — reverses the ban on deadline distance.** `dl(v)` = `head_deadline(v)`; `a` = the
  first version of C with `dl(v)` set; `W(C) := dl(H) set && dl(H) − a.published_at > OPEN_WINDOW_SECS (365 days)`.
  Reads C only (ADR-0017 E5), never now, title or source; the SQL form through the shared deadline helper (or
  `the_deadline_window_is_written_once` updated in the same commit). Both reasons for the old ban are refuted by the
  table: 841082 is a DPS, and 7207004's 2029 date is a DPS notice's receipt deadline, not a duration end. Measured
  windows: one-shots ≤ 122 days (38279 122, 66939 116, 7940050 112, 8805808 97, 8571596 33, 8582368 31); schemes ≥ 439
  (913640 439, 634469 720, 179459 826, the DAK § 130a rows ~1,085).
- **D (an explicit legacy no-award), new sub-unit 2b.** In `read_legacy_results`, `TED-PROCUREMENT_DISCONTINUED` and
  `TED-PROCUREMENT_UNSUCCESSFUL` (Integer markers, Channel::Integer) set `reason` = `discontinued` / `unsuccessful`;
  `D(H) := own(H) non-empty && every r in own(H) has such a reason`. D overrides W only, never K1, K2 or E2. The served
  `reason` gains two legacy-only codes (sql.rs notes).
- **Not rule inputs:** title, now, `tenders.source`; BT-765 `fa-*` (45 closers carry it, 42 dead one-shots); BT-144
  (`other` on ended and live schemes alike); result latency (a candidate for step 0d to measure, not adopted).

**What it trades on the 186** (judge's re-run): 153 close, 33 open. Wrongly closed: 0 measured live schemes (the
decided rule wrongly closed 9 live schemes, 5,321 tender-days, plus 7 ambiguous). Wrongly open: 4 dead procedures
already served open today — 996597 (to 2029-12-31), 255271 (to 2028-03-31), 1003799 (a probable year typo, to
2026-12-10; issue 474's class), 913640 (its CAN `wird aufgehoben`; to 2026-11-30). Ambiguous kept open: 7 (FTS DPS /
e-marketplace rows, 232118).

**Exemplars after:** 403644 open (K2; lot 1108460 too); 445631 open (K1 over the chain); 336277 open (K1, K2); 634469 open
(W 720 d; live — 5 joiners, no termination text); 179459 open (W 826 d); 913640 open (W) though DEAD — an accepted
cost, dropped from the must-stay-open set; 841082 open (K1, a DPS); 7207004 open (K1 legacy leg — v1 is the F02 `DAS
ICT Desktops`, receiving requests to 2029-07-31; added to the stop set); 8483433 open (K1; its F14 extended the DPS to
2026-10-21, so it leaves `open` before any deploy); 6739006 and 7422682 close through D; 38279, 66939, 8805808 close.

**Verify line 1 changes** from 841082 (a DPS) to **8805808**, publication 00684240-2026: a one-shot open procedure
(`Sanierung Neu- und Ausbau Stiegstrasse und Stiegweg - Ingenieurleistung`), its CAN 47360037 one lot `clos-nw`, BT-144
`other`, 0 tenders, BT-300 `Begründung des Abbruchs: … Formfehler`; window 97 days; deadline 2027-01-07, the latest of any
closing eForms one-shot, so `[]` proves the flag rather than a passed date. Read it before 2027-01-07. Fallback 7940050
(00598729-2026). The Verify text's "841082 … a restricted procedure, not an open-ended scheme" is wrong and goes.

**Step 0d (new, before the build):** one bounded read giving each of the 186 its W anchor (`published_at` of the first
deadline-bearing version) and own(H) reasons. Stop if any row read as a live scheme closes, or if W keeps open more than 5
rows that read as one-shot with a window over 400 days. Measure result latency as a candidate only.

**Step 0d — read 2026-10-15 (no stop).** Public reads of every version of the 186 (`533-step0d-2026-10-15.json` beside
this file): each version's own submission deadlines (the DATES ids: `BT-131(d)*`, `TED-DATE_RECEIPT_TENDERS`,
`TED-RECEIPT_LIMIT_DATE`, `TXT-DT/DD`, the SDK01 EndDate; F14 targets not read — they move a deadline, never anchor a
first one), W's anchor, the window, own(H)'s reasons and markers, and the first-notice-to-result latency.
- Windows: ≤ 122 days 141; 123–365 days 3 (1148283 178, 1026899 192, 38279 123 — all one-shot calls, all close); 366–400
  days 1 (1003799 379, the year typo); > 400 days 40; no anchor 1 (8483433, open through K1 anyway).
- The re-made rule: **153 close, 33 open** (the judge's count). W keeps open the FTS DPS / e-marketplace rows, the six DAK
  § 130a(8) open-houses, 634469, 179459, 232118, 7241490 (550's case), and the dead 996597, 255271, 913640, 1003799. D
  closes nine legacy rows despite W: the six with `TED-PROCUREMENT_DISCONTINUED` and 6739006, 7329347, 7329346 (each
  head notice carries `TED-PROCUREMENT_UNSUCCESSFUL` beside `TED-NO_AWARDED_CONTRACT`).
- Stop rules: no row read as a live scheme closes; W keeps open no row that reads as a one-shot with a window over 400
  days (the long-window rows are schemes, ended schemes or the typo). Latency of the closers: median 10.7 days.

**Step 0b — read 2026-10-15.** One bounded read (plan read locally first: `SEARCH t USING INDEX
tenders_current_deadline`, then `(tender_id, seq)` and PK seeks): the open-head Lot-kind lots whose head notice gives a
`clos-nw` result and whose every result is `clos-nw` — 418 lots in 200 Tenders, 186 of them the closing cohort above.
Outside it, the lot term alone would close **27 lots in 14 Tenders** that stay open. Read on the public API with the
re-made exemptions: **11 lots in 7 Tenders close**, all one-shot calls (windows 17–277 days: 517841, 8804700, 8805828,
8579112, 190875, 734185, 8585745 — e.g. 8585745's LOT-0001 `clos-nw` beside five lots with no result); **16 lots in 7
Tenders stay open** through W or K1 — the two `Arzneimittelverträge mit Beitrittsoption` open-houses (1040643 717 days,
893495 682), the DPSes 8460540, 7842081, 8419587 and the DIS 643970 (K1 and W), and 934849 (a one-shot telemedicine
call whose 394-day window keeps its one `clos-nw` lot open beside seven awarded lots — an accepted wrongly-open lot).

**Rule-branch tests to add:** a CN `dps-list` then a CAN `none` keeps K1; `TED-DPS` on a CAN keeps K1; W at 366 days
open and at 364 closes; a deadline-less PIN or UK2 does not anchor W; D overrides W but not K1.

**Filed from this read:** issue 550 — R2.0.9's standard award blocks write `DATE_CONCLUSION_CONTRACT`, which the
legacy reader never reads, so the era has no award date and a winner-silent award (`AWARDED_CONTRACT` with a date, no
contractor) reads `clos-nw` by inference (7241490, the Prüfsystem the judge found closing). Decide 550 before 533's
build: an inferred `clos-nw` on an open head closes the Tender here.

**Step 0a — the scheme table, read 2026-10-15: the STOP RULE FIRED.** The Repro cohort read (plan re-read locally first:
`SEARCH t USING INDEX tenders_current_deadline`, every subquery a `(tender_id, seq)` or PK seek) returned 186 Tenders
(178 on 10-14): TED 160, FTS 26; head subtype 29: 134, UK12: 19, legacy (no subtype) 12, award+contract 7, 33: 7, 30: 5,
32: 2. Each Tender, and each of the six exemplars, was then read on the public API: `procedure_type`, every version's
subtype, and every version's raw codes for BT-765-*, BT-766-* and the DE1/SDK01 `ContractingSystemTypeCode`, plus any
field naming DPS/TERMINATION/DISCONTINUED (the legacy markers). The table is committed beside this file
(`533-scheme-table-2026-10-15.json`; script `scheme_table_533.py` in the session scratchpad). The decided rule over it:
- **Cohort (180):** 172 close; 8 stay open — 7 through K1 (a `dps-*` on the HEAD: 305012, 1110330, 841082, 987397,
  1077484, 149151, 797346) and 705918 through K2 (`oth-single`). E2 fires on none. 43 of the 172 closing rows carry a
  framework code (`fa-wo-rc`, `fa-w-rc`, `fa-mix`) on some version.
- **The six exemplars — STOP (any one closing stops):**
  - 403644 `oth-single` → open (K2) ✓; 336277 → open (K1 and K2) ✓.
  - **445631 closes**: its CN (v1) declares BT-766 `dps-list` (`ДСП …`), its head result (v2, subtype 29) declares
    `none`, and K1 reads the head only.
  - **634469 closes**: `nicht-exklusive Rabattvereinbarung nach § 130a Abs. 8 SGB V`, procedure `open`; BT-765, BT-766
    and the DE1 code all read `none` on every version.
  - **913640 closes**: the same § 130a(8) open-house scheme (`OH-Mod.`); BT-765 `fa-wo-rc`.
  - **179459 closes**: `nicht-exklusiver Rahmenvertrag … im Open-House`; BT-765/DE1 `fa-w-rc`/`fa-mix`.
- **Legacy stop — fires**: 7207004 (`TED-DPS` on v2 and v3; the 2029 date is its duration end) and 8483433 (`TED-DPS`
  on v2) close with no TERMINATION/DISCONTINUED marker. The six legacy closers that carry
  `TED-PROCUREMENT_DISCONTINUED` (7481617, 7383945, 6507010, 6424988, 7422682, 7674485) close rightly.
- **841082 declares `dps-list` on its head**, so it is a DPS and stays open; per the stop rule Verify line 1 needs a
  closing one-shot eForms row from the table instead.

What the re-decision has to weigh: K1 over every version of C (fixes 445631, costs nothing in the cohort: no closing
row declares `dps-*` on an earlier version); the legacy `TED-DPS` marker as K1's legacy leg (7207004, 8483433); and the
open-house class, which no projected code separates — 634469 carries none — while BT-765 `fa-*` (rejected at decision
as a rule input) would keep 43 closing rows open, most of them ordinary frameworks whose one competition failed.

**Step 0c — read 2026-10-15 (stop rule did not fire).** The open DÖE heads with a NULL-decision head result, shortest
deadline gap first (one bounded read, plan read locally first: `SEARCH t USING INDEX tenders_source_deadline (source=?
AND current_deadline>?)`, then `tender_versions` and `tender_version_lot_results` seeks; 8 rows): 8834111 (23 days),
8841643 (79), 7952167, 7975754, 8582264 (91), 1736912, 1738807, 1744631 (92). The raw content of four, each
`NoticeTypeCode` `can-standard` with `TenderResult-AwardDate` equal to its `IssueDate`:
- 47371097 (25892804-1, tender 8831547, Verify line 2): `Trockenbauarbeiten`, award 2026-10-05, EndDate 2027-04-30;
  no invitation text, a Staatsanzeiger document link.
- 47376608 (25902454-1, tender 8834111, the SHORTEST gap, 23 days): `Upgrade und Wartung Atradis` — "Auftragnehmer:
  TCC GmbH … Vergabeart: Verhandlungsvergabe ohne Teilnahmewettbewerb gem. UVgO § 8 Absatz 4 Nr. 14"; EndDate 2026-10-31.
- 47392196 (25918012-1, tender 8841643): "Der Auftrag wird als Verhandlungsvergabe ohne TW nach § 8 Abs. 4 Nr. 17 UVgO
  … vergeben. Name des beauftragten Unternehmens: RPT GmbH"; EndDate 2026-12-31.
- tender 7952167 (25782126-1): `GSM-ESF 2026_27 Einsatz päd. Fachkräfte`, award 2026-09-03, EndDate 2026-12-04; no
  invitation text.

Each is an ex-post award publication of a negotiated procedure WITHOUT a participation contest (UVgO § 30), naming the
contractor where it names anything; none invites tenders or requests to participate until the date. 1(b) stands.

**1. Build 2a and 1(b)** in `project.rs`.

**2. Build the rule and its plumbing:**
- the helpers and constants;
- the column: SCHEMA (`canonical.rs:132`), `reset_tender_layer`'s CREATE (`canonical.rs:15176-15185`), an `add_column` in `migrate()` beside `lib.rs:861`/`988`, and a `COLUMN_NOTES` entry (`sql.rs:1045`, which says that past-deadline rows read NULL until their next refold);
- the fold and the change-row clauses;
- the read path, and the `read.rs:1223-1225` comment.

**3. Tests:**
- **Rule branches:** R1 closes; re-launch stays open; a winner in an earlier round stays open; an uncovered lot stays open; NULL stays open; K1, K2 and E2 each keep it open; a whole-UK12 closes; a partial UK12 stays open; a lotless Tender closes; a `lot_key`-None result beside Lot-kind lots stays open; a correction to `selec-w` reopens.
- **1(b):** sdk-0.1 `can-standard` emits no `submission_deadline` while `cn-standard` keeps it; in a CN→CAN chain the CN's deadline carries forward.
- **Agreement test and tripwire.**
- **Head writers:** add a writer pin for the new column, beside `the_head_columns_have_exactly_one_writer_that_decides_them` (`head_election_agreement.rs:529`).
- **`status_head_range.rs:28`:** update the closed string.
- **Plan tests** with the deferred indexes built: `status=open` by id order, by `sort=deadline` ascending and descending with a cursor, and source-pinned, all still on `tenders_current_deadline`/`tenders_source_deadline` with no new SCAN.
- **Existing plan tests unchanged:** `head_pointer_plan_tests` (`read.rs:5374`) and `lots_open_head_seed.rs` (423's control).
- **Golden:** regenerate `fold_writer_golden.rs` deliberately (`GOLDEN_CAPTURE_495=1`).
- **`api.rs` and SSE:** a result notice gives `removed` on a `status=open` tape; a re-launch gives `added`; `lots?status=open` drops the lots.
- **Supervisor:** a dry and wet test for `refold-open-head`.

**4. Gate and roll out:**
- Gate with `ops/check.sh`.
- Deploy with unit 4's served sentence. The flag makes "By submission deadline." false, which is the issue 370 class.
- Run `refold-open-head` dry, then wet, then `project`.

**5. Post-deploy reads:**
- the Verify block;
- the Repro read with `AND t.current_closed_by_notice IS NULL` and an exemption column, which must return only K1/K2/E2 rows;
- the count of open heads caused by an sdk-0.1 `can-standard` notice, which must be 0;
- 403644 and lot 1108460 still open;
- an SSE `status=open` subscription receiving `removed`, and a public change row for each flip;
- 273's closing matrix, `status=open&sort=deadline` with and without `source=ted`, `status=open&deadline_after=…`, and 423's `lots?status=open&cpv=45&limit=100`. None may regress.

### Where the proposals disagreed
- **DPS:** project BT-766 (proposal 1) rather than accept the gap (proposal 3). 445631 (ДСП) and 336277 (DAS) are named DPS-shaped schemes in the cohort, and a false close hides a live opportunity.
- **BT-765:** not projected (against proposal 1). The rule does not read it.
- **Coverage:** the Repro definition (proposal 1) rather than own-round coverage (proposal 3). With K1 guarding DPS, the stricter clause only adds false opens, and keeping the Repro read as the reference lets unit 2 check that the cohort drops to exactly the exempt rows.
- **UK12 without awards:** stays open (proposal 3) rather than a subtype branch (proposal 1). It does not say which lots it ends, and nobody has measured it.
- **E2:** adopted from proposal 3. It only keeps Tenders open, and it reads projected data.
- **1(b):** the mapping (proposals 1 and 2) rather than a flag that keeps the date (proposal 3). Keeping it serves a self-contradictory `submission_deadline`, and proposal 3's head-election objection is false.
- **Flag type:** INTEGER 1/NULL (proposal 2) rather than a TEXT reason (proposal 3). With 1(b) in the mapping there is only one reason.
- **Per-lot term:** kept with proposal 2's bound and fallback rather than deferred (proposal 3). The issue's 1(c) asks for it, and step 0b measures its population.
- **Lot seed:** unchanged (proposal 2) rather than flagged (proposal 3). The flag would make the covering `(current_deadline, id)` read do about 42k table lookups.
- **SSE:** adopt `StatusBy::Version` (proposal 2). This was verified at `read.rs:2831` and `sse.rs:465`.
- **Change rows:** add explicit clauses (proposal 2). Fold-time lot closures change no lot fact.
- **Backfill cohort:** ~42k (proposals 1 and 2) rather than ~684 (proposal 3). The head-results cohort misses DÖE award heads whose AwardDate-only result yields no round, and R3 costs one verify each.

### Accepted risks, carried to unit 3 or noted
- **Late result after a re-launch.** If a re-launch CN is published before the earlier round's late `clos-nw` result, the Tender closes wrongly. BT-140 and OPP-090 are not parsed. After deploy, read the closed Tenders that have two or more contract-notice versions.
- **Legacy-era open-ended schemes close.** Legacy procedure codes never map to `oth-*`, and the legacy DPS markers are not projected. Step 0a's stop rule is the guard.
- **Unprojected FTS signals.** PCR-2015 DPS via `tender.techniques` is not parsed, so such a DPS closes. An explicit eForms DPS termination (BT-119) stays open under K1.
- **Past-deadline flags stay NULL** until the Tender's next refold. That refold over-delivers one harmless correction. A future R4 would announce all of them at once, so count them before any R4.
- **Transient after deploy.** Between the deploy and the end of `refold-open-head`, the per-lot SQL term runs on stored rows that lack 2a's `dps` facts. Run the refold right after the deploy.
- **Per-lot closures are R0.** Where the Tender's flag does not move, a per-lot closure has no correction row. CHANGELOG must say so.
- **Other head-relative SSE predicates.** `Scope::At` stays head-relative for every other head-column predicate (deadline bounds, value, published, source, kind). File that as its own issue.
- **Rust/SQL drift.** The twins agree only as far as the fixtures and the tripwire cover them, so both must change in one commit.

### Rejected at decision
- Proposal 1, Option A (`current_open_until` plus its own index): status would need two ranges and the new index would serve nothing. The index is deferred, so it walks 7.9M rows until a Reindex, and its NULL default forces a 7.92M backfill or a correction on every later refold.
- Proposal 1, UK12/tenderCancellation termination branch that closes a Tender whose UK12 head has no awards of its own: it cannot say which lots it ends, and the population is unmeasured.
- Proposal 1, projecting BT-765 framework alongside BT-766: framework is not a rule input, so it adds projection scope and fill cost for no change in the rule.
- Proposal 3, the generic `award-only` rule (every version published results, no open-nw): it would close any all-award chain, including selec-w multi-round DPS/framework shapes in the unmeasured 175.
- Proposal 3, keeping the DÖE award's date and closing through the flag: it keeps serving a self-contradictory submission_deadline through the served field, sort=deadline and deadline_after. Its claim that dropping the date breaks head_election_agreement is false, because that test counts store/app selector lines only (head_election_agreement.rs:580-596).
- Proposal 3, adding `t.current_closure IS NULL` to the open-head lot seed (read.rs:3869-3884): it turns bare status=open's covering (current_deadline, id) seed into about 42k table lookups.
- Proposal 3, the head-results backfill cohort (~684): it misses DÖE award heads with no result round that 1(b)'s mapping still moves. The ~42k open-head range costs one R3 verify each.
- Proposal 3, own-round-only lot coverage: with K1 guarding DPS it only adds false opens, and it breaks the Repro read as the reference definition for unit 2's post-deploy check.
- Proposal 3, TEXT `current_closure` reason codes: with 1(b) handled in the mapping there is one reason, so INTEGER 1/NULL follows the is_buyer convention.
- Proposal 3, moving the 3-5 raw sdk-0.1 reads to unit 4: the mapping drop depends on the date not being a submission deadline, so the read stays a step-0 precondition with a stop rule.
- Proposal 3, no SSE probe change: tenders_query passes StatusBy::Head for Scope::At (read.rs:2831), so a fold-time close reads closed on both probes and is dropped at sse.rs:465.
- Proposal 2, closing DÖE award heads through the flag while keeping the date: replaced by the mapping drop, which proposal 2 itself allowed once the raw read shows the date is not a submission deadline.
- A job-706-style SQL UPDATE backfill: R2 needs the fold's own function over stored rows, and no loader exists (stored_version returns raw leaf rows, canonical.rs:21611). Transcribing the rule into SQL is issue 375's hazard.
- Title, deadline distance from publication, BT-144 reason, `now` and `tenders.source` as rule inputs (all three proposals agree).

## Unit 2 build notes (2026-10-16, after the build review)

- **1(c)'s per-lot bound fired, so the per-lot leg shipped out (the decided fallback, no re-decision).** The builder's
  `#[ignore]` perf test passed at 1.02x only because every fixture lot carried its own deadline, which made the
  baseline the 424 EXISTS's own quadratic walk. On lots without a deadline of their own (re-measured locally, debug
  build, same fixture otherwise): `lots?tender=X&status=open` 1.62x at 300 lots, 1.83x at 1,200, **2.71x at 2,400**
  (413 ms → 1.12 s); the lots stream page (`status=open&limit=100`), where the leg is correlated per lot, **88x at 300
  lots** (109 ms → 9.6 s). So a lot is closed exactly when its Tender is: `lot_closed_by_result(_sql)`,
  `closed_lot_keys` and the lot-leg set are gone, `StatusBy::Lot` reads only the Tender's closure (the stored flag on a
  page, `closed_by_notice_sql` on the At probe), and a lot's change row fires on its Tender's flip. Step 0b's 11 lots
  in 7 open Tenders (a partial UK12's cancelled lot) stay open — unit 3. The perf test now builds lots without own
  deadlines and times the stream page too (the flag alone: a residual / one PK seek per lot).
- **The K1/K2 leg walked the corpus.** With no row stats turso planned `cb_c`'s two-scheme OR as `MULTI-INDEX OR cb_c
  (tender_version_classifications_code, …)` — a `(scheme=?)` seek per branch over every `procedure` and `dps` row,
  filtered on the version afterwards (read locally with EXPLAIN QUERY PLAN; the review measured one At probe at 6 ms →
  1.85 s beside 100k non-matching rows). Pinned `INDEXED BY tender_version_classifications_version` (in SCHEMA, never
  deferred); the plan test now rejects a `MULTI-INDEX OR`, a `(scheme=` access or any non-version access on every
  `cb_*` leg, alone and inside the Tenders and Lots At statements, and the perf test times an At probe beside 100k
  other rows.
- **The SQL twin's coverage leg was quadratic on the At probe** (found while re-timing): `NOT EXISTS (a Lot-kind lot
  with NOT EXISTS (a result naming it))` walked the version's result slice once per lot, and a big Tender's flip
  writes one lot change row per lot, each probed at two versions by every lots subscription with `status` — 120
  probes of a 2,400-lot all-`clos-nw` Tender had not finished in 15 minutes. Now a COUNT of Lot-kind lots against
  the distinct Lot-kind lots the results name (results joined to their lot row by primary key): 2,400 lots cost 8.07x
  300 (linear), about 0.25 s a probe in a debug build. The perf test pins the scaling (< 16x for 8x the lots).
- **Runbook:** the 2a fill gains a third field refold, `TED-DPS` + `TED-PROCUREMENT_DISCONTINUED` +
  `TED-PROCUREMENT_UNSUCCESSFUL` on `notice_integers` (K1's legacy leg and 2b's reasons are Integer markers, which the
  `notice_codes` refold never reaches).
- **Served text:** W is measured from the PUBLICATION of the first deadline-bearing notice to the current deadline (the
  first cut said "between the first deadline and the current one", which predicts a close for 78054, 232118, 8606643
  and 634469 that the code keeps open). The `dps` scheme is documented on `classifications[]` (the version detail
  serves every scheme, so 1(a)'s "a new dps scheme changes no REST payload" held for list rows only).
- **The At probes paid the whole rule on every probe (second review, 2026-10-16).** turso 0.7 runs every correlated
  subquery of a loop before it tests any condition there, so the "cheap legs first" order spared nothing: a version
  with no results paid both coverage counts, and the lots At probe of a 2,400-lot Tender cost 10-17x the pre-533 one
  (the Tenders At probe 23x), with 4,800 probes per subscription on a flip. Now (`canonical::closed_at_version_sql`)
  the At probes read the stored flag at the head, where the agreement test pins flag = rule, and run the rule only
  at an earlier version, in two loops: an own result and no non-`clos-nw` result on `cb_v`, then the counts and the
  exemption on `cb_g`, the same row one loop deeper (the `clos-nw` leg carries `LIMIT 1` so turso does not unnest it
  into an anti-join after `cb_g`). The coverage count's lot join is keyed on the version row, so its results seek
  `(tender_id, seq)`, not every version's slice (1.7x on 31 versions). Measured, debug, 2,400 lots, best of 3 x10,
  each against itself with the closed leg cut out: lots At at the head 1.29x, at the CN 1.03x, Tenders At at the head
  1.59x — a constant 4-5 ms of preparing the rule's SQL, whatever the size — and 2.18x at an earlier award version
  whose LAST result names a winner (the rule's own pass over the results; bounded at 3x). The perf test pins all
  four; the plan test now requires `(tender_id=? AND seq=` on every version-keyed leg. Still paying the full rule: an
  earlier version that passes both cheap legs (one that closed, or an exempt all-`clos-nw` one), linear in its lots.
- **Served text, second pass:** the closing clause names every result the procedure has published (all rounds, as
  decided: the Repro coverage, not own-round coverage) and "together they decide every lot"; the reopen trigger is any
  later notice without results of its own, a corrigendum or modification as well as a re-launch CN (unit 3 decides
  whether a non-CN head should keep the closure); the Dates caveat says "outside an open-ended scheme"; the
  `current_closed_by_notice` note no longer says "before its deadline" (the rule reads no timing); the reasons note
  says "since issue 533 deployed", not "2026-10". The 370 gate: RETIRED holds "By submission deadline." and "(by
  submission deadline)", and `the_docs_say_status_reads_the_newest_notice` pins one phrase per sentence.
- **Fixtures:** four chains pin the coverage count's DISTINCT (two results on one lot of two; a lot decided in two
  rounds) and its result-side kind filter (a result naming only a group; results naming the group and the lot) —
  each mutant of the SQL now fails the agreement test.

## Proposed units

1. **Decide (S–M; the decision is the step).** Record the decision here with its reasoning. It has three parts.
   - (a) **Does an all-`clos-nw` head result, or an FTS UK12 head, close a Tender?**
     - The candidate answer is yes for a one-shot procedure (841082, 38279, 8576007).
     - For an open-ended scheme (DPS/DAS, open-house, LOV/authorisation, qualification system, e-marketplace), a
       `clos-nw` award may report one round with no award. Explicit cancellations (6739006, 7422682) sit in the same
       class as 403644.
     - BT-765 and BT-766 (framework/DPS) are not projected; grep finds them only in comments. Either project them
       first, as a sub-unit, or decide on what is projected (procedure type, subtype, the deadline's distance from
       publication). Never decide on the title.
   - (b) **What about award-type heads with a NULL decision (DÖE sdk-0.1 `can-*`)?** Two options:
     - Stop admitting a `submission_deadline` that an award-type notice publishes, in `deadline_admitted` or
       `head_deadline` (one home, 474). This also changes the served `submission_deadline`, `sort=deadline` and
       `deadline_after/_before`.
     - Or keep the date served and close an award-type head through the status column.
     Read 3 to 5 more raw notices first, to settle what that date means on an award.
   - (c) **The read shape.** `status` must stay an indexed range on a head column (273). Two options:
     - Option A: a fold-maintained `current_open_until` (equal to `head_deadline`, NULL when the rule fires), with a
       `(col, id)` index read by `status`. `current_deadline` keeps sort and the deadline bounds.
     - Option B: a flag folded into the same range.
     Today `status=open&sort=deadline` and the `deadline_after` "closes soon" shape ride `tenders_current_deadline`
     (`read.rs:1903`). Read both plans locally and re-run 273's closing matrix; nothing may regress.
     - Lots: a lot whose head-notice result is `clos-nw` is closed. That is a NOT EXISTS term in 424's per-lot EXISTS,
       through the `canonical` helper. 423's cpv-prefix control must not regress.
     - Re-launch: a head CN after an earlier `clos-nw` stays open. The 88 were inferred from head subtype only, so
       confirm this on a sample.
2. **Build, gate, deploy, backfill (M).** The rule only changes answers for the open head (`current_deadline > now`,
   an indexed range of about 42k), because a past deadline is closed already.
   - Option B, with a flag that defaults to not-closed, needs a backfill over that range only. Option A needs the
     column on every row: job 706 filled 7.92M for `current_deadline`.
   - Either way, reuse the fold's own function (375), in 216's job-706 shape, not a corpus refold.
   - A result that arrives later must flip the tender at fold time.
   - Then read the Verify and the Repro cohort read. The cohort should drop to what 1(a) deliberately keeps open.
     Check that an SSE `status=open` subscription and a webhook filter see the change.
3. **Measure the rest (M).** File what remains as new issues.
   - The 175 with a winner: read BT-765/BT-766, UK13–16 and the deadline distance on a sample. Does "awarded before
     the deadline" ever close a non-DPS procedure?
   - The ~102 unclassified non-DÖE heads (partial, `open-nw`, null decision).
   - The lot rule's per-lot leg, shipped out of unit 2 under 1(c)'s fallback (`## Unit 2 build notes`): step 0b's 11
     lots in 7 open Tenders. It needs a shape that stays under 2x on the lots stream, not a per-lot SQL set — e.g. a
     fold-maintained per-lot flag beside the version's lot rows.
   - The reopen trigger as built is "the head published no results of its own", not "a contract notice": a
     corrigendum, modification or result-subtype notice without results reopens a closed Tender (served as such since
     the second review). Decide whether a non-CN head without results should keep the closure.
   - The channels nobody has measured: FTS OCDS `tender.status` (ignored at `checklist.rs:116`, so a cancellation
     carried only there is invisible) and eForms BT-140 change notices (not parsed).
4. **Docs (S).** `openapi.json:755` and `docs.rs:172` name what closes a Tender besides its deadline. Correct the
   `Status::Closed` doc, the `read.rs:1124` rationale and `head_deadline`'s doc (award notices DO carry a deadline,
   inherited or their own), and make `checklist.rs:116` true or correct it. Gate the served sentence (the 370 class).
