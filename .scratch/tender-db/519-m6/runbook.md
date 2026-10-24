# 519 M6 — box runbook: POST, dry/wet rekey, paired refold (one 551 class-W segment)

From `build-specs/519.md` §8–§9 with its Check fixes 1–16 applied. Prepared 2026-10-24; nothing here has been run.
Only the `/root/aj.sh …` lines run on the box (over ssh). The kit commands run locally from `$B6`, the `tools/*.py`
scripts locally from `$M6` (Check fix 7).

```
M6=/home/user/tender-db/.scratch/tender-db/519-m6
B6=/home/user/tender-db/.scratch/tender-db/551-lot-sum/b6
COHORT=519-m6-2026-10-24          # POST body: $M6/verdicts.json (sha256 cb9fff5f… after challenger C1/C2/C4, 245 rows; was e259a87a…)
HOLD=519-m6-hold-2026-10-24
```

Symbols: **P** = 245 rows posted; **s** = stale; **d** = M6 rows the dry plan denies (names / legal-form /
multi-target / pending-move); **N** = the final dry plan's key count (Check fix 6: not the registry's K);
**M**, **V** = merged, moved; **T** = tenders touched by the wet run.

## Preconditions (free; all must hold)

1. `ssh root@zebreus.click '/root/aj.sh "/admin/jobs?limit=20"'` shows the queue idle. No other agent's writer-lock
   line stands in 551. No `refresh-org-names` wet run is in flight (operations.md:3882-3883).
2. `cd $B6 && python3 -I -B u9/registry.py status ; echo "EXIT=$?"` exits 0.
3. Placement (fast plan step 8): after 582 U2 → 473/440 → **519 M6** → 583 U3; or between 577 slices, with M6's
   touched Tender ids added to 577's listed NOTE set before the next slice. (On 10-24 at 18:1xZ the 577 slices
   2773/2776/2778 were running or queued.)
4. No other arm's reviewed dry plan awaits its wet run (altid, r2, e0, nhs-body). If one does, run it again after
   M6 (Check fix 8): M6 withholds 245 GB-NHS orgs and deletes up to 87 merged orgs.
5. Freshness. R1/R2 were read 2026-10-24 18:15–18:17Z.
   - `post_body.py` already dropped nothing (0 gone). Any org that left since is caught by the POST's `stale`.
   - If an 08:35Z chain ran since then, re-read R2 for the merge orgs (≤ 4 statements; **7 of the 23 remain**) and
     rebuild the refold list with `tools/notices.py`.
6. Advisable (reading.md deviation 1): an independent agent challenger over the 102 high merge/move rows in
   `review.json`. Any row it disputes: set `challenger_agrees: false`, then rebuild the body:
   `cd $M6 && python3 -I tools/post_body.py review.json live.json $COHORT verdicts.json; echo "EXIT=$?"`.

## Step 0. Open the segment (local, from $B6)

Use the honest numbers: runs-left 1, 45 min per run (POST, dry, diff, wet, the project), 45 read-minutes.

```
cd $B6
python3 -I -B u6/health.py --label CB-w519-m6 ; ssh root@zebreus.click '/root/aj.sh "/admin/jobs?limit=100"' > u9/jobs/w519-m6_cb.json ; python3 -I -B u6/health.py --label CB-w519-m6b
python3 -I -B u9/segment.py open w519-m6 --class W --tag writer-519-m6 --cb-label CB-w519-m6 --cb-label2 CB-w519-m6b --jobs u9/jobs/w519-m6_cb.json --lock "Writer: 519 M6 (segment w519-m6) until <ISO>" --runs-left 1 --minutes-per-run 45 --read-minutes 45 --switchoff-gate <covered|uncovered> --expect-new 0 --expect-moves "0 R moves (identity merges of buyer/supplier rows; any R move is read)" --bar "class W: every NEW and every R move read" ; echo "EXIT=$?"
```

