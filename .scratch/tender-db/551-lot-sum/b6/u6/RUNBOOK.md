# Issue 551 B6 — U6 freeze, U7 flip + staged drain, U8 verify: runbook

Prepared 2026-10-21 (kit build, before the 07:35Z chain finished; review findings applied the same morning). Binding
sources: issue 551 "B6 — decision" points 1–14 and every B6 section after it; `b6/decision-b6.md` U6–U9, D1–D7,
switch-off and critic amendments C1–C10. This runbook only orders the kit's commands; where it and those records differ,
the records win.

```
B6=/home/user/tender-db/.scratch/tender-db/551-lot-sum/b6
SP=/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad
cd $B6          # every command below runs from here; python always as `python3 -I -B`
```

Conventions:

- **Box reads** go only through the kit's `Box` (`u6/kitlib.py`). Each statement is plan-probed locally first: turso's
  plan-probe runs on a copy of `$SP/v551/plan.db`, and any SCAN of a table is refused before sending. The probe binary
  itself runs from a kit copy (`$SP/b6u6kit/plan-probe`, sha256 beside it), made once from `target/debug/plan-probe`, so a
  gate's auto `cargo clean` cannot take it away mid-gap. Then the statement goes as one bare SELECT via
  `ssh root@zebreus.click /root/sq.sh`, at most one every 2.5 s, with a budget of 300 shared by U6–U8 and at most 280 in
  any hour. A body without `rows` and `columns` (a 408, a shed, an error) is saved, sets a flag, and is **never
  retried**. After that, run `python3 -I -B u6/box_health.py` (`SELECT 1`). Only then re-run the command, which resumes:
  after a transport failure or a shed the same statement may go out again; after a 408 / timeout ("capped") the
  identical statement is refused for good and the resumable readers re-send that page or chunk HALVED (W-on kit,
  u9/RUNBOOK-W.md §0; a capped statement of a non-resumable script is a refusal: hand it to the owner).
  Raw bodies and the log are in `u6/box/`.
- **Public API**: only through fetch_b6's GET client on the shared stamp (at most one GET per 1.5 s across fetch_b6,
  confirm_draw and the kit). That covers `fetch_b6.py`, the API band reads and **every `/health` read**:
  `python3 -I -B u6/health.py --label <C0|C1|CB-<run>|CA-<run>|…>` prints rev, the change cursor and the last job (kind,
  outcome, started/finished) and appends the line to `u6/health.log`. No raw curl.
- **The band** (D4, V4, the switch-off verify) is the plan-probed `/v1/sql` range seek. If that statement does not
  answer (408 or any error body) it is **not retried**: the reader goes on to the public API band (B5's route:
  `/v1/tenders?min_value=10000000000&limit=1000` paged by next_cursor, ~40 pages at 1.5 s, keeping
  `value_derivation.method == lot-sum-awards`), records `route: api`, and the box flag stays set until box_health. To
  skip the statement altogether, pass `--band-route api`.
- **Admin jobs** run on the box through `/root/aj.sh`. To POST a job, copy its JSON file to the box first, then post
  it:
  `ssh root@zebreus.click 'mkdir -p /root/551b6'; scp -q <file> root@zebreus.click:/root/551b6/` and
  `ssh root@zebreus.click '/root/aj.sh /admin/jobs @/root/551b6/<file>'`.
  To read a finished job's summary:
  `ssh root@zebreus.click '/root/aj.sh "/admin/jobs?limit=100"' | jq -r '.recent[] | select(.job_id==<ID>) | .counts'`.
  A job that is still running shows under `.current`, and queued jobs under `.queued`.
- **Exit codes** of the checkers (`dchecks.py`, `verify.py`): `0` PASS, `10` SWITCH OFF (go to §S at once), `2` STOP (do
  not go on; see the line for what to explain or read), `3` NOT A MEASUREMENT (a read is incomplete, OR a tool fault:
  every uncaught exception, refused read or message-carrying exit maps to 3 and prints `NOT A MEASUREMENT (tool
  fault)` — never a verdict). The readers (`snapshot.py`, the `read` subcommands, `enum_lchanged.py fetch-*`,
  `switchoff.py`) use `3` for an incomplete read too. Python can exit 1 only through code the kit does not call, so an
  exit `1` is also a tool fault: treat it as 3.
  **Never** pipe a checker through `tail`/`grep`. Redirect to a file, then echo `$?` and read that value.

