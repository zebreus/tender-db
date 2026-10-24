# 572 unit 2 — close (2026-10-24, R3 + monitor M0 + Verify)

Deployed code: 3fb8803 (572 U2a + U2b), live in prod rev 6cb4408 (deploy #3; `/health` rev
6cb4408030f3…). Budget for this close: at most 2 `/v1/sql` statements and 21 public GETs. **Spent: 1 SQL, 4 GETs** (plus 1 GET of `/health` by the adversarial check, row 6: 5 of 21).
GETs were sequential and at least 6 s apart. There were no 408, 429 or 5xx responses and no retries. Every request is
logged in `../requests.log` with the tag `(572 u2 close …)`. Bodies are in `close/`.

**Verdict: every bar passes. 572 is DONE.** No reopen trigger has fired.

## Reads

| # | time (UTC) | request | answer (excerpt) |
|---|---|---|---|
| 1 | 19:02:10 | GET `/docs` | 200, 123,369 bytes; `grep -c 'issue 572'` = **1**; the bullet and the clause are present (below) |
| 2 | 19:02:23 | GET `/v1/tenders/8641682` | 200, v2; Verify line 1 jq = `[[30953692,"Northern Housing Consortium Ltd","Birmingham City Council"]]` |
| 3 | 19:02:49 | `/v1/sql` `m0_monitor.sql`, run through `/root/sq.sh` on stdin | rc=0, 0.52 s, 159 rows, `truncated: false`; columns `k,q,org,name,n,lo,hi` → `m0_monitor.out.json` |
| 4 | 19:05:23 | GET `/v1/tenders/8655231` (F1's Verify, open state) | 200, v1; Procedure-Buyer rows `[[30916230,"Rochdale Metropolitan Borough Council"],[30917275,"Star Procurement"]]` |
| 5 | 19:05:29 | GET `/v1/tenders/8580124` (F1's Verify, open state) | 200, v4; Procedure-Buyer rows `[[30914436,"COLLEGE OF POLICING LIMITED"],[30914436,"GLEEDS COST MANAGEMENT LIMITED"]]` |
| 6 | 19:16:51 | GET `/health` (adversarial check) | 200; `rev` 6cb4408030f3863c7dfac17056f4461a48df99bc, the operator notice up (576/586/577) |

The SQL was plan-probed first, on a private copy of `../plan.db` (prod schema plus the prod org indexes). The plan is in
`m0_monitor.plan.txt`; it has 0 SCAN steps.
- python sqlite3 3.45.1: 9 × `SEARCH organizations USING INDEX organizations_identifier_id (identifier=?)`, then
  `SEARCH organization_mentions USING INDEX organization_mentions_org (organization_id=?)` fed by a
  `LIST SUBQUERY` (`SEARCH organizations USING COVERING INDEX organizations_identifier_id`), then a temp B-tree for the
  GROUP BY.
- turso `target/debug/plan-probe plan` (rc=0): the same SEARCH steps, with `USE SORTER FOR GROUP BY`.

## R3 — after the deploy

### /docs carries the bullet: PASS

These are the GET #1 body's "Codes and identities" list and Tender-detail section, whitespace squashed. All 9 needles of
the test `the_docs_say_a_buying_agents_identifier_binds_its_member_bodies` are present in the list: `buying agent`,
`mention_name`, `buyer=`, `v_tender_buyers`, `host`, `procuringEntity`, `Procedure-Buyer`, `roleDetails`, `issue 572`.

- **Bullet** (matched, "Codes and identities"): "**A buying agent's identifier can stand for the body it buys for.** On
  Find a Tender a buyer party can carry the identifier of the account that published the notice instead of its own. When
  a buying agent, consortium or shared procurement service publishes for a member body, the notice can print the
  member's name with the agent's identifier. … Such a party binds by the published identifier (issue 456):
  `organization_id` and `organization_name` are the agent's, and the printed name stays in `mention_name`, so read
  `mention_name` for display. … Find a Tender's `procuringEntity` party is served as `Procedure-Buyer` beside the buyer,
  and its free-text `roleDetails` ("on behalf of …") is not kept. Measured on 2026-10-19: about 186 party mentions on 185
  Find a Tender notices, from 14 agents (issue 572)."
- **Clause** (matched, in "A party names its organization twice" in the Tender-detail section): "… `mention_name` still
  says what the notice said (issue 456). A buying agent that publishes for a member body under its own account is one
  systematic case (see caveats → Codes and identities)."

### `ops/check.sh` reads `GATE-EXIT=0` for each commit: PASS (not re-run)

Unit 2 is one commit, 3fb8803.
- **Its own gate.** The 551 log records "572 U2 gated green at 3fb8803" (2026-10-24, 09:36–10:00Z). The twin
  adcb7b0 re-gated green at 10:23–10:51Z.
- **First live: the batch deploy a63cf23** (2026-10-24 11:32Z, gate green at ed3e2d5 = its tree, 1,915 s; 551 log).
  `git merge-base --is-ancestor 3fb8803 ed3e2d5` → 0.
- **Deploy #2.** `git merge-base --is-ancestor 3fb8803 05313fa` → 0. Deploy #2 is ad8e305, gated green at 05313fa (its
  tree) with 153 suites.
- **Deploy #3.** `git merge-base --is-ancestor 3fb8803 eac7067` → 0, and `… 3fb8803 6cb4408` → 0. Deploy #3 is
  6cb4408, whose code is eac7067, "gate c green 153 suites". `git diff --stat eac7067 6cb4408` touches only
  `.scratch/` (2 files). `deploy.sh` refuses a tree without a green gate.
- **On main.** `git merge-base --is-ancestor 3fb8803 af959ff` (origin/main) → 0.

The U2a docs test and the U2b checklist test were in those gated suites.

### U2b re-queues 0 notices: PASS

- U2b is `crates/ingest/src/fts/checklist.rs` only. It changes the `CHECKLIST` dispositions (`parties[].roles` reworded;
  `parties[].roleDetails` and `parties[].details.classifications` become Ignored("owed: …")) and adds 1 unit test.
- `CHECKLIST` is census and test metadata. The parser does not read it: `fts/parse.rs:249` still maps
  `"buyer" | "procuringEntity"` → `OPT-300-Procedure-Buyer` unchanged. So no stored notice parses differently.
- No reparse, refold or project job was posted for 572, before or after any of the three deploys that carried it (a63cf23, ad8e305, 6cb4408). The 551 C1 code-drift ledger
  records "U2b … metadata only: not a fold input, 0 notices re-queued, no job".
- This close posts no job either.

### M0 recorded: PASS (below)

## Monitor M0 (2026-10-24 19:02Z, 1 `/v1/sql`)

Keyed on the 8 identifiers (`decision.md` "Monitor M"). The count's `organization_id IN (SELECT id FROM organizations
WHERE identifier IN (…8…))` follows the identifier, not a stored id. Scored by `m0_score.py` against
`../triage_frozen_lists.json` (dated 2026-10-19); the full output is in `m0.json`.

### Resolution and head check: 8 of 8

| identifier | agent | resolves to | served head (`organizations.name`) | head check |
|---|---|---|---|---|
| GBPPONPJXZ4423NDBT | NHC | 30953692 (as listed) | Northern Housing Consortium Ltd | agent ✓ |
| GBPPONPXWV6492CGMN | NWSSP | 30913738 (as listed) | NHS Wales Shared Services Partnership-Procurement Services (hosted by Velindre University NHS Trust) | agent ✓ |
| GBPPONPVMQ1845NYPP | Welland | 30913798 (as listed) | Welland Procurement | agent ✓ |
| GBCOHOC322760 | Gibbs Laidler | 30913722 (as listed) | GIBBS LAIDLER CONSULTING LLP | agent ✓ |
| GBPPONPGGP8622HHRX | Invictus | 30930308 (as listed) | INVICTUS PARTNERSHIP LTD (13647652) | agent ✓ |
| GBPPONPLMN9829JXLV | Ginger | 30968739 (as listed) | Ginger energy | agent ✓ |
| GBPPONPRYY6263NQHG | RPJ3 | 30914820 (as listed) | RPJ3 Group Limited on Behalf of Aspirational Futures MAT | agent ✓; known (524): the head names a principal |
| GBPPONPBWD3998QHRQ | NHS London Commercial Hub | 30916628 (as listed) | NHS LONDON COMMERCIAL HUB | agent ✓ |

- Every identifier resolves to exactly one non-provisional row, the listed one, so no merge needed following (0 GETs).
- An extra arm on GBPPONPGJB3773RCMH (Gibbs' mention PPON, AGENT_ACCOUNTS spelling) returns 0 rows. It is still merged
  into the COH org, as on 10-19.

### Counts

| line | 2026-10-19 | M0 2026-10-24 | change | reopen trigger | fired? |
|---|---|---|---|---|---|
| **P@A on the agent accounts** (list (a) principal keys + the 2 transferred slips still bound) | 153 | **154** (71 keys) | +1: Cardiff and Vale on NWSSP, 47428982 | ≥ 306 | no |
| — of which list (a) reviewed keys, buyer role (the "151 once 545 moves the slips" basis) | 151 | 152 | +1 (the same) | | |
| **NHC member mentions under GB-PPON-PJXZ-4423-NDBT** (list (a) NHC keys 96/51 + Riverside slip) | 97 / 52 bodies | **97 / 52** | 0 | ≥ 194 | no |
| — with the unreviewed key counted in | — | 98 / 53 | | ≥ 194 | no |
| **Velindre on NWSSP's PPON** ("Velindre University NHS Trust" 48 + "Welsh Blood Service (Velindre…)" 1) | 47 | **49** | +2 (latest notice 47434325) | own line, no trigger | — |
| **Unreviewed keys** (on no list and not matching the agent's pattern) | 0 | **1 key, 1 mention** | +1 | review it into list (a) or send it to 545 | — |
| agents' own names (pattern) | — | 1,669 | | | |
| other non-principal names (NWSSP variants 39, including "NHS Wales" 11 and "NHS" 2; NHC framework title 4) | — | 43 | | | |
| all mentions on the 8 orgs | 1,881 | 1,916 | +35 | | |

- **Per org, 10-19 → 10-24:**
  - NHC 124 → 125;
  - NWSSP 1,170 → 1,196: own +18, own variants +5, Velindre +2, P@A +1;
  - Welland 287 → 293;
  - Gibbs 163 → 164;
  - Invictus 56 → 57;
  - Ginger 31, NHS LCH 26 and RPJ3 24, unchanged.
- **Every one of the 70 list-(a) keys is still present.**
  - 68 hold exactly their frozen count.
  - Cardiff and Vale (NWSSP) is 10 against 9 frozen: the new 47428982, which R1 read R this morning.
  - Hywel Dda (NWSSP) is 12 against 11 frozen. The extra one is the Tenderer slip 46776118, still bound and still owed
    to 545's follow-up cohort. Under its three raw spellings the 12 split as 10 + 1 + 1.
- **The unreviewed key** is "COMMUNITY GATEWAY ASSOCIATION LIMITED" on NHC 30953692, 1 mention, notice 47418946 (tender
  8853907). It is the same new key q8 found this morning. It looks like an NHC member housing association, so it is
  probably P@A, but under guard G-A it is reviewed as a normal stray and waits in this bucket until a reviewer adds it to
  list (a) or sends it to 545.
- **Both transferred slips are still bound:** Riverside 46798381 on NHC (counted in the 97) and Hywel Dda 46776118 on
  NWSSP. The 5 F-shaped mentions stay 545's follow-up work.

### The other reopen triggers at M0

| trigger | state at M0 | fired? |
|---|---|---|
| a consumer asks | none recorded (no active users) | no |
| current P@A buyer-role rows ≥ 190 | q8 (this morning, 10-24 08:50Z): current P@A 99, of which 2 are the Tenderer slips, so 97 buyer-role (95 on 10-19) | no |
| ≥ 32 principals above 10% of their own `?buyer=` list | not re-read; M does not carry it (q9 shape). If no principal's own list shrank, it is at most 17: 16 on 10-19, and the only new routed rows are Cardiff and Vale (6/76 → at most 7/76, 9.2%, still below 10%; 7/77 = 9.1% if the new tender is also on its own list) and Community Gateway (at most +1) | no |
| any principal with a list ≥ 20 above 30% | Hywel Dda's routed count is unchanged (7 of 27, 25.9%); 1 new row on a list of 20 or more is at most 5% | no |
| FTS structured on-behalf field, or F1 adopts a distinct role | F1 filed today (589), undecided | no |
| GB principal consolidation, 456 design 1 or 300 §10 | 456 is still at design 3 (deployed 2026-10-01) | no |
| a general reviewed (identifier, name key) → org alias arm | none exists | no |
| `mention_name` regresses | Verify line 1 still serves "Birmingham City Council" byte-equal, and R1 read 40/40 R this morning | no |

## Verify (issue 572)

    curl -s https://tenders.zebreus.click/v1/tenders/8641682 | jq -c '[.parties[] | select(.role=="Procedure-Buyer") | [.organization_id, .organization_name, .mention_name]]'
    curl -s https://tenders.zebreus.click/docs | grep -c 'issue 572'

- **Line 1:** `[[30953692,"Northern Housing Consortium Ltd","Birmingham City Council"]]`. This is **unchanged**
  (decision 572-L). PASS.
- **Line 2:** `1`, which is at least 1: the caveat is live. PASS.
- **R1 40 R / 0 W / 0 N, and R2 6 of 6** (read 2026-10-24 morning, `r1.md` / `r2.md`). PASS.
- **U2b landed on main.** 3fb8803 is an ancestor of origin/main af959ff and was gated green. PASS.
- **The 5 F-shaped mentions are with 545.** 545's body section "From 572 unit 1 (2026-10-19 …): follow-up cohort
  additions and guard G-A" adds all 5 (Pagabo ×2, Riverside, Hywel Dda, NWSSP on Swansea University) to the follow-up
  cohort that 545's Status NEXT names ("a follow-up cohort for Wyre Forest 46842882"). PASS. Note: 545's Status line
  named only Wyre Forest; U2c (2026-10-19) had added the body section, not the NEXT line. The adversarial check added
  the 5 to 545's NEXT line on 2026-10-24, so the bar now holds as written.
- **M0 recorded** with the 8 accounts resolved by identifier and their heads checked. PASS.

## F1

F1 is filed as `.scratch/tender-db/issues/589-find-a-tenders-procuringentity-is-served-as-procedure-buyer-and-its-roledetails-is-dropped.md`
(needs-triage). Reads #4 and #5 are its Verify's open state.

## Adversarial check (2026-10-24, after the close)

No new SQL; 1 GET (`/health`, row 6). Re-derived from the saved files, not by re-running `m0_score.py`.

- **Confirmed.**
  - M0's statement resolves the 8 orgs by identifier: 8 `identifier = ?` arms (each 1 row, the listed id,
    `provisional` 0), the Gibbs PPON arm 0 rows, and the count keyed `IN (SELECT id … WHERE identifier IN (…8…))`.
    159 rows (8 org + 151 mention groups), `truncated: false`.
  - An independent re-score by name key against `triage_frozen_lists.json` gives the same numbers: total 1,916; list (a)
    keys 153 (frozen 151: Cardiff and Vale 9 → 10, max notice 47428982; Hywel Dda 11 → 12, the 10-19 total with the slip,
    no notice above 46883773); Riverside slip 1 (46798381), so P@A 154 and 152 without both slips; NHC 96 + 1 = 97 over
    52; Velindre 48 + 1 = 49 (max 47434325); 1 unreviewed key (Community Gateway, 47418946); 0 list (a) keys missing.
    Per-org 10-19 totals are `../measure.md`'s table (sum 1,881).
  - Current P@A buyer-role rows 97 (q8: 97 `Procedure-Buyer` current + 2 `Tenderer` slips), against 95 and trigger 190.
  - R3: the docs test's own extraction (Codes and identities `<ul>`, squashed) finds all 9 needles in the saved
    `/docs`, and the detail section names the buying agent. `/health` serves rev 6cb4408, which contains 3fb8803.
    3fb8803 touches only CONTEXT.md, docs.rs, sql.rs and checklist.rs, and nothing outside checklist.rs reads `CHECKLIST`.
  - 589 is free on every local and remote branch, and its file follows the issue format (Status, Kind, Relates to,
    a Verify with both states).
- **Corrected.** The Cardiff and Vale share (7/76, not 7/77, when the new tender is not on its own list); the first
  deploy carrying 3fb8803 (a63cf23, which the close left out); 545's NEXT line (the only Verify "done" bar that did not
  hold as written; now it does); 572's closure format (`DONE <date> — …` and a `## RESOLVED-VERIFIED` section,
  `docs/agents/triage-labels.md`).