If K1 is not in yet and open refuses NO-GO, POST without opening and close it later as a gap (fast plan step 2b).
Never put two writers in one gap.

## Step 1. POST the verdicts

```
ssh root@zebreus.click mkdir -p /root/519m6
scp $M6/verdicts.json $M6/refold-notices.json $M6/jobs/rekey-dry.json $M6/jobs/rekey-wet.json $M6/jobs/project.json $M6/jobs/nhs-body-dry.json root@zebreus.click:/root/519m6/
ssh root@zebreus.click '/root/aj.sh /admin/identifier-verdicts @/root/519m6/verdicts.json' | tee $M6/post-answer.json
```

**Expected:** `{"recorded": 245, "stale": s, "changed": 245 − s, "cohort": "519-m6-2026-10-24"}`.
- s ≤ 3; 0 is expected.
- changed = P − s. No row is posted `right`, and no GB-NHS triple carries an earlier verdict: the 452, 466 and
  362 bodies hold no GBNHS literal. So every row flips from null to `register_mismatch`.
- A 400 means the whole upload was refused; read the message. post_body.py already checks each refusal the
  handler makes: identifier ≤ 64 characters, correction 1–64 characters, rationale ≤ 4,000 bytes (the largest is 639),
  and the enum fields.

## Step 2. Dry re-key, then hold the plan to the review

```
ssh root@zebreus.click '/root/aj.sh /admin/jobs @/root/519m6/rekey-dry.json'
#   poll /admin/jobs?limit=5 until the job is done, then:
ssh root@zebreus.click '/root/aj.sh /admin/reports/rekey-plan' > $M6/rekey-plan-dry.json
cd $M6 && python3 -I tools/plan_diff.py rekey-plan-dry.json review.json verdicts.json holds.json $HOLD > plan-diff.txt 2>&1; echo "EXIT=$?"
```

**Expected:** EXIT 0, `plan N keys; holds 0; stops 0`.
- N = 100 − d: 87 merges and 13 moves. d is the M6 rows printed as `NOTE … denied` lines; 0–5 is likely. Candidates:
  - 31627918: Care Quality Services → 14647720, which carries the number under the company's former name;
  - 31599268: Boots The Chemist → Boots UK Ltd;
  - 31544155: Mid Yorkshire Teaching → Mid Yorkshire Hospitals.
- The report counts the verdicts of every cohort, so only M6's share is predicted:
  - `unkeyed` grows by 14;
  - `not_high` grows by 23;
  - `several` and `gone` stay 0 for M6.
- Moves are company numbers only. No M6 PPON move is possible: the one ownerless PPON (PNNX5248NRVD) is already
  posted at medium.
- The exact keys are listed by the synthetic self-test, the M6 share of `review.json` with expected shape
  merge/move at high.

**EXIT 2** (not expected): POST the holds, then re-run step 2 and plan_diff until it exits 0.
```
scp $M6/holds.json root@zebreus.click:/root/519m6/
ssh root@zebreus.click '/root/aj.sh /admin/identifier-verdicts @/root/519m6/holds.json'     # expect "changed": 0
```
**EXIT 3:** stop and read every STOP line first.
- A key from outside M6 is an older cohort's key that drifted into the plan. Register-check it, or re-POST it at
  medium in its own cohort.
- A shape or target the review did not expect means re-reviewing that row. If it now reads medium, set it so in
  `review.json`, rebuild `verdicts.json`, re-POST, and re-run the dry.

## Step 3. Wet re-key

```
ssh root@zebreus.click '/root/aj.sh /admin/jobs @/root/519m6/rekey-wet.json'
```