---

## 0. Preconditions (all must hold; any miss = do not start, wait for the next post-chain gap)

1. **The 10-21 chain has finished.** `python3 -I -B u6/health.py --label chain` must show `last_job_kind=project` with a
   `finished_at` after 07:35Z. In `/admin/jobs?limit=100`, fetch-rates, `rederive-eur-recent` and the trailing
   `project` must all show `outcome: ok`, and `.current` and `.queued` must be empty. Record the job ids, their
   `started_at`/`finished_at` (epoch; convert with `date -u -d @<s> +%FT%TZ`) and the rederive's counts text.
2. **Code drift (C1).** `git -C /home/user/tender-db diff --stat d617a78 fa96ee3 -- crates/` must print nothing.
   Checked at kit build: empty. The only crates/ change in the flip is 84c5118 itself, held as
   `b6/flip-84c5118.patch` on `hold/551-b6-flip-v3`, gate green per 9d1b4ca. `rev=` must still be the deployed d617a78
   (the W=false twin, which is also the switch-off rev).
3. **Zero-drift (C2)**: the 2026-10-21 ~01:5xZ re-run on d617a78 gave 3,502 verified, 0 corrected, cursor 958919213,
   so `--drift 0` below. Since then, only the 10-21 chain may have written; L_changed covers it.
4. `python3 -I -B u6/box_health.py` must print `health OK`. Then run `df -h /`: at least 1 GB must be free (the kit
   writes well under 100 MB). Pin the probe:
   `python3 -I -B -c "import sys; sys.path.insert(0,'u6'); import kitlib as K; print(K.ensure_plan_probe())"` prints
   `$SP/b6u6kit/plan-probe` (the copy and its `plan-probe.sha256` exist from the kit tests; this verifies them).
5. **Start rule (D7).** Do not start unless all of U6, U7 and U8 can finish at least 2 h before the next 07:35Z — with
   the switch-off twin's gate budgeted (item 6). Use `python3 -I -B u7/start_rule.py --runs-left 8` (a pessimistic
   guess: 2 stages plus up to 6 L_changed runs). It must print `GO`. It is re-checked with the true count at U7.1, and
   `make_wet.py` enforces it before every wet JSON (§U7.3).
6. **The switch-off rev is NOT pre-gated (point 11 / C7; checked at kit build).** The gate marker is one file per
   checkout and names the last green gate: today `gate-v2 84c5118…` (the flip). Check, read-only:
   `bash -c 'cd /home/user/tender-db && source ops/gate-marker.sh && marker_covers d617a78 && echo COVERS || echo NOT-COVERED; gate_deploy_step d617a78'`
   At kit build it printed `NOT-COVERED` and `refuse-head` (and `gate_deploy_step 84c5118` printed `skip`). So a
   switch-off redeploy of d617a78 re-gates it: `deploy.sh` refuses (`refuse-head`) until HEAD is d617a78's tree, then
   runs `ops/check.sh` on it (22–35 min) before the VPS build — §S step 1 says how. The start
   rule budgets that gate (35 min; `kitlib.SWITCHOFF_MINUTES`: gate 35 + deploy 10 + drain-back and verify 20) and
   prints the latest instant a switch-off may START. **Record in 551 before U7.2**: the marker state, that the switch-off
   includes the gate, and the budget. Optional, to make a switch-off minutes: after U7.2's flip deploy reads healthy (the
   flip's own deploy needs the marker on 84c5118 until then), pre-gate d617a78 from a FRESH checkout (its own `target/`,
   a gate from clean ~35 min and ~28 GiB: needs `df` ≥ 35 GB free, which this box does not have today) and confirm
   `gate_deploy_step d617a78` prints `skip` there. Never hand-edit a marker.

## U6 — freeze (before the flip deploy; nothing is enqueued in U6)

**U6.0 cursor.** `python3 -I -B u6/health.py --label C0` and record `C0 = cursor`. No job may run from here until the
flip deploy; if one does, **re-read C0 and redo U6.2–U6.5** (the rates refresh, U6.1, does not depend on C0).

