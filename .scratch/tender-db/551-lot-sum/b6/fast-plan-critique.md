# Adversarial critique of fast-plan.md (551, 2026-10-24)

Scope: real hazards only. That means wrong data served undetected, an unrecoverable write, a switch-off that cannot
happen, two writers in one segment or a job collision, a rate-limit breach, or a deploy without a green gate. No
objection here rests on caution, interpretability or "one at a time" alone. Each finding below names its failure, the
evidence (file:line) and the smallest fix that keeps the speed.

Method: I read fast-plan.md, decision-w-on-drains.md, u9/RUNBOOK-W.md, the kit (segment.py, gd5.py, kitlib.py, day7.py),
deploy.sh, ops/check.sh, crates/app/src/supervisor.rs, crates/ingest/src/project.rs, the uncommitted notice.rs, and issues
546, 561, 573, 576, 577, 580, 581, 582 and 583. Writing this file was the only action: no box access, no cargo, no git
writes.

## Summary

| # | Hazard | Severity | Fix | Speed cost |
| --- | --- | --- | --- | --- |
| 1 | 577 slices are sized by time from slice 1's rate. A slice over about 500k changed notices makes its paired `project` re-project the whole corpus (about 5 h, never cancelled), running through 02:10Z or 08:35Z. No deploy or switch-off is possible meanwhile. | High | Size slices by notice count: de-1.x whole, de-2.x whole, sdk-1.x in groups of ≤ 300k (make 1e's per-profile counts mandatory). Carry `after` within a profile. | None; slice 1 gets faster |
| 2 | deploy.sh reads the queue once, BEFORE push and nix build. A writer POSTed in that window (2b→2c, 4b→5, step 8) is killed by the restart and re-run from the top under the new binary. | High | No POST from a deploy's queue probe until /health shows the new rev; announce the deploy on the writer-lock line. | ≤ 10 min |
| 3 | If gate A is red or deploy #2 fails, the plan still opens 546 (4.5–8 h, no deploy possible) while 2a's or 1d's reads may be pending (K2). A wrong verdict then means a wrong head served for hours with no banner and no switch-off. | Medium–High | When the banner code is not live, 546 opens only after the pending NEW/MOVED and day-7 reads have verdicts. Expected 0 NEW, so normally no wait. | Normally 0 |
| 4 | C7 is broken: twin-4 is gated "only if no gate B is coming", so from deploy #2 to twin-5 (about 5–10 h) the live base has no gated twin. | Medium (hard constraint) | Gate twin-4 right after deploy #2, during 546. Gate twin-5 first after deploy #3. Check the twin's +/- lines against 111f1a7's. | One 25-min gate in an idle checkout window |
| 5 | The 577 undo ("deploy the previous gated sha") cannot run: the box refuses any ancestor of the deployed rev, with no override. | Medium | Undo forward: pre-build `git revert bdd055d` on a hold branch now; gate it, deploy it and rebuild the twin only if a slice fails. | 0 now; +25 min only on failure |
| 6 | Rule 7 advances K past an unexplained SILENT, an unannounced writer. E1 sees only tender rows, so that writer's NEW heads outside R ∪ Rg ∪ pins (below the band) are never read. | Medium | Exempt SILENT from rule 7: name the job from /admin/jobs over (K, CA] before that job KIND runs again. Other writers continue. | Minutes, only on a SILENT (predicted never) |
| 7 | K1 drops every margin. segment.py close needs an idle queue, so a job ending under about 10 min before a tick lets the tick's jobs into its segment. The gap route (1b/2c stop rule) skips the start rule entirely. | Medium–Low | K1: upper bound + 15 min ≤ next tick. The gap route applies the same arithmetic by hand. | 15 min of slack per tick |
| 8 | Twin builds and gates move HEAD in the single shared worktree. A commit made meanwhile (.scratch records are allowed during gates) lands on the twin branch, and CLAUDE.md's `git push origin HEAD:main` would put path-W-off on main. | Medium–Low | Build twins with merge-tree + commit-tree (no checkout). During a twin gate, freeze commits too. Push by explicit sha. | 0 |
| 9 | Reads: kit 250/h (K4) plus 50/h of ad-hoc reads is exactly 300/h, and the kit's rolling hour cannot see the ad-hoc reads. | Low | Route ad-hoc reads through a kit Box (shared lock and hour), or keep kit readers at 200/h. | ≤ 11% on kit reads |
| 10 | Weekday deploys (10-26+) "right after the chain snapshot" can restart inside issue 222's catch-up loop, and a late TED package is then missed until the next day. | Low | On weekdays, deploy after segment.py's catchup_state says the loop ended (normally tick + 7 min). | ~7 min |
| 11 | §5's switch-off "commit the twin's tree on top of main" now also reverts every undeployed main commit (577 U2, banner, CLAUDE.md) in main's tree. | Low | Commit the twin's tree as a child of the DEPLOYED rev on a hold branch, push it, deploy it by sha. | 0 |
| 12 | The guard probe queues refold + project. The refold aborts but the project runs, so its two jobs are segment jobs. | Low (procedural) | List the guard probe's refold and project ids in `close --own`. | 0 |

