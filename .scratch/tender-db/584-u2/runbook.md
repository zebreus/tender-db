# 584 unit 2: runbook for applying cohort `584-2026-10-24` on the box

**What it does.** It moves 32 mentions off org 49 ("Krajowa Izba Odwoławcza") onto the organizations they name, through issue 317's reviewed rehoming:

POST `/admin/rehoming` → `apply-rehoming` dry (writes the `rehoming-plan` report) → read the plan tuple-wise → wet (held to that plan) → `project` → read-back.

It follows the precedents of 545 (dry 2650 / wet 2651 / project 2652) and 519 (2338 / 2339 / 2340).

| file | sha256 | content |
|---|---|---|
| `cohort.json` | `cad0149f2d43802bc5bdf3a35ee365de9f482a01bff50ca19b728edf72a233f8` | 38 verdicts, 23,228 bytes |
| `parked.json` | `ff0689de075600f18fdd6df64c2fcd0c9226220a4e45a2be2181557104189bdf` | 0 parked |

- **Verdicts:** 38, all `org_id` 49 and all `high`. 32 are `rehome`, each with a `target_org_id`, onto 15 targets. 6 are `keep`, with no target.
- **Targets:** 7788 ×13, 848 ×5, 14553663 ×2, then 1 each for 10689264, 18347269, 23202450, 15146954, 20111694, 8998814, 13575097, 18389587, 20491831, 976, 22583689 and 22456216.
- **Validation:** `final/validate.py` checks the cohort against `crates/app/src/admin.rs` `record_rehoming` (`:96–157`), against the `org_mention_rehoming` CHECKs (`canonical.rs:682`) and against `apply_rehoming` eligibility (`canonical.rs:33820–33880`).
  - No errors.
  - Every required field is present and typed. `action` ∈ {rehome, keep} and `confidence` ∈ {high, medium, low}.
  - Every rehome is eligible: it is high, its `target_org_id` is set, and the target is not 49.
  - No keep carries a target, and no (notice, section) appears twice.
  - The keys equal `584-u1/org49_foreign.json` exactly.
  - All 38 mentions sat on 49 in the challenger's live read (`c1`). Every `target_name` equals the target's live head name.
  - Lengths: section_id ≤ 8, rationale ≤ 556, target_name ≤ 74. The handler checks no lengths and the columns are unbounded TEXT. The body is about 1 % of axum's default 2 MB JSON limit.
- **Changed since the challenge:** wording in the 5 "National Appeals Chamber" keep rationales only. "(and Lot-Mediator)" was true only of 25398003; S2 shows Lot-ReviewOrg alone on the other four. `scripts/build_cohort.py` now emits the corrected text, and a rebuild is byte-identical to `cohort.json`. Verdict tuples are unchanged (diffed).

Run every box command from the dev machine as `ssh root@zebreus.click bash -s <<'EOF' … EOF`, so the JSON quotes survive (`docs/operations.md`). `aj.sh` has no method word: a body makes it a POST. Its output is the server's body; any non-2xx status exits 1 and is named on stderr.

## A. Copy the cohort to the box

```sh
scp /home/user/tender-db/.scratch/tender-db/584-u2/cohort.json root@zebreus.click:/root/rehoming-584-2026-10-24.json
ssh root@zebreus.click 'sha256sum /root/rehoming-584-2026-10-24.json'
```

**Expect** `cad0149f2d43802bc5bdf3a35ee365de9f482a01bff50ca19b728edf72a233f8`. Any other hash means the wrong file; do not post it.

## B. Pre-read (optional; informs, never blocks)

This is one admin GET. It is neither `/v1/sql` nor a public GET.

```sh
ssh root@zebreus.click bash -s <<'EOF' > /home/user/tender-db/.scratch/tender-db/584-u2/final/rehoming-table-before.json
/root/aj.sh "/admin/case-reviews?table=rehoming&limit=5000"
EOF
python3 -I /home/user/tender-db/.scratch/tender-db/584-u2/final/precheck.py \
  /home/user/tender-db/.scratch/tender-db/584-u2/final/rehoming-table-before.json \
  /home/user/tender-db/.scratch/tender-db/584-u2/cohort.json
```