**U6.1 rates (A5).** The refresh must run AFTER the chain's fetch-rates, and preferably before about 14:00Z (when the
ECB adds today's fixing):
```
python3 -I -B u6/rates_refresh.py --after-chain-fetch-rates-done > u6/rates_refresh.out 2>&1; echo "EXIT=$?"
```
Expected output: `OK new rates set $SP/m551/rates-2026-10-21T…Z`, three sha256 lines, and `newest fixing 2026-10-20`.
The two `ecu-ert_*` sha256 values must equal the old set's (47e94127…, 48e2c9f1…). The old set `$SP/m551/rates` is
left untouched. `u6/rates.json` names both sets. If the exit is 2 with `DEVIATION: newest fixing 2026-10-21`, the
ECB has already published today's fixing. Either re-run before 14:00Z the next day (missing this gap), or accept it
with `--accept-newer` and record it: it only moves the S_eur of a head published on 10-21. Exit 2 for any other
reason means stop.

**U6.2 L_changed (C1/C3/C9, the 2630-excluded reading).**
```
python3 -I -B u6/enum_lchanged.py fetch-changes --cursor-now C0 > u6/fetch_changes.out 2>&1; echo "EXIT=$?"
python3 -I -B u6/enum_lchanged.py fetch-notices > u6/fetch_notices.out 2>&1; echo "EXIT=$?"
```
Expected output from the first command: `change rows (958787061, C0]: N; complete=True` (exit 0; 3 = incomplete:
box_health, then re-run — it resumes). That is about (C0 − 958787061)/5,000 statements: roughly 30 at the kit-build
cursor plus the 10-21 chain, a few minutes. The range holds every kind the store writes — tender, lot, organization and
the result graph (lot_result, bid, contract, which dominate fold windows), plus `removed` rows from orphan sweeps and
retirements. The second prints `N0=… NMAX=… chunks=k`, where N0 is the first notice ingested at or after
2026-10-18T20:30Z. Then the rederive windows, one per day. `MOVED` is the job's "`N tender(s) stamped epoch-stale`"
figure, and START/END are its `started_at`/`finished_at` as ISO Z. For 10-19 the record says "rederive 0 changed";
take its window from `/admin/jobs?limit=300` if it is still listed. If it has scrolled out, give the 10-19 chain's
rederive window from the 10-19 records.
```
python3 -I -B u6/enum_lchanged.py build --cursor-now C0 \
   --rederive 2026-10-19:<M19>:<START19>:<END19> --rederive 2026-10-20:<M20>:<START20>:<END20> \
   --rederive 2026-10-21:<M21>:<START21>:<END21> > u6/build_lchanged.out 2>&1; echo "EXIT=$?"
```
Expected output:
- `L_changed N (sha256 …), ∩ 3,502: k, outside: m`, then the per-source lines S1–S8 and
  `2630 excluded: 3502 tender + X lot + Y org rows + result rows {…}`;
- `rows by kind/op: {…}` (all six kinds counted), `retired (out of L_changed): r; unmapped lot rows {removed, gone_later,
  bad: 0}`;