## Findings

### 1. 577 slicing crosses the incremental fold's 500k closure cap (High)

**What fails.**
- The incremental project gives up on scoping, and re-projects the whole corpus, when either closure passes 500,000
  notices:
  - the legacy closure (`LEGACY_CLOSURE_CAP`, crates/ingest/src/project.rs:3178);
  - the Tender-link closure (`LINK_CLOSURE_CAP`, :3346, fallback at :3690–3717).
- Issue 244 measured this as a cliff: 20 packages tipped into "INCREMENTAL → FULL fallback", followed by an 85–91 min
  plan phase and a pass that stalled. The rule it drew: "batch by changed NOTICES, not by packages".
- A whole-corpus project runs about 4.9 h (538's 2564, W11). P10/W11 say it is never cancelled.
- The plan sizes slices 2…n "from slice 1's measured rate, each ending before the next tick". That is a linear
  extrapolation over a step function.
  - sdk-1.0…1.15 holds roughly 2.1–2.5M of 577's 3.3M notices.
  - Any sdk group sized by time to fill the gap before the tick crosses the cap.
- The consequences:
  - a ~5 h project that runs across 02:10Z (P11, which the plan keeps) or 08:35Z (D04);
  - deploy.sh refuses while it runs, so for ~5 h neither a fix nor the switch-off twin can deploy.

**Evidence for what is safe.** 539 run b (refold de-1.0–1.2, segment w539-b, 00:43→01:37Z) and run c (de-2.0–2.1,
01:38→02:34Z) each took about 55 min. A refold unmarks every notice of those profiles, so both profile sets are under the
cap. A full fallback would have taken about 5 h.

**Smallest fix (faster than the plan):**
- **Slice 1 = de-1.0/1.1/1.2 whole, no `packages` cap.**
  - It is proven under the cap.
  - It finishes W7's live test in one slice. A packages-capped slice 1 covers only a prefix of de-1.x, so "0 corrections
    clears W7" would not yet be true after it.
- **Slice 2 = de-2.0/2.1 whole.**
- **sdk-1.x:** group by notice count, ≤ 300k per slice, leaving margin for each Tender's other notices that join the
  closure. Take the counts from 1e's per-profile counts, which become mandatory: 19–40 bounded statements spread over
  546's run.
- **A profile split across slices** takes `packages` plus `after`, where `after` is the previous slice summary's `next_after`.
  - A fresh enqueue without `after` re-walks the prefix from its floor: supervisor.rs:19293–19299 (issue 272; job 358
    redid 303k notices this way).
  - The plan's "the rest of de-1.x" has no `after`.
- **Upper bound per slice:** the re-parse at slice 1's measured rate, plus 539's ~55 min per de-sized group (scaled by
  notice count).
- **Fallback alarm:** if a slice's project log or progress shows "INCREMENTAL → FULL fallback", treat that slice as a
  5 h corpus job (K1 arithmetic) and stop slicing that way.

**Correction to §0.** The unsliced job file is less "unstoppable" than §0 says:
- A reparse honours a stop between notices (supervisor.rs:19238, issue 247) and hands over `{"after": N}`.
- Only its paired `project` must not be cancelled.
- So a slice whose re-parse threatens a tick can be stopped. Its project then folds only what was re-parsed. Its upper
  bound can therefore be counted on the project.
- The conclusion "never post reparse.json as it stands" still holds, now for the 500k reason: 3.3M changed notices would
  re-project the whole corpus.

### 2. Deploy restart collides with a writer POSTed during the build (High)

**What fails.**
- deploy.sh probes the queue at :219, before the push, the box-side `nix build` (:335, "can take a while on a cold
  store") and `systemctl restart` (:362).
- "A deploy RESTARTS the service, and a restart re-runs the job that was running from the top" (deploy.sh:200–205,
  issues 21 and 245).
- The plan overlaps exactly this window three times:
  - 2b deploys "during 2a's reads", and 2c opens 546 "as soon as 2a advances";
  - 4b's deploy #3 overlaps 4a, and slice 1 posts "when 546 advances";
  - step 8 deploys "each as it gates" between writers posted back to back.
- A writer posted in that window starts under the old binary and is re-run under the new one. Its segment straddles two
  binaries.
  - For deploy #4, which is fold-changing (575 U3, 561 U4, 583 U2), the writer's corrections then mix old-code and
    new-code folds.
  - Its class-I prediction no longer describes what ran.
  - The "probe as the first job after the deploy" ordering breaks.
  - The decision itself bans FORCE_BUSY for this reason (§S: it "re-runs the job from the top … and breaks the segment's
    own-jobs accounting").

**Smallest fix:**
- A deploy is a box event. Write "Deploy: <sha> in flight" on the writer-lock line from the queue probe until /health
  shows the new rev, or until deploy.sh has exited non-zero.
- No `segment.py open` and no POST during that time.
- Start a deploy only when no writer is due within ~15 min, or simply deploy first and then open: the queue is idle in 2a
  and 4a anyway.
- Cost: ≤ 10 min.

**Side note.** Do not overlap the restart with kit reads either.
- kitlib.classify_failure (:376) classes an empty stdout with an empty stderr as `capped`. The page is then halved and
  the identical statement is never re-sent.
- So "the box client re-sends any statement lost to the restart" holds only if `/root/sq.sh` writes an error to stderr.
- Harmless, but not as the plan states it.

### 3. No banner and no switch-off for up to 8 h if deploy #2 does not land (Medium–High)

**What fails.**
- Before deploy #2 there is no banner mechanism. notice.rs is uncommitted and only reaches the box with gate A.
- The plan says:
  - "If the gate is red: fix and re-gate; 546 U4 does not wait for it";
  - "If the deploy fails: move deploy #2 to step 4. 546 is not held";
  - 2a's stop rule: "546 U4 still opens".
- Under K2, pending NEW/MOVED reads never block a writer.
- So with gate A red, or deploy.sh failing, and 2a reporting NEW or MOVED (or 1d's day-7 reads still running):
  - 546 starts: 4.5–8 h, and deploy.sh refuses while it runs (P10);
  - a "wrong" verdict that lands during it leaves a wrong head served with no banner, no fix deploy and no switch-off
    until 546 ends.
- The owner's tolerance is explicitly conditional: "prod may carry an issue for a while **if a banner is up**".

**Smallest fix:**
- One conditional rule: if the banner code is not live, a job longer than about 1 h opens only after every pending
  NEW/MOVED and day-7 read has a verdict.
- 2a expects 0 NEW and 0 moves, so this normally costs nothing. With the banner live, nothing changes: writing
  `/data/db/notice.txt` shows within 10 s and needs no deploy (notice.rs header, TTL 10 s).

### 4. C7: the twin for the deployed base is not gated (Medium; a stated hard constraint)

**What fails.**
- 3a gates twin-4 "only if no gate B is coming", and 4c gates twin-5 only after deploy #3. So the live sha-A runs without
  a gated twin from deploy #2 (~14:30Z) to twin-5's gate end: about 19:30–20:30Z typically, about 00:00Z at the upper
  bound.
- C7 requires a re-gate on every newly deployed base.
- In that window, a switch-off is possible whenever the queue is idle:
  - 2a→546 open;
  - 546 aborting on its expect guard;
  - 4a's 40–100 min of reads.
- There it costs +25–35 min of gate, plus up to another gate's length if gate B holds the checkout. A dirty tree from the
  1f builds blocks the checkout entirely.
- "A switch-off cannot deploy during a corpus job anyway" is true only for the job itself, not for the idle windows
  around it.
- The single marker (`target/.tests-green`, currently `gate-v2 ed3e2d5`) is overwritten by every gate. That is why gate B
  uncovers it.

**Smallest fix:**
- Gate twin-4 immediately after deploy #2, during 546's run. Until gate B starts at T546 − 45 min, the checkout has no
  other gate. The 1f builds draft in .scratch for those 25 min.
- After deploy #3, twin-5 is the first gate.
- If P14's marker ledger lands before gate B, gate B no longer uncovers twin-4.
- Exactness check, cheap: the +/- lines of `git diff sha-A twin-4 -- . ':!CHANGELOG.md'` equal those of
  `git diff ed3e2d5 111f1a7 -- . ':!CHANGELOG.md'`. Context may differ where the banner or 577 touched the flip files.

### 5. The 577 undo route is impossible as written (Medium)

**What fails.** Step 5's stop rule says "revert 577 U2 (deploy the previous gated sha) and re-parse the slice".
- The box-side guard refuses any rev that is an ancestor of the deployed one, with no override
  (deploy.sh:310–314, "refusing regression").
- The local guard refuses one behind origin/main (:80–87, FORCE_BEHIND).
- Even forced, it would also roll back the banner and, after deploy #3, 581, 575 U1 and the rest.
- So the step that "can be undone" cannot run when it is needed.

**Smallest fix:**
- Undo forward. Now (0 box, 0 gates), build `git revert bdd055d` on a hold branch.
- If a slice reports a correction outside the NOTE set:
  1. gate the revert (25 min) and deploy it;
  2. rebuild and gate the twin on that base (C7);
  3. re-parse that slice with `after` bounds.
- The re-parse restores the rows from the archive, as the plan says.

### 6. Rule 7 advances K past an unexplained SILENT (Medium)

**What fails.**
- SILENT means a value moved on an R/Rg/pin head with no tender row in the range: a writer that does not announce.
- E1, and therefore NEW detection, enumerates tender change rows only.
- gd5 can find a lot-sum head with no tender row only among R ∪ Rg ∪ pins (gd5.py:383–392: `silent_new` comes from
  `rows_all`). Outside that set, only the band read sees it, and only for rows near or over EUR 100 m.
- Under the old rules a SILENT froze K until the writer was found.
- Rule 7 ("A STOP … advances K … A gap is recorded, not frozen") lets writers continue with that writer unidentified. Any
  further heads it creates outside the watch set and below the band are served and never read: wrong data that would not
  otherwise be caught.

**Smallest fix:**
- Rule 7 stays for NEW, MOVED and LOST, which are enumerated through their tender rows.
- On a SILENT (or `silent_new`), name the job from the saved /admin/jobs body over (K, CA] before any job of that KIND
  runs again. Unrelated writers continue.
- Cost: minutes, and only when SILENT fires (predicted never).

### 7. K1's zero margin, and the gap route that skips the start rule (Medium–Low)

**What fails.**
- `segment.py close` refuses unless /admin/jobs is idle (segment.py:396), and it reads CA twice around a jobs body.
- K1 lets a job's upper bound end exactly at the tick. If the project ends at 02:08Z, the weekly jobs (02:10Z) start
  before CA can be read. They then sit inside the writer's segment: two writers in one segment.
  - The weekly segment can no longer be closed on its own.
  - The 08:35Z equivalent pushes daily fires into a drain's segment, where they never become `chain-*` (and so never
    enter the day-7 population).
- The 1b/2c stop rule ("run 546 U4 by the gap route" or "post without opening") skips segment.py open, and with it every
  start-rule check.
  - 546 posted that way after about 18:10Z (8 h upper bound) can still be running at 02:10Z. That breaks P11, which the
    plan itself keeps.

**Smallest fix:**
- K1: GO iff now + upper bound + 15 min ≤ next tick. The 15 min is the time to capture CA, not the old 2 h.
- The gap route applies that same arithmetic by hand before the POST.

### 8. Twin work moves HEAD in the shared worktree (Medium–Low)

**What fails.**
- 3a "cherry-pick 111f1a7 onto sha-A" and every twin gate check out a hold branch in the one worktree (P15).
- The gate rule freezes edits outside .scratch, but .scratch record commits are allowed during a gate. Any agent
  committing a 551 record then commits onto the twin branch.
- CLAUDE.md's prescribed `git push origin HEAD:main HEAD:<handover>` would then publish the path-W-off commit to main.
  - The next `./deploy.sh` from main is covered: the twin's tree has a green marker.
  - It ships W off. Daily folds then re-decide touched heads off path W.
  - The chain table counts those re-decisions as daily LOST ("not a STOP"), so this would not read as an error.

**Smallest fix:**
- Build twin commits without a checkout:
  `git merge-tree --write-tree --merge-base 111f1a7^ sha-A 111f1a7`, then `git commit-tree`, then `git branch`.
- While a twin is checked out for its gate, freeze all commits (not only edits) and announce it in 551.
- Push by explicit sha after checking its ancestry: `git push origin <sha>:main`.

### 9. The /v1/sql 300/h has no single counter (Low)

The server allows 960/h per token (sql.rs:129–130), so nothing sheds; the risk is to the owner's 300/h ceiling.
- K4 sets the kit at 250/h (readers 225, switch-off ≥ 25).
- 1f allows ≤ 50/h of ad-hoc reads (582 U1, 527 U3, 565, 473 U1).
- The kit's rolling hour counts kit boxes only (kitlib BOX_LOCK and state), so any overshoot by the ad-hoc readers passes
  300 unseen.

Fix: send the ad-hoc reads through a kit `Box` (it shares the lock and the hour), or hold kit readers at 200/h whenever
ad-hoc reads run.

### 10. Weekday deploys restart issue 222's catch-up loop (Low)

The loop lives only in the scheduler task (supervisor.rs:19582–19650). On startup, `catch_up_missed_tick` stays quiet when
the tick's probe already ran (:19549–19580), so a late TED package is missed until the next day's walk-forward.
- Rule 25's "right after that chain's snapshot" can fall inside tick → tick + 3 h on 10-26 and later.

Fix: on weekdays, deploy only after `catchup_state` reports the loop ended (segment.py:225). On a normal morning that is
tick + 7 min.

### 11. The switch-off contingency puts the twin's tree on main (Low)

§5 says "Commit the twin's tree on top of main". main is now ahead of the deployed rev (bdd055d, the banner, CLAUDE.md,
then gate B's commits), so that commit also reverts all of them in main's tree.

Fix: commit the twin's tree as a child of the deployed rev on a hold branch, push that branch, and run `./deploy.sh <sha>`.
- deploy.sh accepts any published branch (:46–77).
- The box's contains-deployed check passes.
- marker_covers matches by tree.

### 12. The guard probe's paired project is a segment job (Low, procedural)

The `refold` arm always queues refold + `project` (supervisor.rs:5386–5396). `expect: 1` makes the refold abort before
writing (:10303–10322), but the project still runs. List both ids, together with the wet refold's and its project's, in
`segment.py close --own`. Otherwise close records foreign jobs (a STOP to explain).

## Checked and cleared (not hazards)

- **Disk / auto-clean (rule 10).**
  - check.sh cleans first on a re-hash, then its preflight needs 13 GiB with no reusable family (check.sh:101–125,
    ops/gate-disk.sh).
  - After a clean that is about 20.5 GB free, so it either fits or refuses before cargo: never a half-run gate.
  - Striking W13'(b) is safe.
- **The banner needs no deploy once live.** notice.rs reads `/data/db/notice.txt`, cached 10 s. The kit's health.py
  parses /health with `json.get`, so the new `notice` field is harmless.
- **No hold for 581.** Refolds that move no grouping input retire 0 ids, structurally (580: 0 over 2.33M re-folded). Daily
  chains retire about 12 a day either way. Shipping 581 in deploy #3 without holding 546 or 577 loses at most those.
- **561 U4's epoch 5 is a lazy fill** (561 §A): quiet per-version UPDATEs on touched Tenders, 0 change rows, no corpus
  job. Keeping it after 577 U3 is enough.
- **546 U4 is a whole-corpus re-projection.**
  - 4.5M legacy notices exceed the closure cap, so issue 305's pre-check takes the full path (project.rs:3550).
  - That is consistent with its 4.5–8 h budget.
  - Under sha-A it also acts as a corpus-scale probe of deploy #2's fold, read by 546's own gd5. So "no probe for deploy
    #2" is sound, and 577 U2's fold-time half is text facts only (the project.rs TEXTS entries plus the BT-749 section
    filter in bdd055d).
- **Two writers at once** cannot happen: the job queue serializes. The real risks are writers sharing a segment (findings
  2 and 7).
- **The early day-7 read.** day7.py takes `--allow-not-chain` (day7.py:58, 136). The population is tag-based, so reading
  the 3 heads now matches the 10-28 verdict under "read all, 0 wrong".
- **adcb7b0 is unusable**, confirmed: the box guard refuses it as divergent, since it does not contain a63cf23
  (deploy.sh:316–323). B02's correction stands.
- **Restart catch-up ticks.** Deploys on 10-24 after 07:35Z, and on 10-25 after the 08:35Z chain, trigger no catch-up
  (tick already served). The weekly tick has no startup catch-up at all.
- **K5, if built.** It loses no currently served head, provided the R ∪ Rg ∪ pins and band snapshot is taken at CA; the
  E1 rows are immutable history.
  - It does need segment.py's `CB == K` open check changed (segment.py:318).
  - A switch-off taken while a range is unread re-reads (K, N] itself. At 250/h that is 36–90 min for 546's range.
