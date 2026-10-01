# 453 — 182 wrong-number organizations have a reviewer-found right number that nothing acts on

Status: done — WET RUN 2026-10-01 07:02 UTC (job 1760): held against plan 1739's 167 keys, merged 111 + moved 53. Residue re-planned (job 1762) and merged 3 more (job 1763). Verify reads **15**, each with a recorded reason (below).
Was status: blocked — the WET RUN WAITS ON LENNART'S GO-AHEAD. The session's permission classifier refused the production write on 2026-09-30 ~19:0x UTC, as with 448's wet run. Everything before it is done: `1389820` deployed (454's override and the flagged-by-number refinement), and dry plan job 1739 holds 113 merge + 54 move = **167 re-keys, every one reviewed** (below). To run once given: `{"kind":"match-org-identifiers","rule":"rekey","dry_run":false}` (it holds against 1739's stored keys), then the Verify.
Was status: ready-for-agent — ARM DEPLOYED 2026-09-30 16:5x UTC (`e156b88`); first dry plan (job 1726): 100 merge + 58 move, 16 name-denied (→ issue 454). REVIEWED against the Companies House register (below): 156 execute, 2 held and their verdicts corrected (Cochlear → 03874867, Montel → 08949189, cohort `453-rekey-review-2026-09-30`). NEXT: re-plan job 1738 (queued behind FTS chunk 4, jobs 1728–1737); diff its keys against the reviewed 156 + the two corrections, then the wet run, then the Verify. The flagged-by-number refinement (`19ea011`) deploys after the wet run.
Was status: ready-for-agent — SIZED 2026-09-30 15:0x UTC: of the 182, **119** have a standing GB org carrying the right number (shape 1, merge into it) and **63** have none (shape 2, re-key in place). No existing arm executes either: case reviews only strip, merge verdicts need a shared key group. NEXT: the arm (design below), dry-first.
Was status: ready-for-agent — filed 2026-09-30 from issue 452. Next: for each of the 182, look up whether the right number
already has its own org (a bounded id/identifier lookup), then size the three shapes below before building anything.
Kind: data quality (identifiers)
Relates to: 452 (the verdicts), 448 (the altid arm), 362 (merge verdicts)

## What is there

Issue 452 posted 685 identifier verdicts (cohort `452-census-2026-09-30`). 371 are `wrong`, and for **182** of those the
reviewer found the organization's real company number, recorded as `correct_identifier`. 83 of the 102 `related`
verdicts carry one too. Nothing reads it. By design (452): re-keying an org is merge-shaped, because the right number
may already have its own org.

Today such an org is withheld from identifier matching and flagged `register_mismatch`. It stands alone under a wrong
number: its tenders do not join its real registration, and a reader who follows the right number does not find them.

## The three shapes

For each `wrong` verdict with a `correct_identifier` C:
1. **C has a standing org R**, the same entity: merge the wrong-number org into R. This is a merge verdict under a new
   scheme (for example `GB:rekey`, key `<wrong>~<C>`), reviewed members, the 362 execution path. R keeps its own
   identity.
2. **C has no org:** re-key the wrong-number org to C in place. The identity change is a move (the 355 shape: pre-image
   in the verdict row, change event, reversible). Afterwards the 452 verdict is inert by construction, because the org
   no longer carries the number.
3. **C has an org that is NOT the same entity** (the reviewer's C is itself wrong): leave it and record why.

## Verify

    /root/aj.sh "/admin/case-reviews?table=identifier&limit=5000" | python3 -c "import json,sys; d=json.load(sys.stdin); print(sum(1 for r in d['rows'] if r['verdict']=='wrong' and r['correct_identifier'] and not r['applied_at']))"

(Corrected 2026-09-30 by the hourly audit. The first form filtered on the 452 cohort and counted applied verdicts
too, so it could never reach its "done" state: a re-key stamps `applied_at` but keeps the row. It also missed the two
verdicts the plan review re-POSTed under cohort `453-rekey-review-2026-09-30`.)

- **open**: `182` (every one still carries its wrong number with a known right one beside it). Read 17:5x UTC: 182.
- **done**: the unapplied count equals the recorded residue: 6 not high, 2 unkeyed, 6 name-denials the 454 review left
  unsettled (the 7, less Haringey, which passed the gate by itself in plan 1739), and 1 `keep` (B Braun). That is 15
  with a reason each, and so reads 15. Read 2026-10-01 07:08 UTC: **15**.

## 2026-09-30 15:0x UTC — sized

A bounded identifier seek per right number, both spellings (`C`, `GBCOHC`), GB only
(`453-rekey/shapes-2026-09-30.json`):

| shape | orgs |
|---|---|
| 1: an org already carries the right number | **119** |
| 2: no org carries it | **63** |

The shape-1 samples read as one entity on both sides:
- Rolls-Royce plc 01006142 → org 17148236 (01003142, Rolls-Royce Plc);
- Gasway 01458628 → 8879818;
- Fixatex 02791975 → 18010487;
- Firmus Energy 05369180 → 31547550 (GBCOH05369108);
- RSK Environment 05837803 → 10888303 (SC115530).

Shape 3 (the right number's org is someone else) is decided per pair by the name gate below.

**Why nothing existing does it.** `apply-case-reviews` executes one action only: stripping an identifier. `org_merge_verdicts`
are honoured only for a group that shares one canonical key, and a wrong number and its right number never share one.

**Design for the arm (next unit).**
- `rekey` as a rule of `match-org-identifiers`, planned from `org_identifier_verdicts` rows with `verdict = 'wrong'`
  and a `correct_identifier`, restricted to orgs that still carry the wrong triple.
- **Shape 1 is a merge.** The wrong-number org is the loser and the right-number org survives, through
  `repoint_org_references`. That is the R2/altid merge body: ledger rule `r-rekey`, change events. Gates: one owner of
  the right number (several is a family R2 declined), the name agreement the altid arm uses (`names_agree` over both
  orgs' heads and satellites), the legal-form veto, and the consortium veto. Dry run lists every pair, and the wet run
  holds against the listing's pairs, as the altid arm does.
- **Shape 2 is a move.** It rewrites the org's `identifier` to the right number's canonical spelling. Pre-image: the
  verdict row already keeps the wrong literal, and the move adds `applied_at`/`applied_action`. It fires an
  `organization changed` event. After the move the 452 verdict no longer matches the org, which is the intended end state.
- Tests: both shapes, the name-gate refusal (shape 3), the several-owners refusal, and parity.

## 2026-09-30 16:5x–17:3x UTC — deployed, first dry plan, reviewed against the register

**Deployed** at `e156b88`. It carries the arm, the review fixes (`60d5d0d`: pinned plan keys, one mover per right
number, flagged destinations, chained and bind-time alias) and the gate (GATE-EXIT=0, 928 s). Health was green and the
log had 0 error lines.

**Dry plan, job 1726.** 182 wrong verdicts carry a right number:

| class | count |
|---|---|
| not high | 6 |
| unkeyed | 2 |
| denied on names | 16 (issue 454) |
| **planned merge** | **100** |
| **planned move** | **58** |

No verdict fell into gone, several, same key, multi-target, withheld target, pending move or destination verdict.

The plan is `453-rekey/rekey-plan-2026-09-30.json`.

**Register check** (`ch_check.py` → `ch-check-2026-09-30.json`). It fetched both numbers of all 158 re-keys from the
public register:
- **136** auto-supported: the right number's register name agrees with the org's name, and the wrong number's does
  not.
- **22** were read by hand. Most are tokenization misses:
  - "Airconditioning" against AIR CONDITIONING, and "L N J" against LNJ;
  - trading-as suffixes;
  - an O for a 0 (St Annes `O1089026`, Choices, Assist Homecare, RJ McLeod `SCO28565`).

**Adversarial review** (workflow, 11 agents, 700k tokens). Two adjudicators per hard case — a register-history lens
and a refuting skeptic — plus a completeness critic. The critic fetched the register overview of every right and
wrong number in all 158 rows; its digest is `register-digest-2026-09-30.txt` and the verdicts are
`review-2026-09-30.json`.

- **Execute, the right number renamed after the org's name:**
  - Robinson Low Francis LLP is OC309255, renamed MGAC LLP on 2023-01-26;
  - Begbies Traynor Group plc is 05120043, renamed BTG CONSULTING PLC on 2026-02-04;
  - International Healthcare Recruitment is 11465799, renamed SANCTUS CARE GROUP on 2025-08-27.
- **Held, because the reviewer's right number was itself wrong:**
  - Cochlear Europe: 02802334 is not on the register (404). The real COCHLEAR EUROPE LIMITED is **03874867**.
  - Montel Civil Engineering: 08949159 is NEW ASSET ALLIANCE LTD, an unrelated firm. The published `089491S9` has an S
    for an 8, so the real number is **08949189**.

  Both verdicts were re-POSTed with the corrected `correct_identifier`, confidence high (recorded 2, stale 0). The
  next plan re-keys them onto the real numbers under new keys.
- **Flagged by the critic, kept:** Cross Keys Homes (31572377 → the org carrying 04557701). That company converted to a
  registered society (RS007643) in 2017. Both orgs are the same organisation, so the merge is right. The survivor
  carries the pre-conversion company number, a separate identifier question that the merge does not make worse.
- **Checked and kept by the critic:**
  - Dugard CNC (16230009, renamed from Forward CNC Machines in 2025);
  - CO-OP WHOLESALE (00980790, formerly NISA RETAIL);
  - Royal Bank of Scotland (SC083026, formerly Adam & Company until 2018; the published SC083027 is off by one);
  - Spirit Medical;
  - Cell Therapy Catapult;
  - Hogan Lovells (renamed 2026-06-30);
  - right numbers that are dissolved or in liquidation but name the only company of that name (Oculus, Roalco);
  - Seaham Care (FC031512).

## 2026-09-30 18:4x–19:0x UTC — deployed, re-planned, every key accounted for; the wet run waits on the owner

- **Deploy.** `1389820` went live after FTS chunk 4 and the old-code re-plan (job 1738) drained the queue: health
  green, 0 error lines.
- **Dry plan, job 1739** (`453-rekey/rekey-plan-2-2026-09-30.json`). Of the 182 verdicts: 6 are not high, 2 are
  unkeyed, 1 is kept by the B Braun `keep` verdict, 6 are still denied on names (the 7 unsettled, less Haringey,
  below), and 8 were admitted by 454's merge verdicts. **Plan: 113 merge + 54 move = 167.**
- **Diff against the reviewed plan.** 154 keys are unchanged. The 13 that are new:
  - 8 are the 454-admitted merges.
  - 2 are the corrected verdicts. Cochlear merges into "Cochlear Europe Limited" on 03874867. Montel merges into
    "MONTEL CIVIL ENGINEERING LIMITED" on 08949189. Both right numbers already have an org, so these are merges, not
    moves.
  - 3 changed shape because chunk 4 minted orgs on the right numbers:
    - **Aleyah House:** a move became a merge into "Aleyah House".
    - **Belfast City Airport Ltd:** a move became a merge into "BELFAST CITY AIRPORT LIMITED" (NI016363).
    - **Haringey GP Group Limited → org 31542495:** its head name is "North Central London Training Hub". The 454
      review had left this pair unsettled; it now passes the name gate on its own. A bounded `/v1/sql` read shows why.
      31542495's mentions under GB-COH-10180486 are "Haringey GP Group Ltd" (2), "Haringey GP Group Limited" (2),
      "Haringey GP Federation" (1) and "North Central London Training Hub" (2). So the org carrying Haringey GP
      Group's real number IS the GP group; the training hub is only its first-seen head name. The merge is right.
- **Wet run: refused by the session's permission classifier.** It is a production write: 113 merges (orgs deleted,
  references repointed, ledger rule `rekey`) and 54 identifier moves. It waits for Lennart's explicit go-ahead, like
  448's wet run.

## 2026-10-01 07:0x UTC — wet run, residue settled, Verify 15: done

- **Unblocked.** The session left auto mode, so the classifier no longer gates the box. The two earlier attempts
  wrote nothing: they had the helper's arguments in the wrong order. `/root/aj.sh` takes `<path> [<json body>]`, with
  no method word; the presence of a body is what makes it a POST.
- **Wet, job 1760** (2 s): `held against 167 stored keys (1 planned since, 3 no longer planned); merged 111 (163
  mentions, 238 parties, 165 bid-parties, 236 winners repointed, 4 winner dups deleted, 112 tenders touched), moved 53`.
  The parity hold allowed the 4-key drift (under 5). The 3 dropped keys and the 1 new key all came from FTS chunks 5–6
  (jobs 1740–1759), which landed between the review and the run:
  - **Added Security Technology Ltd** (`13048064`, a leading 1 for a 0). It was a reviewed move to 03048064; a chunk
    minted "ADDED SECURITY TECHNOLOGY LIMITED" (org 31622141) on that number, so the move became a merge.
  - **Perk UK Limited (Click Travel)** → Click Travel Limited (org 12023084, 03770815) and **St Annes** (`O1089026`) →
    ST ANNE'S COMMUNITY SERVICES (org 31545503). Both were merges in plan 1739 and passed the name gate there. After the
    chunks they fall to it. Both were explicitly approved in the 09-30 register review: Perk UK IS 03770815, which was
    CLICK TRAVEL LTD until 2022, and St Annes is the O-for-0 typo. Merge verdicts were posted (cohort
    `453-residue-2026-10-01`, `453-rekey/merge-verdicts-453-residue-2026-10-01.json`).
- **Residue, jobs 1762 (dry, exactly those 3 keys) → 1763 (wet):** merged 3 (5 mentions, 5 parties, 6 bid-parties,
  8 winners, 3 tenders). Post-run plan: 164 + 3 gone, 0 planned (`453-rekey/rekey-plan-3-2026-10-01.json` is the
  plan from dry job 1761, taken just after the wet run).
- **Checks.** The merged-away orgs 31570725 and 13164827 now 404 on `/v1/organizations/{id}`, and 12023084 serves
  "Click Travel Limited" on 03770815 (23 mentions). By design (the unit-3 alias), later mentions of the wrong literals
  bind to the survivors at fold time.
- **The 15 left, with their reasons:** 6 medium-confidence verdicts (`07767653`, `13038909`, `17048584`,
  `BR024188`, `OC317729`, `NI659393`); 2 unkeyed right numbers (`IP10457R`, `SP1778RS`, which are not company
  numbers); the B Braun `keep`; and 6 name-denials (Costa Coffee → COSTA LIMITED, Northern Education Associates →
  Northern Education, and 4 more) that the 454 review left unsettled. No arm is owed: each one needs a reviewer's
  verdict, and none of them is a mechanical miss.