- `rederive <day>: moved M confirmed=True (<reading>)` three times — the reading is `[START, END)`, `[START, END]`, or
  "the first k of END's second in cursor order" (the trailing project can share END's second; the rederive commits first);
- two `DEVIATION` lines (to record at U6.6), `NOTE` lines (removed lots, retired Tenders), `ANOMALY` lines (listed and
  included where they name a Tender: L_changed grows, the conservative side; a sweep-only instant names none);
- no `STOP` lines.

Exit 2 = a STOP line. Read it; do not go on. The STOPs are:
- the 2630 rows are not exactly 3,502 tender rows on the 3,502;
- a kind outside the six;
- an added/changed lot row whose lot vanished with **no** removed row in the range (a removed lot, or a lot a later
  row removes, is expected and counted);
- one of the 3,502 was **retired** in the range (it can no longer be refolded: the 3,502, P and stages 1/2 must be
  re-decided and recorded in 551 — the kit assumes all 3,502 exist);
- a rederive count that no reading matches (the seq-less stamps within 2 min of the window are printed);
- a notice parsed but not projected (the chain is not finished);
- the range is not contiguous.

A Tender **retired** in the range (its last tender row is `removed`: merged away, no tenders row) is taken out of
L_changed — it cannot be refolded or PK-read — and listed under `retired` with the sources that named it.

Residuals and deviations recorded in `u6/l_changed.json` (no bounded read can enumerate them):
- 541's 627 stamped-not-corrected plus 67 touched Tenders, and 571's 1 verified Tender: no change row; their carriers
  can only be found by a full `notice_texts`/`notice_ids` walk. They compared identical, and none is in the 3,502.
  This departs from C1/C3's "whole refolded sets": recorded under `deviations` and in 551 at U6.6.
- Notices ingested before the cutoff but first parsed after it (C3's "folded since" read as U2 correction (2)'s
  "first ingested since"): the second `deviations` entry.

If the 541 whole set ever becomes available as a file, pass `--set541 FILE` (the first deviation then drops).

**U6.3 refetch L_changed ∩ 3,502, and run the port of record (v2+both).**
```
python3 -I -B u6/predict.py backup > u6/predict_backup.out 2>&1; echo "EXIT=$?"     # copies the k bodies + sidecars to u6/pre_refresh/
T_REF=$(date -u +%FT%TZ)
python3 -I -B fetch_b6.py --ids u6/refresh_ids.json --refresh > u6/fetch_refresh.out 2>&1; echo "EXIT=$?"
python3 -I -B u6/predict.py run --refreshed-after $T_REF > u6/predict_run.out 2>&1; echo "EXIT=$?"
```
- `backup` prints `k ids backed up` (k = L_changed ∩ 3,502). If k = 0, skip fetch_b6.
- `fetch_b6` must exit 0: k tender GETs plus any notices no layer holds, about 1.5 s each.
- `predict run` checks the port sha256 (23cbd850…) and reads `u6/rates.json`. It runs all 3,502, about 8 minutes;
  in the offline test the port reproduced U2 exactly. Expected output:
  `P_final n (ids_sha256 …); Rg r (equals U2: …); replaced x; same as U2 3502−k; S_eur-only moves y; errors 0`,
  exit 0.
- With k = 0 or no fact change, P stays at **3,467, ids_sha256 a2619fd9e380d3d5…**, and Rg at 35 (R_single ∪ R_excess).
- STOP (exit 2) when:
  - an id outside L_changed decides differently from U2;
  - an S_eur moves on a head dated on or before the OLD set's newest fixing (2026-10-16; read from the old zip, so the
    first date allowed to move is 2026-10-17, recorded as `rates_move_from`): the ECB revised its history;
  - a refetch id was not re-read after T_REF;
  - any port error occurs.

**U6.4 freeze.**
```
python3 -I -B u6/freeze.py > u6/freeze.out 2>&1; echo "EXIT=$?"
```
Expected output: `P 3467 (a2619fd9…), Rg 35 (=U2 True), B_P 29 (=U2 True), near […5842187…], band re-read 0, L N, snapshot ids M`,
exit 0.
`u6/freeze.json` holds:
- P's n, ids_sha256 and P_final's sha256;
- Rg, B_P, the near-100m list, and U5's watch id 899272;
- `pins_expected`: 585917 EUR 25000000 BT-118 on 25416405; 5995030 PLN 154951160 with vouched_by TED-VAL_TOTAL
  19612533 cents 154639000; 8426318 GBP 300000000000 TED-HIGH 20083570; each lot-sum-awards with any of the port's
  vouches accepted (A4). Also the trio at eur 166066168294 with no derivation, the 16 path-V sums (rule 537, and 492
  for 928196), the 9 must-stay ids, and 6465510;
- `L_ids.json` (3,502 ∪ L_changed, runs of 5,000 listed) and `snapshot_ids.json` (L ∪ pins ∪ 7511890 ∪ the 61).

Exit 2 with `STOP` lines: a path-W pin the port does not predict, or 6465510 in P — stop. Exit 2 with only
`READ (B-band …)` lines (**point 8 / B-band**): `band_reread` lists every one of U2's 29 band heads whose S, n or
vouched_by changed or that left P, AND every head of the final B_P that is not among the 29 (a new band head: an
L_changed ∩ 3,502 refetch, or the refreshed rates — 899272 sits at ≈ EUR 98.95 m). Read each before the deploy, under
the M2 protocol, at 0 wrong of any class, record the reads in 551, then go on (freeze.json is written either way; D4
then expects those heads).