**Expected:** `held against N stored keys (0 planned since, 0 no longer planned); merged M (… T tenders touched),
moved V, … residual 0 re-recorded`.
- M = 87 − (merge denials), V = 13, and M + V = N.
- Nothing else writes in the segment, so the max(2%, 5) parity cannot bite.
- No second pass is expected: no two M6 rows name the same ownerless number. If the dry listed `pending-move`, re-run
  step 2. Its keys must be exactly those rows as `merge:<the moved literal>`, and plan_diff skips them (Check fix 1).
  Then run step 3 again.

## Step 4. The paired refold (D6/D7)

First check whether 583 U2 is live:
```
r=$(curl -s --max-time 30 https://tenders.zebreus.click/health | jq -r .rev); git -C /home/user/tender-db grep -c 'issue 583' "$r" -- crates/store/src/canonical.rs || echo "$r: 0"
```
At 18:3xZ on 10-24, `/health` served rev **6cb4408**, deploy #3, which carries 583. The count should be ≥ 1, so
the "583 live" branch applies.

- **583 live:** `ssh root@zebreus.click '/root/aj.sh /admin/jobs @/root/519m6/project.json'`. The merge already
  stamped and re-queued its Tenders. Also record this project in 583 as its U3 "project after the next wet merge"
  (Check item 12).
- **583 not live:** `ssh root@zebreus.click '/root/aj.sh /admin/jobs @/root/519m6/refold-notices.json'` (136
  notices, one chunk ≤ 1,000). It queues `refold-notices` plus `project rebuild=false`.
  - If the final plan's merges differ from the review, rebuild the list from the final dry plan first:
    `cd $M6 && python3 -I tools/notices.py review.json verdicts.json live.json refold-notices.json rekey-plan-dry.json`,
    then scp it again.
  - Expected answer: `n notice(s) named: re-queued R, stamped S tender(s) … epoch-stale`, with n = 136, R ≤ n and
    S ≥ T. Match R and S with a regex: the text holds a run of spaces (Check item 16).
- **Either way, the project's `compare: X verified, Y corrected`:**
  - X + Y ≥ T, and Y ≤ the stamped count;
  - Y should be order-only (party, bid-party and winner row order);
  - any value, row-set or org-id move is read under the class-W rules at gd5 check.

## Step 5 (optional, informing). Dry nhs-body census

Skip this step if any nhs-body wet run is pending: it overwrites the stored nhs-body plan.
```
ssh root@zebreus.click '/root/aj.sh /admin/jobs @/root/519m6/nhs-body-dry.json'
```
Expected:
- `admitted_codes` [];
- `nhs_body_regrown` [] and `0 keys regrown (S4)`;
- 171 codes applied;
- `nhs-body` ledger rows unchanged at 306.

## Step 6. Close, read, check, advance (local, from $B6)

```
cd $B6
python3 -I -B u6/health.py --label CA-w519-m6 ; ssh root@zebreus.click '/root/aj.sh "/admin/jobs?limit=100"' > u9/jobs/w519-m6_ca.json ; python3 -I -B u6/health.py --label CA-w519-m6b
python3 -I -B u9/segment.py close w519-m6 --ca-label CA-w519-m6 --ca-label2 CA-w519-m6b --jobs u9/jobs/w519-m6_ca.json --own <dry id(s)>,<wet id(s)>,<project id or refold-notices id>,<project id>[,<nhs-body dry id>]
python3 -I -B u9/registry.py show                      # read the registry's K for --k (Check fix 6)
python3 -I -B u9/gd5.py read w519-m6 --k <registry K> --cursor-after CA --class W --jobs u9/jobs/w519-m6_read.json > u9/w519-m6_read.out 2>&1; echo "EXIT=$?"
python3 -I -B u9/ajobs.py build w519-m6 --from-seg w519-m6
python3 -I -B u9/ajobs.py count w519-m6 --summary … ; echo "EXIT=$?"
python3 -I -B u9/gd5.py check w519-m6 --k <registry K> --cursor-after CA --class W --a-from u9/ajobs/w519-m6/A.json --runs-left 0 --minutes-per-run 45 --read-minutes 45 --switchoff-gate <covered|uncovered> > u9/w519-m6_check.out 2>&1; echo "EXIT=$?"
python3 -I -B u9/registry.py advance w519-m6
```

