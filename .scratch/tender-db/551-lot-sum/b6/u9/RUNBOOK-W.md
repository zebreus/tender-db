# Issue 551, path W on: the W-on drains runbook (kit E1–E6)

Prepared 2026-10-23 (kit build, after the decision was recorded; revised the same night after the review panel's
findings). Binding sources: `b6/decision-w-on-drains.md` in full (A–I, the per-drain table, W1–W15, W13'),
`b6/decision-b6.md` (U9, D1–D7, the switch-off, C1–C10) and issue 551's B6 sections. This runbook only orders the
kit's commands. Where it and those records differ, the records win.

```
B6=/home/user/tender-db/.scratch/tender-db/551-lot-sum/b6
SP=/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad
cd $B6          # every command below runs from here; python always as `python3 -I -B`
```

## 0. Conventions (on top of u6/RUNBOOK.md §Conventions, which still hold)

- **The boxes (B5).** Every statement goes through `u6/kitlib.Box`:
  - one lock (`b6/.box.lock`) for every box: one statement in flight across all of them, ≥ 2.5 s apart;
  - the rolling hour counts every kit box together (u6/box, u9/box, u9/box_confirm, u7/switchoff/box);
  - **no lifetime cap** on the W boxes (B5 sets a rate only); past 560 statements (section 5's upper estimate) a
    WARNING line is logged once: record it in 551;
  - segment readers (`u9/box`: gd5, replay, day7) stop at **125/h**; the switch-off's own box (`u7/switchoff/box`)
    at **150/h**, so a switch-off always keeps ≥ 25/h, both inside B5's 150;
  - every statement is plan-probed on `$SP/b6u6kit/plan_w.db` (plan.db plus prod's `changes_entity_cursor`).
- **A non-answer.** The body is saved with its failure class and the flag set; only `SELECT 1` runs until health.
  - `capped` (a 408, interrupted, a timeout, a truncated or unknown body): the IDENTICAL statement is never sent
    again (prod-box-reads: never retry a 408). Re-running the read resumes with the page or chunk HALVED (recorded in
    its manifest) down to 500 rows / ids / cursor values; at that floor it stops: NOT A MEASUREMENT, hand it to the
    owner.
  - `shed` (503/429) or `transport` (empty stdout and an ssh error: it never reached /v1/sql): after health the same
    statement may be re-sent; the re-run resumes there.
  - Health: `python3 -I -B -c "import sys; sys.path.insert(0,'u9'); import wkit as W; print(W.w_box().health())"`
    (the switch-off box: `W.switchoff_box().health()`).
- **E1** (`kitlib.sql_tender_keyset`) is the tender-kind keyset joined with the PK subquery, naming its index
  (`INDEXED BY changes_entity_cursor`). Its plan (pasted for 551, prod-box-reads):
  ```
  SEARCH c USING INDEX changes_entity_cursor (entity_kind=? AND cursor>? AND cursor<=?)
  CORRELATED SCALAR SUBQUERY 1
    SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
  ```
  **E1 is gated in code (W12):** until `u9/replay/replay.json` says PASS (and `u9/confirm_w12a.json`
  index_exists), `--route auto` reads all-kind and an explicit `--route e1` is refused (exit 3); `--route both` is
  allowed. A span of ≤ 50k cursor values always reads all-kind (an explicit e1 there is refused). The switch-off
  degrades instead of refusing.
- **/admin/jobs files.** B1/W10 work from saved bodies:
  `ssh root@zebreus.click '/root/aj.sh "/admin/jobs?limit=100"' > u9/jobs/<label>.json`.
  Posting a job works as in u6/RUNBOOK.md: scp the JSON to `/root/551b6/`, then `/root/aj.sh /admin/jobs @…`.
- **Cursors are never typed.** Read `/health` twice around every saved jobs body, with labels:
  ```
  python3 -I -B u6/health.py --label CB-p0 ; ssh … > u9/jobs/p0_cb.json ; python3 -I -B u6/health.py --label CB-p0b
  ```
  segment.py takes the two labels (`--cb-label/--cb-label2`, `--ca-label/--ca-label2`). Both reads must give the
  same cursor and bracket the body's `measured_at` (±120 s); otherwise it is NOT READY / NOT YET, and nothing is
  recorded (a close refusal leaves the segment open). `--cursor N` is only a cross-check.
- **The jobs mark.** Every jobs window starts at the CA jobs snapshot of the segment that last advanced K (R0: K0's
  /health read, 10-23 18:48:01Z). A job since the mark that is not a `refold-lot-sum` DRY run belongs to no segment:
  open refuses (gap), a chain or weekly records it as a pre-tick foreign job (STOP). A body whose `recent[]` no longer
  reaches back to the mark is NOT A MEASUREMENT (save the window's evidence in 551).
- **Exit codes** (every u9 script; `gd5.py` as `dchecks.py`): `0` PASS; `10` SWITCH OFF (go to §S at once); `2` STOP
  or HOLD (read the line); `3` NOT A MEASUREMENT or a tool fault (treat an exit 1 as 3).
  **Never pipe a checker through `tail`/`grep`.** Redirect it to a file, `echo "EXIT=$?"`, and read that value.
- **Every writer is a segment** (B1). `segment.py open` runs just before the segment's first POST that writes (with
  the start rule's numbers: it refuses a NO-GO and records the segment's latest switch-off start, which is both the
  read deadline of its NEW ids and the end of its gap). `segment.py close` runs after the project. Then `gd5.py read`
  (with a fresh jobs body) and `gd5.py check`, then `registry.py advance`. A STOP, a NOT A MEASUREMENT or a SWITCH OFF
  freezes K: `registry.py status` then says HOLD (or SWITCH OFF), and no writer and no chain check can open. A frozen
  STOP made only of MOVED / LOST / SILENT (or a chain's unannounced SILENT / 3× count) turns into SWITCH OFF in
  `registry.py status` at the end of the gap if it is still unexplained. To clear a STOP, explain or read it within
  the gap and record that in 551. Pass it to `gd5.py check --explained FILE`, then advance the same segment:
  ```
  {"reads": {"<id>": {"verdict": "right|neutral|wrong", "ref": "<551 entry>"}},
   "explanations": {"<id>|foreign_jobs|missing_own|z_rows|z_corrected|count_3x|parity|cap|class_premise|
                    unknown_kind:<kind>|vanished_lots|w_input_prestep": "<text as recorded in 551>"}}
  ```
  `vanished_lots` must name every vanished lot cursor; `class_premise` takes the salted 30 (the segment's
  pre-registered salt) and each drawn NEW still needs its read; an in-place W-input writer among the foreign jobs
  needs `w_input_prestep` AND `--prediction` (C9'), never free text alone.
- **HOLD (do not start)** when `registry.py status` is not 0, the twin is uncovered and no gate is budgeted, a gate
  ended with GATE-EXIT ≠ 0, a chain ran and its U9 check has not closed, it is 10-28 before the day-7 fetch (enforced:
  open refuses), `start_rule` says NO-GO (enforced: open refuses), a W-input pre-step is not recorded, or another
  agent's writer-lock line stands (F, HOLD).
- **Never cancel a running refold or project** (B6/W11). On an overrun, wait: the chain queues behind it, CA is read
  at project end, then the check.

## K. Kit build (2026-10-23; done, offline + 1 statement)

1. Tests (offline, no box):
   - `python3 -I -B u9/test_w.py`: 376 assertions, ALL PASS. Sandbox and outputs go to `$SP/w-kit/`.
   - `python3 -I -B u6/test_kit.py` (full, about 10 min) and `--quick`: ALL PASS with the extended
     kitlib/start_rule/make_wet/switchoff/delta.
2. The sha256s of every new or changed kit file (and the plan.db byte copy `u9/plan/plan.db`, gitignored) are in
   `u9/KIT_W_SHA256SUMS`. Record them in 551 before first use (C, §7), then check them:
   `sha256sum -c u9/KIT_W_SHA256SUMS > $SP/w-kit/sums.out 2>&1; echo "EXIT=$?"`. It must print 0.
3. **W12 (a), done.** `u9/confirm_w12a.py` sent ONE statement (2026-10-23 20:30:52Z, 2.09 s; `u9/box_confirm/`).
   `/health` just before it read cursor 959180852 = K0 and rev 512b297.
   - `/v1/sql` reads only its allow-list, and sqlite_master is not on it (crates/app/src/v1/sql.rs ALLOWED). So the
     "free sqlite_master read" is not possible there.
   - The equivalent sent instead: E1 with `INDEXED BY changes_entity_cursor` over (959179090, 959180852], the last
     chunk of the archived all-kind enumeration.
   - Result: **CONFIRMED** (the index exists, the clause is accepted, the 283 tender rows equal the archived chunk,
     0 derivations differ). Record: `u9/confirm_w12a.json`.
4. **R0/K0** (offline, 0 statements). Record its digests in 551 before 539 step 2:
   ```
   python3 -I -B u9/registry.py init > u9/registry_init.out 2>&1; echo "EXIT=$?"
   ```
   - Expect `R0: 3478 entries {'P': 3467, 'chain-2026-10-22': 1, 'chain-2026-10-23': 2, 'delta-U8': 8}`,
     ids_sha256 `8845f0d03f91defccf46a7275a5ead4d13dc84a44a2501155202562365f3ae20`, K0 959180852, B_R 29 and watch 63
     (Rg 35 + 28 pins outside R); the jobs mark at K0's /health read (10-23 18:48:01Z).
   - entries_sha256 `269abd4ee4c682b89012511264acbe06f1ec87390b8a22d5a7795f5187973ddb` (a content digest of K, B_R,
     entries, watch, lost; the same on every re-init of a pristine R0; unchanged by the post-review kit).
   - `init` refuses unless the inputs are exactly what 551 recorded. `init --force` re-writes R.json ONLY over a
     pristine R0; anything else (advanced, frozen, a segment open, reads marked) needs `--discard-version N` and is
     copied to `registry/history/R_forced_*` first.
5. **W12 (b), the replay: BEFORE the first chain read** (tonight in a quiet gap, or on 10-24 once /admin/jobs shows
   the chain done and the queue empty, BEFORE `gd5 read chain-1024`). It is a read, so the no-writer-between-chain-
   and-check rule holds. 4 statements over 18,216 tender rows (the decision estimated 10–20):
   ```
   python3 -I -B u9/replay.py read  > u9/replay_read.out 2>&1;  echo "EXIT=$?"
   python3 -I -B u9/replay.py check > u9/replay_check.out 2>&1; echo "EXIT=$?"
   ```
   PASS needs E1's tender rows over (959004090, 959180852] to equal the archive's exactly, and E1's lot-sum-awards set
   to equal R0 minus the ids the registry recorded LOST since. **This adjusts the decision calendar's 10-24 order
   (U9 check, then the replay): W12 ("before E1 is used anywhere") overrides it; note it in 551.** Until it PASSES,
   every reader is all-kind automatically (the gate in `wkit.changed_since`); a FAIL keeps it so (E1 stays off; 546
   U4 after 11-04, I.2) -- no flag to remember.

## 1. Before every window (B1–B6, W13')

1. `python3 -I -B u9/registry.py status > $SP/w-kit/status.out 2>&1; echo "EXIT=$?"` must exit 0 (READY). It also
   checks, with 0 statements, that the plan-probe copy, plan.db and plan_w.db exist and every statement shape plans
   clean (a missing probe would refuse EVERY statement, the switch-off's too). Pass `--now` only in tests; it reads
   the clock and applies the recorded deadlines (NEW reads, the end of a frozen segment's gap, D7 in flight).
2. The switch-off twin must be covered (B3). Today the twin is adcb7b0:
   `bash -c 'cd /home/user/tender-db && source ops/gate-marker.sh && marker_covers adcb7b0 && echo COVERS || echo NOT-COVERED; gate_deploy_step adcb7b0'`
   It must print COVERS and `skip`. Then the switch-off gate is `--switchoff-gate covered` (0 min). Otherwise use
   `uncovered` (35 min). After a gate with GATE-EXIT ≠ 0, use `from-clean` and HOLD every segment (W13').
3. No other `ops/check.sh` may run in this checkout while a W-input segment or its check is open. Use no second
   worktree. Before any gate, W13' must hold: (a) at least 4 GB free; (b) no build-input re-hash pending; (c)
   GATE-EXIT read from the log. **When pruning the disk, never remove `$SP/b6u6kit/` (the plan-probe copy, plan.db,
   plan_w.db) or `$SP/v551/plan.db`;** `u9/plan/plan.db` is the byte copy (sha in KIT_W_SHA256SUMS).
4. Run the start rule with the segment's honest numbers (segment.py open re-runs it and refuses a NO-GO):
   ```
   python3 -I -B u7/start_rule.py --runs-left N --minutes-per-run M --read-minutes R --switchoff-gate covered [--switchoff-read-statements S]
   ```
   - It takes the earlier of the next 09:35 Berlin chain (07:35Z through 10-24, **08:35Z from 10-25**) and the next
     Sunday 03:10 Berlin weekly tick (**02:10Z on 10-25 and 11-01**). Inside the weekly window (02:10–04:10Z) it adds
     2 h of queue wait.
   - `M` is job + project + check: the UPPER bound for a corpus job (W11). `R` is the NEW reads (about 45 min for
     ≤ 30 heads; W9).
   - `S` is the switch-off's own box statements (24 s each at 150/h): with K current a handful; with K frozen over a
     corpus segment about 2 × its E1 pages (build and verify re-read (K, N]); gd5 read records its estimate
     (`switchoff_read_statements` in read.json) and gd5 check uses it for its D7 advisory.
5. If `segment.py open` refuses with "CB != K" or "job(s) since the last closed segment", something wrote or ran after
   the last closed segment: another agent's job, a catch-up tick after a restart (B1). Close it as a gap:
   ```
   python3 -I -B u6/health.py --label CA-gap-X ; ssh … > u9/jobs/gap-X.json ; python3 -I -B u6/health.py --label CA-gap-Xb
   python3 -I -B u9/segment.py gap gap-X --ca-label CA-gap-X --ca-label2 CA-gap-Xb --jobs u9/jobs/gap-X.json --reason "<the job ids and kinds>" ; echo "EXIT=$?"   # exit 2: STOP
   ```
   - It records every non-dry job since the mark as foreign (none, but CA > K: 'unattributed') and exits 2 (STOP).
   - Record the jobs in 551. Then `gd5.py read gap-X … --class Z --jobs <fresh body>` (always the FULL read: change
     rows, R ∪ Rg ∪ pins, band; never the shortcut) and `gd5.py check gap-X … --class Z --explained FILE` with
     `foreign_jobs`, then advance.
   - An in-place W-input writer (merge-*, match-org-identifiers, dissolve-guid-twins, apply-rehoming, …) without its
     C9' pre-step is SWITCH OFF: its explanation needs `w_input_prestep` (the prediction's sha + the paired refold) and
     the check needs `--prediction FILE` (class I rules over R ∩ touched).

## 2. The U9 daily check: a class-chain segment (every day from 10-24; W4, B6, H)

**Precondition:** §K.5 PASS (otherwise the chain is read all-kind automatically; the decision budgets +55–70
statements for that).
**When.** After that day's tick, once /admin/jobs shows the day's probe, process, fetch-rates, rederive-eur-recent,
project and reveal-recheck finished and `.current` and `.queued` are empty. Never by the clock. **On a weekday**
(Mon–Fri) issue 222's TED catch-up loop may push a second daily until tick + 3 h 05 when the package was late (always
on a W11 overrun morning): segment.py chain is NOT READY until the loop has ended -- no catch-up probe and measured ≥
tick + 7 min; or a trailing `ted daily (probe)` and its project after the last catch-up probe; or tick + 3 h 07 passed.
If a late daily nevertheless lands after an advanced chain, close it as a second chain segment for the same date
(`segment.py chain chain-MMDDb --date D …`), never as a gap. No writer runs between a chain and its check. `D` is the
date, e.g. 2026-10-24, and SEG is `chain-MMDD`.

```
python3 -I -B u6/health.py --label CA-chain-MMDD ; ssh root@zebreus.click '/root/aj.sh "/admin/jobs?limit=100"' > u9/jobs/chain-MMDD.json ; python3 -I -B u6/health.py --label CA-chain-MMDDb
python3 -I -B u9/segment.py chain chain-MMDD --date D --ca-label CA-chain-MMDD --ca-label2 CA-chain-MMDDb --jobs u9/jobs/chain-MMDD.json > u9/chain-MMDD_seg.out 2>&1; echo "EXIT=$?"
python3 -I -B u9/registry.py status                       # prints K
ssh … > u9/jobs/chain-MMDD_read.json                      # fresh (<= 10 min), idle
python3 -I -B u9/gd5.py read chain-MMDD --k K --cursor-after CA --class chain --jobs u9/jobs/chain-MMDD_read.json > u9/chain-MMDD_read.out 2>&1; echo "EXIT=$?"
python3 -I -B u9/ajobs.py build chain-MMDD --from-seg chain-MMDD
#   POST each u9/ajobs/chain-MMDD/AXX_dry.json (dry: writes nothing, exempt from every jobs window); save each .counts to u9/ajobs/chain-MMDD/AXX_summary.txt
python3 -I -B u9/ajobs.py count chain-MMDD --summary u9/ajobs/chain-MMDD/A01_summary.txt [...] ; echo "EXIT=$?"
python3 -I -B u9/gd5.py check chain-MMDD --k K --cursor-after CA --class chain --a-from u9/ajobs/chain-MMDD/A.json > u9/chain-MMDD_check.out 2>&1; echo "EXIT=$?"
python3 -I -B u9/registry.py advance chain-MMDD
```

- **`segment.py chain` exit 2:** not ready (a kind missing -- `--allow-missing fetch-rates` / `reveal-recheck` only,
  the two the supervisor skips when one is already pending; recorded -- a job running, the reads unequal or not
  bracketing the body, the weekday catch-up loop not ended), or a STOP: a non-chain job after the tick, or ANY
  non-dry job between the mark and the tick (another agent's writer, a restart's catch-up tick, the weekly jobs of an
  unclosed weekly range). The chain's range starts at K, so those writes are inside it: explain them in 551
  (`foreign_jobs`) and read every fire of that check before advancing.
- **A is tied to the segment:** gd5 check refuses an A.json whose ids_sha256 is not this read's changed live ids
  (NOT A MEASUREMENT); rebuild A from this read.
- **Cost.** About 5–6 statements (E1 1–3 pages, NEW PK 1, R ∪ Rg ∪ pins 2, band 1) plus 1–2 dry jobs.
- **Verdicts, W4's own table.**
  - Daily fires are counted and registered as `chain-D` (the day-7 population). This is not a STOP.
  - The 3× rule is on A_d: more than 3 × A_d × 1.0974 % is a STOP and read; still unexplained at the end of the gap
    (status escalates it), a U9 switch-off.
  - A chain-MOVED or chain-LOST R head with a tender row is a counted re-decision, registered and read under H. But a
    B_R head, a path-W pin, a partial sum or 6908248 is stop-and-read at 0 wrong (wrong means switch off); a path-V,
    trio or must-stay head (a value move, or one that starts serving lot-sum-awards) is stop-and-explain (W5).
  - With a tender row only: fresh-FX eur-only moves on non-EUR heads published within 8 days, and seq/published_at
    moves, are NOTEs. A value move (vouched_by included, compared on the parsed detail) with NO tender row is SILENT.
  - SILENT: a derivation or detail move with a seq increase is an announcement gap (STOP, file it; never a
    switch-off in a chain). Any other SILENT move is an unannounced writer (STOP; switch off if unexplained by the
    end of the gap).
  - A B_R head missing, or an extra band row, is a stop-and-read at 0 wrong of any class; wrong means switch off.
  - 6465510 serving lot-sum-awards, or 6908248 serving a double count (read wrong), means switch off.
- **Daily fires on DÖE Tenders before all three 539 step-2 runs (drain-539-a/b/c) advanced** carry the flag "possibly
  539-driven" (A6/H); step 1 (drain-539-1) re-sorts no fold.
- **Record in 551.** The fires (ids, S, currency, n, vouched_by), A_d, the count against 3×, the band, the new K and
  the registry's entries_sha256.
- **Days 1–2, measured after the fact (H), once, before any drain writes.** `python3 -I -B u9/ajobs.py build days12
  --archive`, POST the 2 dry jobs, then `ajobs.py count days12 --summary …`: A_1-2 over the chains' 9,975 changed live
  Tenders (cursor > 959026689). `--after-cursor 959004090` builds the decision's literal 13,453 (3 jobs). Record "3
  fires vs 3 × A_1-2 × 1.0974 %".
- **Fallback for 10-24 only.** If the kit were not ready, the old route (`u7/switchoff.py build --flip-cursor
  959004090`, ≈55–70 statements) is the check. It is ready, so do not use it.

## 3. P0: the W-on probe of 546 U3's deploy (512b297), class D (B4)

**Preconditions.** §1 and §K.3/§K.5 PASS; the ledger entry for 512b297 in 551; that morning's U9 check advanced;
pre-registered in 551: class D; 0 W-column moves; NOTEs for seq/published_at and for place/country on R's legacy
r209 heads; NEW expected 0; the bar; the writer-lock line (a probe has no salt: 551 defines none).

```
python3 -I -B u9/segment.py probe-jobs p0                       # R ∪ Rg ∪ pins (3,541 at R0), one run
#   POST u9/segments/p0/jobs/p0_dry.json (a refold-lot-sum DRY run: exempt from the jobs window); save its .counts to u9/segments/p0/jobs/p0_dry_summary.txt
python3 -I -B u7/make_wet.py p0 --summary-file u9/segments/p0/jobs/p0_dry_summary.txt --jobs-dir u9/segments/p0/jobs --runs-left 1 --minutes-per-run 45 --switchoff-gate covered > u9/p0_make_wet.out 2>&1; echo "EXIT=$?"
python3 -I -B u6/health.py --label CB-p0 ; ssh … > u9/jobs/p0_cb.json ; python3 -I -B u6/health.py --label CB-p0b
python3 -I -B u9/segment.py open p0 --class D --tag probe-512b297 --cb-label CB-p0 --cb-label2 CB-p0b --jobs u9/jobs/p0_cb.json --lock "Writer: 551 p0 until <ISO>" --runs-left 1 --minutes-per-run 45 --switchoff-gate covered --expect-new 0 --expect-moves "0 W-column moves" --bar "B4: 0 moves in derivation, cents, currency, eur, detail.lots, vouched_by membership" ; echo "EXIT=$?"
#   paste the writer-lock line in 551; POST u9/segments/p0/jobs/p0_wet.json; wait for its project
python3 -I -B u6/health.py --label CA-p0 ; ssh … > u9/jobs/p0_ca.json ; python3 -I -B u6/health.py --label CA-p0b
python3 -I -B u9/segment.py close p0 --ca-label CA-p0 --ca-label2 CA-p0b --jobs u9/jobs/p0_ca.json --own <wet id>,<project id> ; echo "EXIT=$?"
ssh … > u9/jobs/p0_read.json
python3 -I -B u9/gd5.py read  p0 --k K --cursor-after CA --class D --route both --jobs u9/jobs/p0_read.json > u9/p0_read.out 2>&1;  echo "EXIT=$?"
python3 -I -B u9/gd5.py check p0 --k K --cursor-after CA --class D > u9/p0_check.out 2>&1; echo "EXIT=$?"
python3 -I -B u9/registry.py advance p0                          # then clear the writer lock
```

- make_wet in W mode needs `--runs-left` (a run under u9/ without it is a STOP; U7's stages.json never judges it).
- **`--route both`** runs E1 and the all-kind reader side by side; their parity on the tender rows is part of the
  check (W12). corrected is parsed from the own jobs' saved counts; a D/N/W segment whose own jobs report corrected > 0
  over a range with no tender row is NOT A MEASUREMENT (CA read too early).
- **Bar.** 0 W-column moves: a W-column move is a STOP (SWITCH OFF unexplained at the end of the gap). A path-W pin
  off or a partial sum off its S or vouch is SWITCH OFF at once, tender row or not. 6908248 moving is a stop-and-read
  (read wrong: serving a double count, SWITCH OFF). 6465510 serving lot-sum-awards is SWITCH OFF. An eur-only move on
  a re-dated head WITH its tender row is a NOTE; without one it is SILENT.
- **P1** (after the batched deploy, once its new twin is gated last) is the same with `p1` and `probe-<rev>`.

## 4. 539 step 1: repair-notice-instants, class Z (no change rows)

**When.** After P0 is advanced, in the 10-24 gap, never between a tick and its U9 check. **Open the segment BEFORE the
dry run:** the dry run writes its mechanical rows as walked (unstamped / shifted instants), so it is the segment's
own job, not exempt.

1. Start rule (`u7/start_rule.py --runs-left 1 --minutes-per-run 60 --switchoff-gate covered`), then read CB twice
   around `u9/jobs/z539-1_cb.json` and open:
   `python3 -I -B u9/segment.py open z539-1 --class Z --tag drain-539-1 --cb-label CB-z539-1 --cb-label2 CB-z539-1b --jobs u9/jobs/z539-1_cb.json --lock "Writer: 551 z539-1 until <ISO>" --runs-left 1 --minutes-per-run 60 --switchoff-gate covered`
   (the salt `551-b6-drain-539-` is derived from the tag).
2. POST the dry run `{"kind":"repair-notice-instants"}`. Review its by-profile counts against 539's ≈16,344 plus the
   inversions ingested since 10-19, and record them in 539 and 551.
3. POST the wet run `{"kind":"repair-notice-instants","dry_run":false}` (expect_rows from the stored reviewed plan;
   it aborts if the corpus has moved).
4. Read CA twice around `u9/jobs/z539-1_ca.json`, then
   `python3 -I -B u9/segment.py close z539-1 --ca-label CA-z539-1 --ca-label2 CA-z539-1b --jobs u9/jobs/z539-1_ca.json --own <dry id>,<wet id> --corrected 0`
   (a typed 0 is accepted only when every own job is repair-notice-instants, which re-projects nothing).
5. Read and check (a fresh jobs body for the read):
   ```
   python3 -I -B u9/gd5.py read  z539-1 --k K --cursor-after CA --class Z --jobs u9/jobs/z539-1_read.json > u9/z539-1_read.out 2>&1;  echo "EXIT=$?"
   python3 -I -B u9/gd5.py check z539-1 --k K --cursor-after CA --class Z > u9/z539-1_check.out 2>&1; echo "EXIT=$?"
   python3 -I -B u9/registry.py advance z539-1
   ```

- **The shortcut (D.2).** K = CA: 0 statements. Otherwise ONE E1 statement must show no tender row (only with E1
  proven; else the full all-kind read), with only its own jobs and corrected = 0. A foreign or missing own job forces
  the full read (an explanation clears W10's STOP, not D.2).
- **Any tender row is a STOP.** Then `gd5.py read … --class Z --full` and `check`: every NEW or MOVED is read and
  explained (`--explained`), and only then advance.

## 5. 539 step 2: the R3 refold, class W, three runs by profile (C9', E)

**When.** 10-24, after the U9 check, P0 and step 1.

**Recorded in 551 before run a.** The C9' discharge (A6 + P0: R is at 539's fixed point, prediction 0 R moves;
seq/published_at moves are NOTEs; eur-only re-date moves WITH their tender rows are explained, 899272 and 5842187
under the 2 % rule); the cap formula max(2·A·1.0974 %, A·1.0974 % + 10); the salt `551-b6-drain-539-` (derived from
the tag; segment.py refuses any other, delta.py's predraw too); the bar text (U5's sequential bar; band rows 0 wrong
of any class); the writer-lock line.

**The runs.** Each run is its own segment, with a STOP point between runs and start_rule GO for each (open refuses a
NO-GO). All three close before the 10-25 02:10Z weekly tick; **otherwise run c moves to 10-25** (after the weekly
segment and that morning's U9 check): a run that has not started is never a switch-off (D7 "do not start it"). Never
run between the 10-28 tick and the day-7 fetch.

| run | segment | tag | profiles (as 539 unit 3 records them; confirm against its sizing) |
| --- | --- | --- | --- |
| a | w539-a | drain-539-a | `["eforms:eforms-sdk-0.1"]` |
| b | w539-b | drain-539-b | `["eforms:eforms-de-1.0","eforms:eforms-de-1.1","eforms:eforms-de-1.2"]` |
| c | w539-c | drain-539-c | `["eforms:eforms-de-2.0","eforms:eforms-de-2.1"]` |

Per run R (with `X` = a, b or c):
```
python3 -I -B u6/health.py --label CB-w539-X ; ssh … > u9/jobs/w539-X_cb.json ; python3 -I -B u6/health.py --label CB-w539-Xb
python3 -I -B u9/segment.py open w539-X --class W --tag drain-539-X --cb-label CB-w539-X --cb-label2 CB-w539-Xb --jobs u9/jobs/w539-X_cb.json --lock "Writer: 551 w539-X until <ISO>" --runs-left <runs left incl. R> --minutes-per-run 60 --read-minutes 45 --switchoff-gate covered --expect-new 0 --expect-moves "0 R moves (C9')" --bar "U5 sequential; cap max(2Ar, Ar+10)" ; echo "EXIT=$?"
#   POST {"kind":"refold","profiles":[...],"expect":<reviewed notice count>} (queues refold + project); while it runs:
python3 -I -B u9/registry.py status [--remaining-minutes <project + check + reads still to go, on an overrun>] ; echo "EXIT=$?"   # 10 = D7 in flight
#   wait for the project
python3 -I -B u6/health.py --label CA-w539-X ; ssh … > u9/jobs/w539-X_ca.json ; python3 -I -B u6/health.py --label CA-w539-Xb
python3 -I -B u9/segment.py close w539-X --ca-label CA-w539-X --ca-label2 CA-w539-Xb --jobs u9/jobs/w539-X_ca.json --own <refold id>,<project id>
python3 -I -B u9/gd5.py read w539-X --k K --cursor-after CA --class W --jobs u9/jobs/w539-X_read.json > u9/w539-X_read.out 2>&1; echo "EXIT=$?"
python3 -I -B u9/ajobs.py build w539-X --from-seg w539-X          # POST the dry A jobs, save summaries (0 jobs: count takes none, A = 0)
python3 -I -B u9/ajobs.py count w539-X --summary … ; echo "EXIT=$?"
python3 -I -B u9/gd5.py check w539-X --k K --cursor-after CA --class W --a-from u9/ajobs/w539-X/A.json --runs-left <runs left after R, 0 for c> --minutes-per-run 60 --read-minutes 45 --switchoff-gate covered > u9/w539-X_check.out 2>&1; echo "EXIT=$?"
python3 -I -B u9/registry.py advance w539-X
```

**NEW reads (E, W9).** The NEW heads enter R as `drain-539-X`, read pending, with the segment's recorded latest
switch-off start as their deadline; `registry.py status` HOLDs the next writer until they pass and says SWITCH OFF
once that deadline passes with reads unfinished (W9; an earlier `--switchoff-by` can only move it earlier):
```
python3 -I -B u8/delta.py predraw --population u9/segments/w539-X/new_population.json --salt 551-b6-drain-539- --out u9/segments/w539-X/reads ; echo "EXIT=$?"
#   record the predraw (population, sha256, salt, bar) in 551, THEN:
python3 -I -B u8/delta.py draw    --population u9/segments/w539-X/new_population.json --salt 551-b6-drain-539- --out u9/segments/w539-X/reads ; echo "EXIT=$?"
python3 -I -B fetch_b6.py --ids u9/segments/w539-X/reads/delta_fetch_ids.json > u9/w539-X_fetch.out 2>&1; echo "EXIT=$?"
#   reader + adversarial skeptic per head, a third on disagreement (wf_dc85fa7e-c00's rubric); record in 551
python3 -I -B u9/registry.py reads --tag drain-539-X --verdict PASS|FAIL --ref "<551 entry>" ; echo "EXIT=$?"
```

- **The draw.** All heads if there are ≤ 30, otherwise the first 30 by salt. Band rows always read 0 wrong of any class.
- **The cap.** Over the cap is a STOP before the next run (explainable with `cap` after the NEW are read; every NEW
  stays read-pending).
- **MOVED, LOST or SILENT** is a STOP: read all of them and port each on refetched facts (`fetch_b6 --refresh`, port
  23cbd850, rates refreshed after 10-27). Wrong, or unexplained at the end of the gap (the segment's recorded switch-off
  instant), means switch off.
- **D7 after the check is an ADVISORY for the runs not yet started:** gd5 check prints "HOLD: do not start the next
  run" when start_rule says NO-GO for them; the verdict stays the run's own (PASS still advances K) and nothing is
  frozen. Run c then moves to the next gap (10-25). **D7 in flight** is `registry.py status`: an open D/W/I run that
  cannot finish (its planned minutes from open, or now + `--remaining-minutes`, plus its reads) before its latest
  switch-off start exits 10: start §S at that instant; never cancel the project, the switch-off queues behind it.
- **A run that wrote nothing** (the POST refused, start_rule flipped after open): `segment.py abort w539-X --cb-label …
  --cb-label2 … --jobs <fresh body> --reason "<why>"` (only with /health == CB == K and no job since CB; any job that
  ran, even a failed one, takes close + gd5). A job that ran but changed no live Tender: close, read, `ajobs build`
  writes 0 jobs, `ajobs count` with no `--summary` writes A = 0 (cap 10), check, advance. An open segment HOLDs the
  next `segment.py chain`: clear it before the next 09:35 Berlin tick.
- **Cost.** About 13–21 statements and 4–12 dry jobs.

## 6. The weekly tick: class weekly (Sundays 10-25 and 11-01, 03:10 Berlin = 02:10Z; W3)

Close it after its jobs finish (about 04:10Z) and **before** the day's 08:35Z chain:

```
python3 -I -B u6/health.py --label CA-wk-MMDD ; ssh … > u9/jobs/wk-MMDD.json ; python3 -I -B u6/health.py --label CA-wk-MMDDb
python3 -I -B u9/segment.py weekly wk-MMDD --date 2026-10-25 --ca-label CA-wk-MMDD --ca-label2 CA-wk-MMDDb --jobs u9/jobs/wk-MMDD.json ; echo "EXIT=$?"
python3 -I -B u9/gd5.py read  wk-MMDD --k K --cursor-after CA --class weekly --jobs u9/jobs/wk-MMDD_read.json ; echo "EXIT=$?"     # 0 or 1 E1 statement
python3 -I -B u9/gd5.py check wk-MMDD --k K --cursor-after CA --class weekly ; echo "EXIT=$?"
python3 -I -B u9/registry.py advance wk-MMDD
```

- Ready only when the jobs body was measured after the tick and at least one weekly job started after it; a non-dry
  job between the mark and the tick is foreign (STOP).
- Any tender row in the weekly range is a STOP.
- **If the weekly segment could not be closed before the chain**, the chain's check STOPs on the weekly jobs as
  pre-tick foreign jobs (W3: never silently absorbed as daily data). Explain them in 551 (`foreign_jobs`), read every
  fire of that check before advancing, and record that the weekly range was not closed on its own.

## 7. Day 7 (10-28): the read set builder (C8, H, W4)

**Order.** Tick 08:35Z, then §2 for chain-1028 (advanced), then the builder, then the predraws, the draws and the
fetch, then the marker. **No writer opens on the Berlin date 10-28 until the marker exists** (segment.py open refuses).

```
python3 -I -B u9/day7.py build > u9/day7_build.out 2>&1; echo "EXIT=$?"
python3 -I -B u8/delta.py predraw --population u9/day7/population_c8.json    --salt 551-b6-daily-       --out u9/day7/c8
python3 -I -B u8/delta.py predraw --population u9/day7/population_moved.json --salt 551-b6-daily-moved- --out u9/day7/moved
#   record both predraws in 551 (population, sha256, salt, bar; C8 unchanged; the chain-MOVED salt named here), THEN:
python3 -I -B u8/delta.py draw    --population u9/day7/population_c8.json    --salt 551-b6-daily-       --out u9/day7/c8
python3 -I -B u8/delta.py draw    --population u9/day7/population_moved.json --salt 551-b6-daily-moved- --out u9/day7/moved
python3 -I -B fetch_b6.py --ids u9/day7/c8/delta_fetch_ids.json    > u9/day7_c8_fetch.out 2>&1;    echo "EXIT=$?"
python3 -I -B fetch_b6.py --ids u9/day7/moved/delta_fetch_ids.json > u9/day7_moved_fetch.out 2>&1; echo "EXIT=$?"
python3 -I -B u9/day7.py fetched --ref "<551 entry>" ; echo "EXIT=$?"     # the marker: every drawn id fetched
```

- **The C8 population** is the registry's `chain-*` entries: the daily folds' first lot-sum-awards writes, not P. It
  holds no drain, probe or delta-U8 heads. **The chain-MOVED population** is a separate population under the same bar.
  chain-LOST heads are listed in `u9/day7/day7.json`, with `step2_done_segment` (539 step 2's completion, or a note).
- **The bar.** All heads at 0 wrong if there are fewer than 30, otherwise U5's sequential bar. A failed day-7 read
  means switch off (U9).

## S. Switch-off (E4; F "SWITCH OFF means")

0. Health both boxes: `W.switchoff_box().health()` and `W.w_box().health()` (§0). If a box refuses with PlanRefused or
   FileNotFoundError naming a missing plan-probe or plan db (not a SCAN): restore `$SP/b6u6kit/plan.db` from
   `u9/plan/plan.db` (sha in KIT_W_SHA256SUMS), point `KIT_PLAN_PROBE` at a copy of the probe whose sha256 you have
   checked against `50a8094ed7de7ed93ba1c5ba76dd4787b760859c3b50968c96921409d9b8b712` (KIT_PLAN_PROBE skips the kit's
   own sha check) if `$SP/b6u6kit/plan-probe` is gone, and re-run. No backup of the probe exists outside the
   session scratchpad and target/ (the kit may write only under b6/, where a 71 MB binary would be git-tracked):
   keeping one outside /tmp is an owner step. Never cargo build mid-crisis, never `probe=False`.
1. Deploy the covered twin: a revert commit whose tree equals the twin outside `.scratch`, on top of main. Push by
   explicit ref, then `./deploy.sh`; `gate_deploy_step` must say `skip`. Today the twin is adcb7b0.
   - **deploy.sh refuses while /admin/jobs has a running or queued job, or cannot measure the queue.** Save
     `u9/jobs/off-pre.json` first and act on it: your own A dry jobs, a dry run or the weekly jobs -- wait (the weekly
     window's ~2 h is already in switchoff_by); a refold, refold-lot-sum, refold-sections or project (yours, the
     chain's or a corpus job) -- **never `DELETE /admin/jobs/<id>`** (the cancel that leaves epoch-stale Tenders) and
     **never `FORCE_BUSY=1`** (it re-runs the job from the top under the twin and breaks the segment's own-jobs
     accounting): wait for the project (FIFO), read CA first, then deploy, with step 2's (K, N] covering that range;
     another agent's job -- wait and record it as a gap. "could not measure the queue": read it with `ssh
     root@zebreus.click tender-admin queue` or a saved body, never override blind.
   - switchoff_by covers the gate, deploy 10 and drain-back/verify 20 (plus the weekly wait and the switch-off's own
     box reads when given); it covers no in-flight run. A job still running at switchoff_by has overrun its upper
     bound: record it in 551 as a D7 overrun; the switch-off may then finish after the tick.
2. Read N ONCE with `u6/health.py --label off-N` (after the twin deploy), then build the list:
   ```
   python3 -I -B u7/switchoff.py build --registry u9/registry/R.json --base-cursor K --cursor-now N > u7/switchoff_build.out 2>&1; echo "EXIT=$?"
   ```
   The candidates are R ∪ Rg ∪ pins ∪ the live Tenders with a tender row in (K, N]. Reads go through the switch-off's
   own box (u7/switchoff/box, 150/h over every kit box; no lifetime cap), E1 above 50k cursor values once proven (an
   unproven or small-span e1 degrades to all-kind). **On exit 3: health, then re-run with the SAME `--cursor-now N`**
   -- the fixed read directories resume (a 408'd page halved); a refusal at the hourly cap only waits for the hour.
3. For each `offNN` run: POST the dry job, then
   `u7/make_wet.py offNN --summary-file … --jobs-dir u7/switchoff/jobs` (never held by the start rule), then POST the
   wet job. Once R passes about 4,500 ids, check the D7 budget again.
4. Read N2, then verify:
   ```
   python3 -I -B u7/switchoff.py verify --registry u9/registry/R.json --base-cursor K --cursor-now N2 > u7/switchoff_verify.out 2>&1; echo "EXIT=$?"
   ```
   It reads only what build did not: (N, N2] plus PK reads of the list, the candidates and the changed live Tenders of
   (N, N2]. It must PASS: 0 lot-sum-awards ids and 0 band rows. If the band statement does not answer, the API band is
   read instead (no retry).

**Fallback.** If the registry is unreadable, or K is frozen past a STOP whose range was never read, use the C1 route:
`--flip-cursor 959004090` in place of `--registry/--base-cursor` (u6/RUNBOOK.md §S). It also reads through the
switch-off box (never u6/box's remaining 300): build ≈ 51 statements on 10-23 plus ≈ 20 a day (≈ 71 on 10-24),
verify only (N, N2] plus the PK reads. From about 10-28 a C1 build alone passes the box's 150/h and waits on the
rolling hour: count it in start_rule's `--switchoff-read-statements` whenever the fallback is in play.

## Kit files (W-on extension; sha256s in u9/KIT_W_SHA256SUMS)

| file | what |
| --- | --- |
| `u6/kitlib.py` | E1 (`sql_tender_keyset`, `tender_keyset`, `classify_tender_rows`); `plan_w.db`; the Box: one lock for every box, failure classes (capped / shed / transport), the capped list (a 408'd statement never re-sent), the rolling hour over every kit box, no silent state reset, a warning threshold; resumable readers that halve a capped page / chunk (`tender_keyset`, `enumerate_changes`, `pk_read resume=True`); the plan.db byte-copy fallback |
| `u6/test_kit.py` | the U6–U8 kit tests, updated: the C1 switch-off route on its own box, transport vs 408 |
| `u7/start_rule.py` | E6: the 09:35 Europe/Berlin chain and the Sunday 03:10 Berlin weekly tick (zoneinfo, the earlier one binds); the weekly window; `--read-minutes`; `--switchoff-gate …`; `--switchoff-read-statements`; `CLOCK` (tests) |
| `u7/make_wet.py` | W mode (`--runs-left …`); a W run without `--runs-left` is a STOP |
| `u7/switchoff.py` | E4 (`--registry R --base-cursor K`, the C1 route kept); both routes on the switch-off box; fixed resumable directories; verify over (N, N2] |
| `u8/delta.py` | E5 (`--population --salt --out`); the predraw refuses a salt other than the population's recorded one |
| `u9/wkit.py` | records, the W boxes, the E1 gate (`e1_proven`), `changed_since`, the comparators, the dry-job / W-input kind / corrected helpers, `plan_ready` |
| `u9/registry.py` | E3: R0 init (the jobs mark; `--force` only over a pristine R0), advance (the jobs mark, read deadlines, pin roles, predicted vouches), reads, status (W9 deadlines, gap-end escalation, D7 in flight, plan readiness) |
| `u9/segment.py` | B1/W10: open (logged cursors, the jobs mark, the start rule, the salt, the day-7 hold, the weekday catch-up), close (refusals keep it open, parsed corrected, --redo), abort, chain/weekly (readiness, pre-tick jobs, the catch-up loop), gap (foreign jobs, full read), probe job files |
| `u9/gd5.py` | E2: read (a fresh jobs body; the E1 gate; no shortcut for gaps / foreign jobs) / check per class (F; W4 chain table; class I over R heads; the STOP clearing keys; D7 as an advisory; A tied to the segment; gap-end escalation) |
| `u9/ajobs.py` | A / A_d dry jobs and counts (D.3, H; days 1–2 from the archive; the list's ids_sha256 in A.json; A = 0) |
| `u9/day7.py` | the day-7 populations (C8 + chain-MOVED, with their salts), the 539 step-2 completion, the fetched marker |
| `u9/replay.py` | W12 (b) |
| `u9/confirm_w12a.py` | W12 (a), build time (done: 1 statement, CONFIRMED) |
| `u9/test_w.py` | offline tests (376), sandbox + fake box seeded with the real 10-23 archives |
| `u9/plan/plan.db` | the byte copy of `$SP/v551/plan.db` (gitignored *.db) |
| `b6/.gitignore` | ignores `b6/.box.lock` |