**U6.5 PK snapshot (before the flip deploy, so before any wet run).**
```
python3 -I -B u6/snapshot.py --ids u6/snapshot_ids.json --label pre > u6/snapshot_pre.out 2>&1; echo "EXIT=$?"
```
Expected output: `pre: M of M ids read; failures 0; missing 0` and `serving lot-sum-awards: 0`, exit 0. The statements
are IN-lists of at most 2,000 ids on the rowid, about M/2,000 of them. Exit 3 = incomplete (box_health, re-run: an
incomplete read is overwritten). Exit 2 with a non-zero lot-sum-awards count means STOP: the snapshot is not the
W=false state. A complete snapshot is never overwritten.

**U6.6 record in issue 551 before the deploy:** C0; the rates set and its three sha256; L_changed n, sha256 and
per-source counts, the rows by kind/op and the retired list; **the two deviations from `l_changed.json` `deviations`**
— (a) 541's 627 + 67 and 571's 1 verified Tender are not in L_changed although C1/C3 ask for the whole refolded sets,
with the residual counts, the reasoning (verified = served output identical; 541/571 change served rows only; none in
the 3,502; a missed fire only loses a fix) and how D5 treats them (as unchanged: a fire on one is a D5 SWITCH OFF), and
(b) the ingest cross-check reads C3's "folded since" as "first ingested since 2026-10-18 20:30Z" (U2 correction (2)) —
both as deviations from binding text, BEFORE the flip deploy; P n and ids_sha256; B_P and any band re-reads; L sha256;
the snapshot manifest sha256; and §0.6's switch-off gate state.

## U7 — flip and staged drain (one gap; no chain in between)

**U7.1 stages and job JSON.**
```
python3 -I -B u7/build_stages.py > u7/build_stages.out 2>&1; echo "EXIT=$?"
```
It writes `u7/stages.json` and `u7/jobs/<run>_ids.json` / `<run>_dry.json`. The runs are:
- `stage1`: the 61, the pins, 7511890, Rg, and a salted 300 of P, by `sha256('551-b6-stage1-'+id)` ascending over P
  minus all of those (about 430 ids in all);
- `stage2`: the rest of the 3,502;
- `lc01…`: L_changed minus the named ids, in runs of at most 5,000.

Record the stages.json sha256, the salt and the drawn 300 in the issue **before** the deploy. Re-run the start rule
with `--runs-left <len(order)>`; it must print GO.

**U7.2 deploy the flip.** Deploy the pre-gated flip rev 84c5118 (held patch `b6/flip-84c5118.patch`; the marker covers
it) by the repo's deploy procedure. That procedure is not part of this kit; push by explicit ref (CLAUDE.md). Then
`python3 -I -B u6/health.py --label C1`: `rev=` must be the flip rev, and record its cursor as `C1` (the switch-off
route needs it). If C1 ≠ C0:
```
python3 -I -B u6/enum_lchanged.py fetch-changes --from C0 --cursor-now C1 --dir u6/lchanged/changes_postdeploy > u6/postdeploy_fetch.out 2>&1; echo "EXIT=$?"
python3 -I -B u6/enum_lchanged.py classify --dir u6/lchanged/changes_postdeploy > u6/postdeploy_classify.out 2>&1; echo "EXIT=$?"
```
`classify` must exit 0 (`PASS: no tender, lot or result row`; organization rows are listed only). A deploy writes no
Tender rows; exit 2 means STOP: redo U6.2–U6.5 for the range (C0, C1] before any wet run.

**U7.3 each run R, in `stages.json` order (stage1, stage2, lc01, …):**

1. Dry. Copy `u7/jobs/R_dry.json` to the box and POST it. It is a dry run by default
   (`{"kind":"refold-lot-sum","tender_ids":[…]}`). When it finishes, save its `.counts` text to
   `u7/jobs/R_dry_summary.txt`.
2. `python3 -I -B u7/make_wet.py R --summary-file u7/jobs/R_dry_summary.txt > u7/R_make_wet.out 2>&1; echo "EXIT=$?"`
   Expected output: `start rule (D7): … GO …` and `R: dry admitted N of M (K notices); wrote R_wet.json (expect N)`.
   - stage1: N = M − 1, because 7511890 is not admitted; non-3,502 pins that no lot sum can reach may also show up as
     notes.
   - stage2: N = M.
   - Exit 2 means STOP and **no wet JSON is written**. The causes are: an id of the 3,502 was not admitted; a different
     list was posted; the counts do not add up; the run was already checked; **the start rule says NO-GO** for the runs
     still to do (stages.json order minus the runs checked): do not post; if the drain has begun, switch off (§S) so it
     finishes before the chain.
