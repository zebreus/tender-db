# 551: the fastest safe plan under the owner's velocity ruling (2026-10-24, written 11:55Z)

This file comes from the hold audit (wf_76242a01-d9c). Its inputs were decision-w-on-drains.md, u9/RUNBOOK-W.md, 551's U9
and W-on sections, the Status lines of 546/561/565/572/573/575-583/519/552/521/527, the kit code, and git/marker/disk state.
Writing this file was the audit's only action: no cargo, no box access, no git writes.

Only the holds the audit kept are respected here: P07 (shrunk), P10, P11 (narrowed), P13, P15, P25, P26 (the bracketing
only), P27, P29, P33, D04 (narrowed) and D10 (the order). So are the hard constraints:
- one writer on the box at a time;
- no heavy box work during the 08:35Z chain or the Sunday 02:10Z weekly jobs;
- /v1/sql ≤ 300 statements an hour;
- a green gate on the deployed tree;
- the twin rebuilt on each deployed base (C7);
- no edits outside .scratch while a gate runs.

## 0. State at 11:55Z (this differs from the task text)

- **The batch is deployed:** /health rev a63cf23 since 11:32Z. Its tree equals ed3e2d5 outside .scratch: 539 ce20b28,
  576 U2, 578 U2, 579 U4, 580 U2, 572 U2, with 577 U2 still reverted. The marker reads `gate-v2 ed3e2d5`.
- **P1 PASSED** (11:34–11:47Z): 66 corrected, NEW 0, 0 R moves, pins PASS, band 29. K = **959250675**, no segment open,
  not frozen. 539, 578, 579 and 580 are DONE.
- **576 U3 is running:** refold-sections Touchpoint 2758 and project 2759, posted 11:47Z. **No segment was opened and no
  writer-lock line was written.** The kit will see these two jobs as a gap.
- main = 09bf1bd. 577 U2 is re-applied (bdd055d) but neither gated nor deployed.
- Job files are written:
  - `546-u4/refold_guard_probe.json`: a refold of r209 with expect 1, which reports the true count and writes nothing.
  - `577-u3/reparse.json`: **all 19 profiles in one job, unsliced.**
- **The working tree is dirty outside .scratch.** The operator-notice banner is being built: `crates/app/src/notice.rs`
  plus 13 files, among them the flip files CHANGELOG.md, openapi.json, v1/docs.rs and docs/operations.md. No gate can
  start, and no twin can be checked out, until that work is committed.
- **The switch-off is NOT covered.** 111f1a7 (e718684 reverted on ed3e2d5) is right for a63cf23, but it is not gated, and
  it goes stale at the next deploy. adcb7b0 is no longer a usable switch-off: it sits on 512b297, so deploying it would roll
  back the batch (C7), and the marker no longer covers it. This corrects verdict B02.