**Expect** both of these:
- `pending high rehomes with a target OUTSIDE 584-2026-10-24: 0`;
- `cohort keys present 0 of 38`.

Anything else predicts what step D will show:
- Every "will also be planned" row will appear in the dry plan as an extra tuple.
- A cohort key that is already **stamped** cannot be re-applied by this POST. `record_rehoming` keeps a real move's stamp (`canonical.rs:33735–33739`). See F.

## C. Record the verdicts

```sh
ssh root@zebreus.click '/root/aj.sh /admin/rehoming @/root/rehoming-584-2026-10-24.json'
```

**Expect** exactly `{"recorded":38,"cohort":"584-2026-10-24"}`. The 6 keeps are recorded too; `recorded` counts every verdict.
- A 400 names the failing `notice/section`, and nothing is written: the handler validates every row before the transaction.
- Recording is an upsert on (notice_id, section_id). Re-posting the same file is harmless, and it clears only a `no-op:` stamp.

Confirm the read-back:

```sh
ssh root@zebreus.click bash -s <<'EOF' | python3 -c 'import json,sys; d=json.load(sys.stdin); r=d["rows"]; print(d["count"], sum(x["applied_at"] is None for x in r), sorted({x["action"] for x in r}))'
/root/aj.sh "/admin/case-reviews?table=rehoming&cohort=584-2026-10-24&limit=100"
EOF
```

**Expect** `38 38 ['keep', 'rehome']`: 38 rows, all unapplied.

## D. Dry run, then read the plan tuple-wise (545's way)

```sh
ssh root@zebreus.click '/root/aj.sh /admin/jobs "{\"kind\":\"apply-rehoming\"}"'
```

This answers `{"enqueued":[J1]}`. `dry_run` defaults to true for this kind (`supervisor.rs:4648`). The queue serializes it behind any running job.

When J1 has finished, read its line:

```sh
ssh root@zebreus.click '/root/aj.sh "/admin/jobs?limit=50"' | jq -r '.recent[] | select(.job_id == J1) | .outcome, .counts'
```

**Expect** `ok`, then this line (`supervisor.rs:15849`):

> apply-rehoming (issue 317 Unit A) DRY RUN — plan recorded, nothing written: **P** pending verdicts, **32** eligible; **32** mentions would move (**≥186** party and **19** bid-party rows follow them across **18** tenders, whose causing notices the wet run will re-queue so the fold re-derives the winners); **0** no-ops, **M** name a target org that does not exist or no target at all (v1 never mints one)