- **Noted, no trigger affected.**
  - A new NWSSP spelling (47434180) reads "Velindre University NHS Trust - Please note NHS Wales Shared Services
    Partnership (hosted by Velindre University NHS Trust) will manage this procurement on behalf of the contracting
    authority. Due to having the same identifiers, we're unable to reflect this in the notice options." The agent
    pattern makes it an own name (the rule as decided), so Velindre reads 49. By substance it is a Velindre mention
    (50), and the publisher is stating 572's mechanism in its own words.
  - The decision says to re-probe M0 through the endpoint. This close probed locally instead (sqlite on `plan.db` plus
    turso `plan-probe`), as its task said; both gave SEARCH only.
- **Gaps, not reasons to reopen.** With 572 DONE, `ops/board-verify.sh` reads it as closed, and nothing schedules the
  2027-01-19 monitor run, so a scheduled task or a carrier issue has to hold it. The two principal-share triggers are
  on no scheduled read: the quarterly M should add the q9 shape, or bound them from the routed-row delta as M0 did.

## Files

- `m0_monitor.sql`, `m0_monitor.plan.txt`, `m0_monitor.out.json`: the M0 statement, its plan and the raw body.
- `m0_score.py` → `m0.json`: the scoring.
- `close/`: the 4 GET bodies.
- `close_get.sh`: the GET runner (6 s spacing, cap 21, no retry; a copy of the session script that ran).