- Disk has 4.6 GB free (W13' needs ≥ 4 GB). Neither 577 U2 nor the banner work changes a Cargo file or a profile, so the
  gate will not auto-clean.
- Next box events: the weekly tick, Sun 10-25 02:10Z (about 2 h of jobs), then the chain, 10-25 08:35Z.
- The C8 population is 3 heads: 97901, 8859070 and 8859315. chain-1024 added none.

Anchors used below: T576 = 576 U3's project end (posted 11:47Z, 1.5–2.5 h, so about 13:15–14:15Z). T546 = 546 U4's
project end.

## 1. The plan, in order

### Step 1. Now, while 576 U3 runs (11:55Z → T576). Nothing here writes to the box.

**1a. Writer-lock line (P07, kept).**
- Do: write "Writer: 551 576-u3 (jobs 2758/2759) until 14:30Z" in 551. Clear it when the project ends.
- Verify: /admin/jobs shows 2758/2759 as running or queued.

**1b. Kit amendments K1–K4.** These are .scratch-only, so they can be made during a gate. They are critical: they must land
before T576.
- K1, `u7/start_rule.py`: GO if now + the job-and-project UPPER bound ≤ the next tick (chain or weekly). Drop the 2 h
  margin, the read minutes and the switch-off-gate minutes.
- K2, `u9/registry.py status`: pending NEW reads and D7-in-flight become WARN lines. They never HOLD and never time-escalate
  to SWITCH OFF.
- K3, `u9/segment.py open`: drop the DAY7_DATE hold (`wkit.DAY7_DATE = None`). The weekday catch-up refusal applies only to
  jobs over 2 h.
- K4, `wkit`: readers 125 → 225/h and the box 150 → 250/h. The switch-off box keeps ≥ 25/h.
- Verify:
  - `python3 -I -B u9/test_w.py` and `u6/test_kit.py --quick` both pass;
  - KIT_W_SHA256SUMS is re-recorded and `sha256sum -c` exits 0 (P33);
  - the changes are recorded in 551.
- Why it is critical: under the current rule, 546 U4 (M = 630, twin uncovered) is NO-GO for any open after 12:35Z, and it
  is now behind 576.
- Stop rule: if K1–K4 are not in by T576, run 546 U4 by the gap route (step 2b). Do not wait.

**1c. Banner.**
- Do:
  - commit the banner work (focused test with the gate flags, then `git diff` each file before staging);
  - then **gate A** on main: 577 U2 + CLAUDE.md + the banner, about 25 min;
  - announce gate A in 551 so that nobody edits outside .scratch while it runs.
- Verify: GATE-EXIT=0 read from the log file, never through a pipe; marker = `gate-v2 <sha-A>`.
- Stop rule:
  - Red gate: fix it and re-gate. 546 U4 does not wait for this.
  - Banner not committable by about 16:00Z: park it as a .scratch patch and gate 577 U2 alone, because 577 U3 needs 577 U2
    live.

**1d. Day-7 read, now (see §3).**
- Do:
  - `day7.py build --allow-not-chain --out u9/day7-early`. It is allowed now because no segment is open.
  - Record the predraw in 551, then draw (salt `551-b6-daily-`).
  - Fetch with `fetch_b6` (public GETs, 0 /v1/sql).
  - A reader and an adversarial skeptic per head.
- Verify: 3 of 3 read, 0 wrong.
- Stop rule: a wrong head means a banner once it is live, plus a fix or a switch-off. The queue keeps running.

**1e. Closures and sizing.**
- 572: run R3 (`/docs` grep 'issue 572' ≥ 1) and M0 (2 SQL + 21 GETs), then close it.
- 576 U3 sizing: the bounded Touchpoint COUNT (1 statement) is now only an after-the-fact record.
- 577 U3 slice sizing: either per-profile bounded counts (about 19–40 plan-probed statements spread over 546's run), or
  skip them and size from slice 1's measured rate.

**1f. Builds and reads in parallel.** Code edits happen only between gates; drafts go in .scratch during gates. Details per
issue are in §2.
- Builds: 581; 575 U1; 573 U3; 561 U3 (markdown) and U4; the 521 census arm; 552's operations.md re-base (after the
  banner commit, which touches the same file); the P14 marker ledger in `ops/gate-marker.sh`; the C9' prediction builder.
- Bounded reads: 582 U1, 527 U3, 565's re-baseline, 473 U1.
  - While a kit reader runs, keep these reads at ≤ 50/h. Keep the total ≤ 300/h.
  - Send none of them during the chain or weekly jobs.
  - Never re-send a 408'd statement (P25).

### Step 2. At T576 (about 13:15–14:15Z): close 576 U3, deploy #2, open 546 U4

**2a. Close 576 as a gap.**
- Do:
  - read the CA labels twice around a fresh `/admin/jobs` body;
  - `segment.py gap gap-576u3 --reason "576 U3: refold-sections 2758 + project 2759, posted 11:47Z without open (owner ruling)"`.
    Exit 2 is expected.
  - `gd5.py read gap-576u3 --class Z --full`: E1 over about 100–170k tender rows, 25–40 statements, about 10 min at
    225/h.
  - `gd5.py check --explained`, with foreign_jobs, z_rows and z_corrected written as a class-N drain;
  - `registry.py advance`.
- Verify: NEW 0, R MOVED/LOST/SILENT 0, pins PASS, band 29, corrected about equal to the carriers' Tenders.
- Stop rule:
  - A NEW head or an R move: read it and port it on refetched facts. A wrong head means a banner and a fix (revert 576 U2
    and run refold-sections again), or a switch-off.
  - 546 U4 still opens: it is class N and independent of 576.

**2b. Deploy #2, if gate A is green.**
- Do: `./deploy.sh <sha-A>`, pushed by explicit ref. It runs while 2a's reads run; the queue is idle, which P29 requires.
  The restart blip is handled by the Box as transport, so the statement is re-sent after health.
- Record a code-drift ledger line in 551:
  - 577 U2's fold-time hunks are TEXTS entries and a BT-749 filter: text facts, not W columns;
  - the banner is app-only;
  - so there is **no probe** (§4): 546's gd5 reads R ∪ Rg ∪ pins anyway.
- Verify: /health rev = sha-A; `/v1` and `/health` serve `notice` (empty); the 577 Verify target 47414082 PROCEDURE still
  serves a label (the full Verify is 577 U4).
- Stop rule: deploy.sh refuses, or the build fails. Then skip; deploy #2 moves to step 4's seam. 546 is not held.

**2c. Open 546 U4 as soon as 2a advances.**
- Do:
  - `segment.py open w546-u4 --class N --tag drain-546-u4 --minutes-per-run 480 --switchoff-gate uncovered --expect-new 0 --expect-moves "0 R moves; place/country NOTEs; 579's 6 order-only NOTEs"`;
  - one lock line;
  - POST the guard probe (expect 1), which refuses and reports the count;
  - check the count against 546's estimate (about 4.5M) and the cohort-versus-profile rule;
  - POST `{"kind":"refold","profiles":["ted-export-r209"],"expect":<count>}`, which queues the refold and its project.
- Without K1, open refuses with NO-GO. Then POST without opening and close it later as a gap, exactly like 2a. Never two
  writers in one gap.
- Verify: the refold's count equals expect; the project is ok.
- Stop rule:
  - An expect mismatch refuses by itself: re-read the count, never force.
  - **Never cancel the refold or the project (P10).** If it overruns 02:10Z, the weekly jobs queue behind it (FIFO);
    record that.
- Expected T546: open + about 4.5 h (532's rate), so about 18:15–19:15Z. The upper bound is open + 8 h, about
  21:45–22:45Z.

### Step 3. During 546 U4 (local work; no box writer)

**3a. Build twin-4 now.**
- Do: cherry-pick 111f1a7 onto sha-A as `hold/551-switchoff-4`.
- Verify:
  - its diff against sha-A outside .scratch is e718684 inverted (CHANGELOG aside);
  - it sets `LOT_SUM_AWARDS_PATH = false`;
  - it keeps the banner, 577, 580, 576, 539 and 572 hunks in the flip files (C7, P13).
- Gate it only if no gate B is coming before T546. A switch-off cannot deploy before 546's project ends anyway (P29), so
  coverage during a corpus job buys nothing until its last half hour.

**3b. Gate B, at about T546 − 45 min** (watch the refold's progress counters).
- What goes in: whatever is committed and leaves the fold alone: 581, 575 U1, 573 U3 (only if confined to the
  refresh-org-names planner), 561 U3's text, 552's operations.md, the 521 census arm, P14.
- What stays out: the fold-changing commits (575 U3, 561 U4, 583 U2). They wait for 577 U3's last slice (§2: D10's harm).
- Never two gates at once (P15): one checkout and 4.6 GB.
- Stop rule: gate B red. Then deploy nothing new at T546 (sha-A is already live) and gate twin-4 instead.

**3c. Optional kit work, if time allows:**
- K5: split read. At CA, take only the state snapshot (R ∪ Rg ∪ pins in 2 statements, the band in 1), then let the next
  writer open; E1 over the closed range runs alongside.
- O1: the public `/v1/changes?entity=tender` enumeration for 546's read.
- O2: the switch-off reuses a completed E1 read.

**3d. Preparation:**
- 582: the U2 rehome plan and its review;
- 519 M6: the cohort review (a reader and a challenger);
- 583 U1 and 473 U1;
- the 577 slice plan, and O3 recorded in 577 (see step 5).

### Step 4. At T546 (typically about 18:15–19:15Z): close 546, deploy #3, gate the twin

**4a. Close and check 546.**
- Do:
  - `segment.py close w546-u4`;
  - `gd5.py read --class N`: E1 150–380 statements, about 40–100 min at 225/h (72–182 min without K4). With K5 the next
    writer goes after the snapshot instead;
  - `check`, then `advance`.
- Verify: NEW 0; R moves 0; band 29; place moves and 579's 6 as NOTEs.
- Stop rule:
  - Any NEW: read every one, as enumeration misses.
  - More than 30 NEW: a banner, and a salted 30 under U5's bar, read in parallel (P06 amended).
  - An R move: read it and port it.
  - A wrong head: a banner, then a switch-off or a fix.

**4b. Deploy #3** (sha-B), if gate B is green.
- It overlaps 4a's reads, and no probe is needed: none of its commits touches a W fold path.
- Record the ledger line. Close 581's Verify once a retirement has happened.

**4c. Gate the twin for the deployed base** (twin-5 on sha-B, else twin-4), right after 4b.
- 577's slices are short, so a switch-off can deploy between them from here on, and coverage now matters.
- Verify: `marker_covers <twin>` prints COVERS and `gate_deploy_step <twin>` prints skip.

**4d. 546 Verify.**
- 8432519 serves ['FRK22']; 20297187 no longer matches ?country=DE.
- Then commit the caveat retirement (RETIRED "issue-546 refold"); it rides the next gate.

### Step 5. When 546 advances (typically about 19:00–20:30Z; upper about 23:00–00:30Z): 577 U3 in slices

The order is kept (D10): an undrained 546 or 576 change would trip 577's 0 bar falsely and hide a real mapping gap.

**5a. Record the O3 reading in 577 first.**
- The slice bar becomes "0 corrections outside the NOTE classes".
- The NOTE classes: place-only and order-only corrections, and an explicitly listed id set (the Tenders touched by any
  writer run between slices).
- **Do not POST `577-u3/reparse.json` as it stands.** Nineteen profiles (about 3.3M notices) in one unbounded job would run
  across 02:10Z and 08:35Z with no way to stop it (P10, P11).

**5b. Slice 1: de-1.0, de-1.1 and de-1.2, with a `packages` cap** (about 20–30 min).
- Each slice is its own segment, class Z: open, POST `{"kind":"reparse","profiles":[…],"packages":N}`, close, the Z
  shortcut (corrected = 0 and 1 E1 statement), advance.
- Measure notices per minute from it. **This slice is W7's dynamic discharge (§4).**

**5c. Slices 2 to n, back to back:**
- the rest of de-1.x, then de-2.0/2.1, then the sdk-1.x groups;
- each sized from the measured rate to end before the next tick (tonight 02:10Z);
- a few minutes between slices.

Stop rule for any slice with corrected > 0:
- 577's own stop for that slice;
- `gd5.py read/check` class N on the slice;
- the E1 tender-row ids are compared with the NOTE set;
- a correction outside it is a 577 mapping gap: stop the slices, read those Tenders, then fix 577, or revert 577 U2 (deploy
  the previous gated sha) and re-parse the slice under the old parser. That restores the rows (the raw notices are stored),
  so the step can be undone;
- an R head MOVED wrong means a banner and a switch-off (the twin is covered from 4c).

### Step 6. 10-25, 02:10Z → about 04:10Z: the weekly window (P11)

- No writer job may still be running at 02:10Z.
- Close `wk-1025` (class weekly, 0–1 statement) as soon as /admin/jobs is idle.
- Resume the slices right away (about 04:15Z). Each must end before 08:35Z.

### Step 7. 10-25, 08:35Z: chain-1025 (D04, narrowed)

- No writer from the tick until the chain's advance: about 10–20 min (Sunday, so no catch-up loop).
- Run the check the moment the queue is idle.
- Read the chain's fires the same day (the rolling day-7 read, §3).
- Then resume the slices until 577 U3 is done, then run 577 U4 (R3–R6 and the Verify).

### Step 8. After 577 U3's last slice (10-25): fold-changing deploys and in-place writers, back to back

**Deploy #4:** 575 U3, 561 U4 and 583 U2, each as it gates.
- POST a P-probe as the first job after the deploy (refold-lot-sum over R ∪ Rg ∪ pins, about 15 min, class D, informing),
  while the twin re-gates in parallel. 561 also runs its U5 canary before the next chain.

**Then the writers.** Each is its own segment, closed by gd5 before the next opens.
- The order: 582 U2 → 473/440 runs (after 582) → 519 M6 → 583 U3 → 575 U4 windows.
- In-place writers pair with a refold-lot-sum of the touched cohort Tenders in the same segment. Dissolves have this built
  in through 579 U4.
- Check as class I with a prediction file, else class W (read every NEW and every R move).

**Allowed earlier, between 577 slices,** as their own short segments:
- 575 U1's dry census, 573's dry re-plan and wet 15, 527's dry plan and wet 2-id job, 521's dry census (class Z if dry);
- 582 U2 or 519 M6, if their reviews are done early, with their touched Tenders added to 577's listed NOTE set before the
  next slice.

### Step 9. Daily, while W is on

- Run the chain check at queue-idle; read the new fires that day.
- 10-28: the day-7 tally (§3).
- 11-04: decide whether to retire the twin (each deploy pays a 25-min twin gate until then).

## 2. Per issue: build now, ship at the next seam

| Issue | Now (10-24) | Box step and earliest slot |
| --- | --- | --- |
| **581** (successor ledger, 308/410; not a W input) | Build now (store retire path). Gate B. | Deploy #3 at T546; no probe, because it touches no W fold path. It is time-sensitive (no backfill: 546's and 577's regroup retirements lose successors until it lands), but do not hold the refolds for it. Verify after the first retirement. |
| **582** (5,076 pre-358 provisional rows) | U1 bounded read now (1–2 statements; it gives the touched list); R ∩ touched offline; U2 plan and review. | U2: apply-rehoming paired with a refold-lot-sum of the touched cohort Tenders, class I (prediction) else class W. Right after 577 U3, or between slices with its ids listed. Before 473's runs. |
| **583** (merge order drift) | U1 now (code reading + a bounded read). U2 build (stamp the Tenders `repoint_org_references` touches as stale). | U2 deploy after 577 U3 (it changes refold scope), then the probe. U3 cohort refold, class W, right after. |
| **519 M6** (452 verdicts) | Cohort review now (reader + challenger). Rewrite its Status line: drop "Blocked by: 551 C9' (after 11-04)". | One segment, the writer + a paired refold-lot-sum, class I/W. After 577 U3, or between slices with ids listed. |
| **575** | U1 build now (a new Spec arm: `Box::pin` it; supervisor.rs is a flip file, so the twin carries it). Strike "Blocked by: 539" and the 560/579 conditions. | U1 in deploy #3; its dry census as a class-Z segment between 577 slices. U2 decided the same day. U3 build/gate, then deploy #4 after 577 U3, with the probe. U4 dissolve windows right after (579 U4 is live, so the refold is built in), class I/W. |
| **561** | U3 (markdown) now. U4 build now (no box involved, so 551 cannot "mix"). | Deploy #4 after 577 U3 (projection epoch; keep it out of 577's 0 bar), then the probe, then the U5 canary (about 1,000 ids, corrected 0, values_refreshed > 0), all before the next chain. |
| **573** | U3 build now (R0d epoch). | Deploy #3 if it is confined to the planner, else deploy #4. Dry re-plan right after: exactly the 15 R0d changes. The wet 15 ids in the next writer slot. |
| **527** | U3 measurement now (bounded SQL). | Dry refresh-org-names [30533823, 30533824] in a seam; the wet 2 ids in the next slot. |
| **521** | Census arm build now. | Next deploy; the dry census in the next seam; the per-word table the same day. |
| **565** | Re-baseline reads now; decide the same day. | A build only if A-wide or fan-out wins. |
| **552** | The operations.md re-base after the banner commit (same file). | Rides gate B. |
| **546 / 576 / 577** | See steps 2–5. | 576 U4: the Verify (8851944, 8851996) + 30 minted parties read, right after 2a. |

## 3. Day 7: it can run now

The population is built from registry tags, not from the date. `day7.py` takes the `chain-*` entries only, so drain and
probe fires never enter it. Today that is 3 heads (97901, 8859070, 8859315) and 0 chain-MOVED.

- With fewer than 30 heads the bar is "read all, 0 wrong". So reading the 3 now and then each chain's fires on the day they
  are registered gives the same verdict as one read on 10-28.
- A head re-decided later is re-read as chain-MOVED.
- On 10-28 (and on 11-04) only tally. Draw the salted 30 only if the population passes 30. Reuse the reads already done for
  any drawn head.
- `build` refuses while a segment is open, so run it in a seam: right now, before 546 opens.
- Use `--out u9/day7-early` until K3 removes the 10-28 hold.

## 4. W7: can a dynamic check replace the static hold? Yes, for the stock, and measured exactly.

- **A refold-lot-sum probe cannot see 577's parser.**
  - A refold re-projects stored parse rows and never runs the new parser.
  - 577 U2 also has a fold-time half (project.rs TEXTS entries and a BT-749 filter). A refold does see that half, but it
    makes only text facts, outside the W columns.
  - So a probe before the re-parse is the identity on W columns by construction, and not worth a box slot.
- **After the re-parse, the slice's own paired `project` IS the refold under the new parser** for every Tender in the slice.
  - Its corrected count is exact over the whole slice, which is stronger than R alone.
  - The slice's gd5 reads R ∪ Rg ∪ pins by PK (W columns, tolerance 0), the band, and E1 for NEW.
  - So slice 1 = **de-1.x first**: the only profiles where W7 found code moves (BT-27-Lot, BT-271, BT-21).
  - 0 corrected there discharges W7 empirically.
  - A separate dry run or probe after the re-parse adds nothing, except the A count on a slice that corrects.
- **New daily de-1.x notices are parsed by the new parser from deploy #2 on.**
  - Slice 1 normally runs before the 10-25 chain, so no chain folds under the new parser before the stock verdict.
  - If one does, its class-chain gd5 counts fires and MOVED R heads: the instrument that already watches every daily fold.
- **A pre-deploy local fold comparison on real notices is not available here.**
  - No public route serves raw notice XML (only `/v1/notices/{id}` and `/content`).
  - `reparse` has no dry mode.
  - What exists is the fixture version inside the gate: project_golden, doe.rs
    `eforms_de_1x_lot_values_bind_the_lot_ids` and the eforms.rs tests.
  - Optional (about 30 min, rides a gate): a fixture test that folds a de-1.x CAN carrying a lot estimate, a framework
    maximum and a lot name, and asserts equal Amount/StatedTotal/lot-title facts. Not required.

## 5. Contingencies

- **Switch-off before deploy #2:**
  - commit or stash the banner work;
  - gate 111f1a7 (25 min);
  - wait for the running project (never DELETE, never FORCE_BUSY: P29);
  - commit the twin's tree on top of main, push by explicit ref, `./deploy.sh`;
  - `switchoff.py build --registry … --base-cursor K`, the refold, `verify`.
  After deploy #2 the twin is twin-4 (or twin-5).
- **A gate with GATE-EXIT ≠ 0:** no deploy of that tree (deploy.sh enforces it). Writers continue. Start a from-clean twin
  gate only when a switch-off needs one.
- **546 or 577 overrun past a tick:** the chain or weekly jobs queue behind it. Record it; never cancel. Close the chain or
  weekly segment once its jobs finish.
- **A writer posted without `segment.py open`** (as 576 U3 was): close it as a gap before the next writer posts, so that no
  two writers ever share one gap.

## 6. Rule amendments for decision-w-on-drains.md and the kit (one line each)

1. Header and §1 (P34, A01): checks inform; no date (10-28, days 8–14, 11-04) gates a writer, deploy or build; review the
   twin's retirement at 11-04.
2. Verdict, I.2, I.7 and I.10, the §5 table's When column, §6's calendar: replace the dates with the dependency order P1 →
   576 U3 → 546 U4 → 577 U3 → fold-changing deploys → in-place writers, each next.
3. B1/P01 + segment.py open (K5): the next writer opens once the previous segment is closed and its state snapshot is saved;
   E1, NEW reads, A and the advance run alongside.
4. B1/P02/D04: "no writer between a chain and its check" becomes "none from the tick to the chain's snapshot or advance";
   strike "U9 check morning" in 576, 577, 578 and 579.
5. B1/D02 + `wkit.DAY7_DATE = None` (K3): the day-7 read is rolling, and 10-28 and 11-04 are tallies.
6. B2/W9/P04 + registry status (K2): pending NEW reads warn; never a HOLD, never a time-driven SWITCH OFF.
7. B2/P05/P26: a STOP registers its ids in R as 'unexplained', advances K and raises the banner; only NOT A MEASUREMENT keeps
   K; a gap is recorded, never frozen.
8. B3/P12/P17: the twin is rebuilt on every deployed base (C7 stays) and gated when the slot is free; uncovered adds 35 min
   to a switch-off and is never a writer HOLD; a red gate holds no writer.
9. B3/P14 + `ops/gate-marker.sh`: the marker keeps a short ledger of green trees, so a main gate stops uncovering the twin
   (one gate to land it).
10. B3/W13'/P16: rely on check.sh's preflight and auto-clean; strike W13'(b)'s "do not gate that day" and the 15 GB lines in
    576 and 578.
11. B4/P18: the probe is the first job after a deploy that changes a W fold path, with no wait for the twin; app-only,
    parse-only and TEXTS-only deploys get a ledger line; "deployed but not probed" is no longer a HOLD.
12. B4/P19: any number of fold-changing commits per probe; bisect offline if the probe moves a head.
13. G/W7/P20/P21: a parse change deploys when gated; its W-neutrality is measured by the re-parse's slice bar,
    code-move profiles first, then the chain gd5.
14. G/C9'/P22/A13/A14: W-input code is deployed, then probed at once; in-place writers pair with a refold in-segment (class I
    with a prediction, else class W); no 11-04.
15. B5/P23 + wkit (K4): the protocol cap is 250/h (readers 225/h, switch-off ≥ 25/h); adopt O1 for corpus spans.
16. B5/P24: no reads during chain or weekly jobs; E1 and NEW reads of closed ranges may run during a later drain job.
17. B6/D7/P09 + start_rule, registry, segment open, make_wet (K1): GO iff the job-and-project upper bound ends before the next
    tick; delete the in-flight D7 SWITCH OFF.
18. B6/P11: the weekly rule is "no writer job running at 02:10Z"; close the weekly segment at queue-idle and resume writers
    then, not after the chain.
19. B6/P03 + segment.py: the chain check runs at queue-idle; the catch-up refusal applies only to jobs over 2 h; a late daily
    is chain-MMDDb.
20. D.1/P08: the segment record and the predraw file are the pre-registration; copy them into 551 afterwards.
21. E/P06/P31: over the cap, or more than 30 NEW: a banner and the salted-30 read in parallel; drains continue; switch off
    only on a failed read.
22. F/P30: a missing pre-step record is a NOTE; SWITCH OFF only on a head read wrong.
23. F's HOLD list shrinks to:
    - another writer's job queued or running;
    - a red gate on the tree to deploy;
    - the chain's tick-to-snapshot window;
    - the weekly jobs running.
24. P07: one writer-lock line, in 551 only, cleared at project end; a lock whose job is not in /admin/jobs is stale and is
    cleared.
25. W14/W11 (B02): deploy when the queue is idle and the deploy plus its probe end before the next tick, else right after that
    chain's snapshot; never a day's slip.
26. R1/D10 stay. Record O3 for 577 now (class N per slice, the NOTE classes plus listed ids), and keep fold-changing deploys
    out of the 577 slice window.
27. W8 and §7: rewrite every "Blocked by: 551 C9' (after 11-04)" line (575, 519 M6, 473, 561, 578/582, 581) to name its real
    dependency.
28. §C "Not adopted" (A16): file Judge 2's I6 counter as an ordinary follow-up that rides the next deploy.
29. O1, O2 and O3: adopted (§3 of the decision).
30. A new rule for 577 U3: never one unbounded reparse; slices sized from a packages-capped first slice, each ending before
    the next tick.