| field | expected | why |
|---|---|---|
| P (pending) | **219** if no rehoming verdict was recorded or applied since 545's wet 2651; otherwise the prior pending + 38 | `apply_rehoming` reads every unapplied row of every cohort (`canonical.rs:33847`). Keeps are never stamped. 545's plan 2650 showed 243 pending; 2651 then stamped its 62 moves, leaving 181 |
| eligible | **32** (+ step B's "will also be planned" rows) | high + rehome + target |
| moves | **32** | |
| parties | **≥ 186** | The 32 mentions' own rows are 186 (55 on a current version). The count is by (notice, origin org 49) (`canonical.rs:33939`), so KIO's own org-49 sections on 17 of the notices add rows. Those rows are rebuilt unchanged, not moved |
| bid_parties | **19** | one per supplier section |
| tenders | **18** | 78715, 134851, 136020, 174615, 241175, 306091, 313839, 418730, 445071, 497408, 572868, 614155, 655893, 824617, 914542, 990702, 997224, 1092955 |
| noop | **0** | All 38 sat on 49 in `c1`. A no-op means a mention moved since; see F |
| missing_target (M) | unchanged from the prior baseline: **4** at 545's plan, from 519's PMO verdicts | this cohort adds 0 |

**Read the plan.** 545 saved `/admin/reports/rehoming-plan`, `json.loads`-ed its `body` (a JSON string) and compared the `plan` tuples to the cohort. Do the same here:

```sh
ssh root@zebreus.click '/root/aj.sh /admin/reports/rehoming-plan' > /home/user/tender-db/.scratch/tender-db/584-u2/final/rehoming_plan_J1.json
python3 -I /home/user/tender-db/.scratch/tender-db/584-u2/final/plan_check.py \
  /home/user/tender-db/.scratch/tender-db/584-u2/final/rehoming_plan_J1.json \
  /home/user/tender-db/.scratch/tender-db/584-u2/cohort.json; echo "PLAN-EXIT=$?"
```

**Expect** `cohort rehome tuples 32; plan tuples 32 (distinct 32); extra 0; missing 0`, then `PLAN == COHORT` and `PLAN-EXIT=0`.
- **Read the `PLAN-EXIT` value**, not a grep.
- Check that `age_seconds` is younger than J1, so you are not reading an older plan.
- The plan **is** the wet run's contract. The wet job loads the latest `rehoming-plan` and refuses unless it computes the identical tuple list (`supervisor.rs:15769–15800`, `canonical.rs:33980`).

## E. Wet run and the paired project

Run this only when step D printed `PLAN == COHORT`. Then enqueue both jobs back to back; the queue serializes them, so they hold one writer at a time.

```sh
ssh root@zebreus.click '/root/aj.sh /admin/jobs "{\"kind\":\"apply-rehoming\",\"dry_run\":false}"'   # {"enqueued":[J2]}
ssh root@zebreus.click '/root/aj.sh /admin/jobs "{\"kind\":\"project\"}"'                             # {"enqueued":[J3]}
```

**Wet J2, expected:** `ok` with

> apply-rehoming (issue 317 Unit A): **P** pending verdicts, **32** eligible; **32** mentions moved (**≥186** party and **19** bid-party rows follow them across **18** tenders, whose **R** notices are re-queued so the fold re-derives the winners); **0** no-ops, **M** …

- R is at least 32: every notice that built one of the 18 tenders.
- After J2, the 32 rows carry `applied_action = "rehomed from 49"` with `job_id` J2. The 6 keeps stay unapplied, as 545's 12 do.
- **If J2 ends `error` with "rehoming ABORTED: the reviewed plan holds … moves, this run computes … and they are not the same moves":** nothing was written. Something changed between J1 and J2. Re-run D, re-read the plan, then wet again. J3 then runs harmlessly on whatever else is queued.

**Project J3, expected:** `ok`. It re-derives the re-queued notices. Expect up to 18 tenders corrected.
- For comparison, 545's 48 tenders gave 47 corrected and 1 verified.
- The counts include any other notices pending in the queue at the time.

## F. Read back (Verify), and the failure modes

**1. Mentions: one bounded `/v1/sql` statement.**
- It is 38 index seeks on `organization_mentions_notice` (`notice_id = ?`, then the section filter), plan-probed locally: `final/readback.plan` shows 38 SEARCH steps and 0 SCAN.
- Send it on stdin only. Never retry a 408.

```sh
python3 -I - <<'PY' > /home/user/tender-db/.scratch/tender-db/584-u2/final/readback.out
import subprocess, sys
sql = open('/home/user/tender-db/.scratch/tender-db/584-u2/final/readback.sql').read()
p = subprocess.run(['ssh', 'root@zebreus.click', '/root/sq.sh'], input=sql, capture_output=True, text=True, timeout=120)
sys.stdout.write(p.stdout); sys.stderr.write(p.stderr)
PY
python3 -I /home/user/tender-db/.scratch/tender-db/584-u2/final/readback_check.py \
  /home/user/tender-db/.scratch/tender-db/584-u2/final/readback.out /home/user/tender-db/.scratch/tender-db/584-u2/cohort.json; echo "RB-EXIT=$?"
```

**Expect** `38/38 as expected` and `RB-EXIT=0`. That means each rehome sits on its target, each keep on 49, and nothing on 36.

**2. Tenders: public GETs.** Run them sequentially, `sleep 3` between them, and never retry a 408 or 429.

```sh
curl -s https://tenders.zebreus.click/v1/tenders/306091 | jq -c '[.parties[] | select(.role=="Procedure-Buyer") | .organization_id]'
#   expect [7788]                    (issue 584 Verify; was [49])
sleep 3; curl -s https://tenders.zebreus.click/v1/tenders/914542 | jq -c '{t:[.parties[]|select(.role=="Tenderer")|.organization_id]|unique, w:[.lot_results[].winners[]?.organization_id]|unique, b:[.bids[].parties[].organization_id]|unique}'
#   expect {"t":[13575097],"w":[13575097],"b":[13575097]}      (WTU; was 49)
sleep 3; curl -s https://tenders.zebreus.click/v1/tenders/997224 | jq -c '[.parties[]|select(.role=="Procedure-Buyer")|.organization_id]|unique'
#   expect 848 present beside GDDKiA's org, 49 absent           (MPWiK co-buyer)
sleep 3; curl -s https://tenders.zebreus.click/v1/tenders/241175 | jq -c '[.bids[].parties[].organization_id]|unique'
#   expect 18347269 and 15949738 (J&M lead + BICO partner), 49 absent
```

On every tender read, `[.parties[]|select(.organization_id==49 and (.role|test("Review|Mediator")|not))]|length` should be 0. KIO's own review-role rows legitimately stay on 49 or 36.

**Budget.** The read-back needs 1 `/v1/sql` statement and 4 GETs. This workflow has used 8 of 10 statements and 24 of 40 GETs, so it fits.

**Failure modes:**

| symptom | meaning | action |
|---|---|---|
| D shows an EXTRA tuple | another cohort's high rehome is pending (step B lists it) | Do not run the wet job blind: it would apply that cohort too. Apply only if that cohort's issue records it as reviewed and ready; otherwise settle that cohort first and re-run D |
| D shows a MISSING tuple, or noop > 0 | the mention left 49 since `c1`, or its target row is gone | Read it with the F1 statement. Correct the verdict and re-POST: a `no-op:` stamp clears on re-record. Then re-run D |
| F1 shows a rehome on **36** | the refold re-resolved it: a published fact changed, so `publishes` failed, and E0 picks org 36 (`canonical.rs:21255`, lowest id) | **A re-POST will not move it.** The real-move stamp is kept on re-record (`canonical.rs:33735–33739`), so `apply_rehoming` never sees the row as pending. Record it in 584 for org 36's cohort. (545's read-backs persisted 11/11, so this is unlikely) |
| F1 shows a rehome still on **49** | J2 did not apply it | Check J2's outcome; re-run D |

## G. After the apply (for the issue, not box steps)

- 584's Verify line (306091 → 7788) is then **done**. **584 does not close**, because three items stay open:
  - **Org 36:** same triple, lower id, 205,496 mentions. It is where every new KIO-NIP bind lands, and it holds at least 133 foreign-named tenderer mentions on 91 tenders (`challenge-raw/c2_org36_view.txt`). Its buyer side is not sized. It needs a census job (it is beyond the `/v1/sql` budget) and a second cohort under 545's rules, with consortium and sister-company parks (for example "REMONDIS KROeko" is not Remondis Bydgoszcz).
  - **The 36/49 duplicate:** a merge decision. A merge un-poisons the canonical key (`guard-spec.md` §1 Scope), so it should not precede the guard.
  - **The guard decision:** on the revised `guard-spec.md` (E2 plus the mandatory role condition) and the §6 census over 36 ∪ 49.
- **Unmerged twins found while reviewing** (not this cohort): Zarys 976/9241, Remondis 15146954/20376473 and Digitex 20111694/19005947.
- **Out of scope here, still W:** tenders 816754 and 1058540, where the tenderer section prints KIO's own name, and the mirror shape on org 11786.