3. `python3 -I -B u6/health.py --label CB-R`: `CB=` the cursor, read immediately before the wet POST.
4. Copy `u7/jobs/R_wet.json` to the box and POST it (`"dry_run":false,"expect":N`); it queues its own `project`. Wait
   for the project to finish. Record the wet job's `re-queued X, stamped S epoch-stale` and the project's
   `V verified, C corrected`.
5. `python3 -I -B u6/health.py --label CA-R`: `CA=` the cursor, read immediately after the project.
6. `python3 -I -B u7/dchecks.py read R --cursor-before CB --cursor-after CA > u7/R_read.out 2>&1; echo "EXIT=$?"`
   This enumerates the change rows in (CB, CA] (all six kinds; Tenders retired there are expected-absent), then reads
   by PK the named ids, the pins and every live Tender with a change row there, then the band (the /v1/sql seek, or the
   API if it does not answer — then box_health before the next box read). Exit 0, or 3 = incomplete.
7. `python3 -I -B u7/dchecks.py check R --stamped S --verified V --corrected C --drift 0 > u7/R_check.out 2>&1; echo "EXIT=$?"`
   The expected result is `R: PASS`, with the rule lines printed (D7's line names the deadline and the latest
   switch-off start) and `u7/results/R.json` written.
   - **stage runs**: C = `stages.json` n_P for R, and S = N = V + C.
   - **lc runs**: C = the new fires (expected about N × 3,502/251,526) plus any noted non-W corrections (for
     example, a DÖE re-date). D6 is the count rule here.
8. The next run starts only on PASS (or on a STOP explained and recorded in 551 within the gap, for the STOPs that
   allow it below). D7 is checked in step 7 and enforced again by make_wet in the next run's step 2.

Verdicts, by rule:

| rule | measured on | SWITCH OFF (§S) | STOP |
|---|---|---|---|
| D1 (stage runs) | snapshot vs post PK reads of the named ids | a named id serving lot-sum-awards outside P; a P fire with S / currency / n different from P's, or with a vouched_by outside P's vouches (membership, A4); an Rg id not identical | a P id that did not fire: explain it within the gap; switch off if it is still unexplained at the gap's end (a named id retired in the range: explain) |
| D2 (stage runs) | snapshot diff, change rows, job counts | named diff ≠ the P fires once D1's non-fires are set aside; a named non-P id with a change row; corrected ≠ \|fired ∩ P\| + the non-fires that moved otherwise + drift (0), unless the surplus equals the touched expansion, which D5/D6 then judge (C4) | a predicted id that did not fire (D1's STOP, reported here too: D2 takes it out of its set and count, so the run reads STOP, not SWITCH OFF); stamped ≠ dry admitted; verified + corrected ≠ stamped; a named pin outside the 3,502 whose only change is seq/published_at (non-W drift, e.g. 539's DÖE re-date; C2 probed the 3,502 only): explain |
| D3 (every run) | pins post PK | any pin off | a pin retired in the range: explain |
| D4 (every run) | band (≥ EUR 100 m: the plan-probed seek, LIMIT 200; or the API route, no limit) vs B_P ∩ drained | a B_P head missing | a missing head within 2 % of EUR 100 m (explain it); an extra row: read it at 0 wrong of any class, and switch off only if it reads wrong. An early B_P fire through the expansion is a note |
| D5 (every run) | PK read of every live Tender with a change row in (CB, CA] | lot-sum-awards outside P ∪ L_changed | a P fire with no tender change row; a kind outside the six; a lot that vanished with no removed row; a named id retired in the range |
| D6 (lc runs; stage expansion) | new fires (named + touched, not in P, not fired before); named Tenders' other moves | — | more than max(2 × proj, proj + 10), with proj = N × 3,502/251,526: stop and read before the next run. A delta band row: read it at 0 wrong of any class. A named Tender whose served VALUE columns moved without a new fire: explain (a seq/published_at-only move is a note) |
| D7 (every run, and make_wet) | start rule for the runs left, with the switch-off gate budgeted | the runs left cannot finish before the latest switch-off start (chain − gate − deploy − drain-back/verify): switch off now so it completes before the chain | the 2 h margin is gone: do not start the next run unless re-estimated to fit; otherwise switch off before the latest switch-off start. make_wet refuses the wet JSON on NO-GO |

## U8 — verify and the delta read

```
python3 -I -B u8/verify.py read  > u8/verify_read.out 2>&1;  echo "EXIT=$?"
python3 -I -B u8/verify.py check > u8/verify_check.out 2>&1; echo "EXIT=$?"
```
The read covers L, the pins, 7511890, every fire the runs saw (Tenders the runs saw retired are expected-absent), and
the band (API route if the statement does not answer). Expected output:
`u8 verify: PASS; delta population n (band rows […])`, with V1 to V5 PASS:
- V1: every P id serves P's S, currency and n with a member vouched_by, and Rg is identical to the snapshot;
- V2: nothing outside P ∪ L_changed serves lot-sum-awards;
- V3: the pins;
- V4: the band equals B_P ∪ the delta band rows. **V4 shows STOP for each delta band row until it is read** (0 wrong of
  any class): with any delta band row, the first check reads STOP by design;
- V5: every run ended PASS (a run that ended STOP shows STOP until its explanation is recorded).

After the reads and explanations are recorded in 551, re-run the check with them:
`python3 -I -B u8/verify.py check --explained u8/explained.json > u8/verify_check2.out 2>&1; echo "EXIT=$?"`, where
`u8/explained.json` is `{"band_reads": {"<id>": {"verdict": "right"|"neutral"|"wrong", "ref": "<551 entry>"}},
"run_explanations": {"<run>": "<the explanation as recorded>"}}`. A band row read right or neutral, and an explained
STOP run, become notes; a band row read wrong is SWITCH OFF (exit 10 → §S).

**U9's count baseline (C8).** The check also prints `U9 count baseline (C8): F new fires / A dry-admitted over the lc
runs = r` (and writes `u9_count_baseline` into `u8/verify_result.json`): the fire rate observed in the L_changed runs.
Record it in 551 as U9's daily-count baseline before the first U9 check (it replaces "1.39 % of admitted").

**Delta read (U8, salt `551-b6-delta-`).** Record the predraw record in the issue first.
```
python3 -I -B u8/delta.py predraw ; echo "EXIT=$?"      # population n, its sha256, the salt, the draw rule and the FULL bar -- record in 551 now
python3 -I -B u8/delta.py draw    ; echo "EXIT=$?"      # u8/delta_read_set.json, u8/delta_fetch_ids.json
python3 -I -B fetch_b6.py --ids u8/delta_fetch_ids.json > u8/fetch_delta.out 2>&1; echo "EXIT=$?"
```
`delta_predraw.json` fixes, before the draw, the bar text below (`bar`) and which population case applies
(`bar_applies`); the issue entry before the draw carries the same text. If the population is 30 or fewer, read all of
them. Otherwise read the first 30 by `sha256('551-b6-delta-'+id)`; the next 30 (or every remaining head, if fewer) are
listed as the continuation. Every delta band row (at least EUR 100 m) is read in full. Each head gets a reader and an
adversarial skeptic, plus a third reader on disagreement (wf_dc85fa7e-c00's rubric). Readers get the served head (S,
currency, n, vouched_by) and the fetched Tender with its notices.

**U5's bar, as it applies here (fixed in the predraw record):**
- Population over 30: 0 wrong of the first 30: PASS. Exactly 1 wrong: read the continuation 30, which must hold 0
  wrong. 2 or more wrong: FAIL, switch off.
- Population 31–59: as above; the continuation is every head left after the first 30.
- Population of 30 or fewer: all are read, and 1 wrong already fails (no further heads exist; C8's "fewer than 30: read
  all at 0 wrong").
- Neutral heads outside the band are allowed and counted.
- A delta band row must read 0 wrong of any class.

Then record everything in issue 551 and in operations.md ("Path W v2+G" and its runbook), and start U9's 14-day
watch.

## S — switch-off (any SWITCH OFF verdict; a failed delta read; D7; an unexplained D1 non-fire at the gap's end; an extra band row that reads wrong)

1. **Redeploy the W=false twin**: the rev deployed before the flip (d617a78; `u6/health.py` `rev=` must show it after).
   It is **not pre-gated** (§0.6), unless a fresh-checkout pre-gate was made and covers it. `gate_deploy_step d617a78`
   says what deploy.sh will do: `refuse-head` → HEAD must first be d617a78's tree (in this shared worktree: only with
   `git status --short` empty outside `.scratch/` and the other agents told, `git checkout --detach d617a78`; or a fresh
   checkout, whose from-clean gate needs ~35 GB free); then `run` → deploy.sh gates it (an incremental gate here, about
   22 min: the flip changes no build input) and deploys; `skip` → deploys at once. Return HEAD afterwards. Never
   `SKIP_TESTS=1`, never a hand-written marker. This gate is in the start rule's budget; start the switch-off no later
   than the `switch-off must START by` time every D7 line prints.
2. Build the id list of every head serving lot-sum-awards (point 11 / C7: drain ids ∪ every id with a change row since
   the flip cursor). `C1` is the cursor recorded at U7.2; read `N` now (`u6/health.py --label off-N`):
   ```
   python3 -I -B u7/switchoff.py build --flip-cursor C1 --cursor-now N > u7/switchoff_build.out 2>&1; echo "EXIT=$?"
   ```
   Both cursors are REQUIRED. It PK-reads the 3,502 ∪ L_changed ∪ every fire seen ∪ the pins, plus every live Tender with
   a change row in (C1, N] — which finds a fire no completed check recorded (the run in flight at a D7 switch-off, a
   check that returned NOT A MEASUREMENT or crashed). It writes `u7/switchoff/jobs/offNN_ids.json` /
   `offNN_dry.json`, in runs of at most 5,000. Exit 3 = incomplete: box_health, re-run.
3. For each off run: POST the dry; then
   `python3 -I -B u7/make_wet.py offNN --summary-file <dry counts> --jobs-dir u7/switchoff/jobs` (never held by the
   start rule); then POST the wet (it queues a project). Expect corrected = the run size, each head returning to its
   max election.
4. Read `N2` after the drain-back's project(s) (`u6/health.py --label off-N2`), then
   `python3 -I -B u7/switchoff.py verify --flip-cursor C1 --cursor-now N2 > u7/switchoff_verify.out 2>&1; echo "EXIT=$?"`
   must print PASS (exit 0): 0 ids serving lot-sum-awards (the list, the candidates and every Tender with a change row
   since C1) and 0 band rows. If the band statement does not answer, the API band is read instead (no retry);
   `--band-route api` forces it. Exit 2 = a head still serves lot-sum-awards: build again and run another off run.

## Kit files

| file | step |
|---|---|
| `u6/kitlib.py` | the shared plumbing: paths, records, constants (cursors, stamps, pins, salts, switch-off budget), the Box, plan_check (from the probe copy), the SQL builders, pk_read (expected-absent ids), enumerate_changes, classify_changes (the six kinds), the band reader (sql → API fallback), run_main (exit-code discipline) |
| `u6/confirm_schema.py` | kit-build only: the 5 bounded confirmation reads (`u6/box_confirm/`) |
| `u6/l_changed_545_whole.json` | 545's 48 Tenders (47 corrected + 8714759), from confirmation read #5 |
| `u6/health.py` | every `/health` read (C0, C1, CB, CA, switch-off cursors), paced on the shared GET stamp |
| `u6/rates_refresh.py` | U6.1 |
| `u6/enum_lchanged.py` | U6.2 (and U7.2's post-deploy delta + `classify`) |
| `u6/predict.py` | U6.3 |
| `u6/freeze.py` | U6.4 |
| `u6/snapshot.py` | U6.5 |
| `u6/box_health.py` | clears the flag after a failed read |
| `u7/build_stages.py`, `u7/make_wet.py`, `u7/start_rule.py` | U7.1, U7.3 steps 1–2 (with the start rule), D7 |
| `u7/dchecks.py` | D1–D7 (read + check) |
| `u7/switchoff.py` | §S |
| `u8/verify.py`, `u8/delta.py` | U8 (and U9's count baseline) |
| `u6/test_kit.py` | offline tests: `python3 -I -B u6/test_kit.py` (full, about 10 min) or `--quick` |
