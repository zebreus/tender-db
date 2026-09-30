# 453 — 182 wrong-number organizations have a reviewer-found right number that nothing acts on

Status: ready-for-agent — ARM DEPLOYED 2026-09-30 16:5x UTC (`e156b88`); first dry plan (job 1726): 100 merge + 58 move, 16 name-denied (→ issue 454). REVIEWED against the Companies House register (below): 156 execute, 2 held and their verdicts corrected (Cochlear → 03874867, Montel → 08949189, cohort `453-rekey-review-2026-09-30`). NEXT: re-plan job 1738 (queued behind FTS chunk 4, jobs 1728–1737); diff its keys against the reviewed 156 + the two corrections, then the wet run, then the Verify. The flagged-by-number refinement (`19ea011`) deploys after the wet run.
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
- **done**: the unapplied count equals the recorded residue: 6 not high, 2 unkeyed, 7 name-denials the 454 review left
  unsettled, and 1 `keep` (B Braun). That is 16 with a reason each, and so reads 16.

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