**Expected:** NEW 0, MOVED/LOST/SILENT 0, band unchanged. Under the velocity rules:
- any NEW or R move is read;
- a read that comes back wrong raises the operator banner, then a fix: a 317 rehome of the misfiled mentions, or a
  re-POST plus a re-key;
- nothing freezes.

Clear the writer-lock line when the project ends.

## Verify and cohort read (§9, with Check fix 5)

```
curl -s -o /dev/null -w '%{http_code} %{redirect_url}\n' https://tenders.zebreus.click/v1/organizations/31601033
#   done: 308 https://tenders.zebreus.click/v1/organizations/30916123     open: 200
ssh root@zebreus.click '/root/aj.sh "/admin/case-reviews?table=identifier&cohort=519-m6-2026-10-24&limit=5000"' | python3 -c "import json,sys,collections; d=json.load(sys.stdin); print(len(d['rows']), collections.Counter((r['verdict'], r['confidence'], bool(r['applied_at'])) for r in d['rows']))"
```

**Expected:**
- Rows = P − s − H = 245 − s − H, where H = rows re-POSTed into the hold cohort (0 expected).
- Before any stale: ('wrong','high',True) = M + V (+ second-pass merges); ('wrong','high',False) = 222 − (M + V);
  ('wrong','medium',False) = 23.
- The hold cohort read, if any, has H rows, all medium and unapplied.

Spot reads (public, ≥ 2 s apart):
- merged losers answer `308` to their reviewed target:
  - 31601033 → 30916123;
  - 31617199 → 30913986;
  - 31627905 → 30974841.
- moved orgs serve the new identifier with `identifier_status` null:
  - 31627902 → `01323762`;
  - 31627922 → `CE003108`.
- withheld junk orgs serve `"identifier_status":"register_mismatch"`: 31618650 and 31559361.
- `/v1/organizations?identifier=GBNHSQHL15E` lists nothing.

## D9 fallback: QHL15E alone

If a writer slot opens before the full cohort is cleared, run steps 0–6 with:
- `verdicts-qhl15e.json`: cohort `519-m6-qhl15e-2026-10-24`, 1 row, high;
- `refold-notices-qhl15e.json`: QHL15E's 10 mention notices.

The dry plan must be exactly `["GB/national/GBNHSQHL15E>merge:GBPPONPWQY9673JBYN"]`. Expected POST answer:
recorded 1, stale 0, changed 1. Wet: merged 1. The full cohort's later POST then re-records QHL15E under
`519-m6-2026-10-24`, because the upsert moves the cohort.

## Not posted (D8): 3 rows over the 64-character cap

| org | literal | reading |
|---|---|---|
| 31623641 | 72 characters | wrong, none (a collaborative) |
| 31610156 | 68 characters | wrong; South Yorkshire ICB's PPON not read |
| 31610157 | 67 characters | wrong; West Yorkshire ICB PPON 30917022, medium |

They wait for the two-line cap raise (admin.rs:474 and :489, 64 → 128), then a 3-row POST.

## Records afterwards (.scratch only)

- `$M6/results.md`, with:
  - the POST answer and every dry and wet summary;
  - the plan-diff NOTEs (the residue list with reasons);
  - the refold or project answers and the gd5 verdict.
- Issue 519's Status line (spec §9 text), with P = 245, W = 245 − s, and C = 137 rows carrying a correction
  (114 high + 23 medium).
- The 551 segment entry `w519-m6`.
- Note in 524 that it is now unblocked. Also flag two heads to refresh, whose mentions name another body:
  - 30915589, heads BOB ICB, 96/100 mentions are Royal Surrey;
  - 30928165, heads "Bristol NHS Foundation Trust.", 36/41 are North Bristol.
