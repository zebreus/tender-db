# Issue 551: drains while path W v2+G is on (decision, 2026-10-23)

> **OWNER RULING, 2026-10-24 ~11:00Z (Lennart): velocity first. This overrides every hold in this file.**
> Development velocity is the #1 priority. Waiting days, or even hours, is never acceptable, and "there is no reason to
> delay things in general". There are no active users yet, so prod may carry an issue for a while **if a banner is up**.
> Never let a protocol block work.
> - Every calendar hold here is VOID: "after the day-7 read", "days 8–14", "after 11-04", "one fold-changing commit per
>   probe", W14's fit-before-gap, W7 as a deploy blocker, C9' as a writer blocker.
> - Checks (gd5, P1, read-backs, verifies) still run, but alongside the work: they inform, never gate. A failed check
>   means a banner, then a fix, not a freeze.
> - Mechanical constraints that stay: one writer on the box at a time (the job queue serializes); a green gate on the
>   deployed tree (deploy.sh enforces it); no edits outside .scratch while a gate runs; /v1/sql ≤ 300 req/h.


Panel: three judges (risk-first, throughput-first, measurement-first); a synthesis that checked each judge's claims against
the records and code on 2026-10-23 ~20Z (nothing run on the box); an adversarial critic. **Adopted: the synthesis as amended
by every non-optional critic amendment (W1–W15, folded in below), plus one recorder's check (R1, section 4).** The optional
amendments O1–O3 are listed, not adopted. Recorded before any drain, probe or kit run under it.

Scope: every writer while path W is on (`LOT_SUM_AWARDS_PATH = true`, e718684, since 2026-10-21 10:24Z): the pending drains
546 U4, 539, 579, 576 U3, 577 U3 and 578 (plus its 358 follow-up), the code deploys they need, and the U9 daily chain they
must not blur. **The end of U9 (11-04) relaxes nothing here.** Every rule holds for as long as W is on.

The question: what protocol must each drain follow so that
- (a) any new lot-sum-awards fire a drain causes is detected and read before it is trusted;
- (b) a W-input change on a served head (a P head re-deciding differently) is caught;
- (c) the box budget and the chain window are respected;
- (d) U9's daily count stays interpretable.

---

## 1. Judges in brief

### Judge 1: risk-first

- **Two harms.** A drain can hurt path W in only two ways: an unread NEW fire, or a served head that re-decides.
- **Exact catches.** For a re-decision: R (the served registry) PK-read before and after every run, tolerance 0. For NEW:
  PK-read every live Tender with a change row in the drain's own cursor range.
- **Standing state.** Registry R and checkpoint K (R0 = `u9/20261023/switchoff.json`, K0 = 959180852). The switch-off twin
  is covered at the start of every window. The code-drift ledger continues.
- **Pre-step before every drain: a fixed-point probe.** `refold-lot-sum` over R ∪ Rg ∪ pins (about 3,550 ids), with a bar
  of 0 value moves. Under deployed code that probe IS "the port diff on the served set".
- **Where the port is still needed.** Only where the store cannot preview the drain's facts:
  - W-input facts on refetched R ∩ touched;
  - in-place writers, modelled;
  - 577's shadow parse;
  - lot-key code, held until U9 ends.
- **Per-run check.** If CA = CB, two GETs prove it. Otherwise a generalised D5 with verdicts SERVED / NEW / PINS / BAND / D7.
- **Order.**
  - The W-input drains (539 step 2, 579) are held until the day-7 read PASSes (10-28).
  - The non-W deploys and drains run in U9 days 8–14.
  - 546 U4 runs after 11-04, as a cohort walk (B) plus 546's own refold (C), with C checked only by R, pins and band.
  - In-place writers run after 11-04.
- **Budget.** ≤ 150 statements an hour.

### Judge 2: throughput-first

- **Verdict.** No drain needs W off or U9's end. All six clear between 10-23 and about 10-27.
- **Premise 1.** Every path-W effect is a served head change, announced by a `tender changed` row (rule T).
- **Premise 2.** The cohort SQL reads only head amounts and lot results.
- **Cheaper enumeration.** A tender-kind keyset on the index `changes_entity_cursor (entity_kind, cursor)` needs about 7–8×
  fewer statements than the all-kind chunks.
- **Classes.**
  - Z: zero corrections by design.
  - N: non-W, with corrections.
  - W: W-input, R-immune or measured.
  - L: lot-key (none pending).
- **C9 narrowed** to a port diff on R ∩ scope(drain). It is discharged by record where already measured (539) or empty (579).
- **Instruments I1–I6.** R/K; G-D5; a switch-off checkpoint; marker hygiene; a probe after every deploy; an optional
  project-summary counter.
- **Order.**
  - Tonight: deploy, twin, probe, 539 both steps.
  - 10-24: 546 U4.
  - 10-25: the batched U2 deploy, then 576 U3.
  - 10-26: 577 U3.
- **Budget.** 280 statements an hour.

### Judge 3: measurement-first

- **Fixed point.** S_now (3,478) sits at the current code's fixed point. Only a fact change or a code change can move it.
- **539 is a W input.** It was deployed 10-19, after 2627, and moves current_seq and supersession. Its W effect already
  leaks into U9's daily folds, so holding 539 keeps the leak running; it does not keep U9 clean.
- **Invariant I-SEG.** Every write since C1 belongs to exactly one single-writer segment, closed by a read before the next
  writer starts.
- **Kit fixes before reuse.**
  - D5's allowed set must be R, not P ∪ L_changed.
  - D6 must be judged against a per-drain pre-snapshot.
  - The 1.0974 % rate is a tripwire only: it rests on 8 events, a 95 % range of 0.47–2.16 %.
  - A one-statement D5-join.
- **Proxies.** A port re-run on unchanged facts is a proxy. The W-on probe (Rust on the store) is the measurement for deploys.
- **U9 as recorded is not a measurement.** It compared 3 fires with ~11,500 Tenders written, while the rate is per
  dry-admitted Tender. It needs A_d per chain.
- **Order.** 539 first (from 10-24), then 579, then the batched U2 deploy after the day-7 predraw, then 576 U3, then the 577
  slices. After 11-04: 546 U4, the 358 rehome, any lot-key drain.
- **Claim corrected later.** It held that a vouched_by-only re-decision may write no change row. A2 and W6 below correct this.

---

## 2. Synthesis, as adopted (critic amendments folded in and tagged)

### Verdict

No pending drain needs W switched off. The order:

0. The kit (E1–E6) and the start_rule clock fix (W2, W3, W11) are built offline. Then P0, the probe for 546 U3's deploy.
1. 539 step 1, then step 2 in three runs, from the 10-24 gap.
2. The day-7 read on 10-28.
3. One batched deploy and its probe P1. The batch holds 576 U2, 578 U2 and 579 U4, plus 577 U2 only if W7's intersection is
   empty, plus 575 U1 optionally.
4. 576 U3.
5. 546 U4. It runs in U9 days 8–14 only if E1 is proven; otherwise after 11-04.
6. 577 U3, in slices, after 546 U4 (R1).

**579 is not a drain (W1).**

These wait until after 11-04 and then follow C9':
- the in-place W-input writers: 578's 358 follow-up, 440/473 runs, 519 M6;
- 575 U3/U4;
- 561 U4;
- any lot-key drain.

Every write is a single-writer segment, closed by an exact check (G-D5) against the served registry R before the next
writer starts.

### A. Facts that settle the judges' disagreements

**A1. The switch-off is covered now.**
- Twin adcb7b0 (e718684 reverted on 512b297) was gated green after 546 U3's deploy (e79114c; 551 U9 entry, ~19:00Z).
- 546 U3 (512b297) is deployed and has had no W-on probe yet.

**A2 (corrected by W6). When a fold announces a served head change.**
- A fold that re-derives a head without appending compares the full StoredHead, derivation and derivation_detail included
  (canonical.rs ~24370-24389). A difference writes rule T's `tender changed` row.
- **On an append the head compare is skipped** (`if shadow && !tally.appended && !head_equal`, canonical.rs:24386). The append
  is announced by its transition rows, and those write a tender row only if the version's facts, rounds, group_members or
  closed flag differ, or if the EUR value moved.
- So a "silent" append can move current_seq, current_published_at and even derivation_detail/vouched_by at equal EUR, with no
  tender row. Judge 3's claim is wrong for non-append folds and partly right for appends.
- E1 stays exact for NEW: path W's admission requires an award copy or nothing as today's head, so a new fire always moves EUR
  and writes the row. The R read stays mandatory for MOVED.

**A3. The keyset index exists in code** (`changes_entity_cursor ON changes(entity_kind, cursor)`, canonical.rs:16205 and 18225).
- It is built lazily at the end of a projection, not at open.
- Confirm it on prod with a free sqlite_master read before any E1 statement (W12).

**A4. The kit cannot be reused as it is.**
- D5's allowed set is P ∪ L_changed (dchecks.py:352). A drain that refolds 8859070 or 8859315 would read SWITCH OFF.
- D6 judges "new" against the W=false U6 snapshot (dchecks.py:375-377).
- d3's VAL includes current_seq and current_published_at, and d3 compares against the 10-21 pre-flip snapshot (W5).
- lc_other_changes covers named ids only.

**A5. switchoff.py re-reads everything since C1 = 959004090.**
- On 10-23 that was 51 statements.
- It grows by about 20 a day, and by hundreds after a corpus drain. That breaks D7's 20-minute drain-back without a
  checkpoint.

**A6. 539 is deployed and still a W input.**
- 539 has been deployed since 10-19 (47def3f is in 0787596 and e718684). Every fold of a DÖE Tender re-dates its versions,
  so its W effect already flows into U9's daily folds.
- R is at 539's fixed point:
  - probes 2656–2658 on 0787596 and 2677–2679 on d617a78 corrected 0 on the 3,502;
  - the delta 8 and the daily fires were folded under e718684 ⊃ 47def3f.
- It is still a W input: version order sets current_seq, which `lot_sum_cohort_sql` and the head read.

**A7. R ∩ S7 = ∅.** Computed offline from u6/lchanged. After W1 it carries no decision weight.

**A8. U9's 10-23 count used Tenders written, not dry-admitted** ("3 fires over ~11,500 Tenders written").
- The 3× rule needs a measured admitted count per chain (C8).

**A9 (W2). The daily chain is the 09:35 Europe/Berlin tick** (supervisor.rs `next_berlin_tick(now, 9, 35)`).
- It is 07:35Z through 10-24 and **08:35Z from 10-25** (EU DST ends 10-25 01:00Z).
- `u7/start_rule.py` `next_chain` hardcodes 07:35 UTC (lines 29-31). From 10-25, between 07:35Z and 08:35Z, it would report
  the next chain as tomorrow and say GO for a multi-hour drain.

**A10 (W3). The weekly report tick is a writer window.**
- It runs Sunday 03:10 Berlin (`REPORT_TICK (0, 3, 10)`), which is 02:10Z on 10-25 and 11-01.
- It queues the censuses, data-quality (~100 min), rehash-probe, wet build-org-match-keys, org-merge-health, dry+wet
  scan-org-match-keys and wet audit-fts-ids: about 2 h on the single queue.
- A restart (any deploy) can also serve a missed daily tick at startup (`catch_up_missed_tick`).

**A11 (W13). Disk.** `/` had 4.5 GB free on 10-23 ~20Z; `target/` holds 16 GB.

**A12 (W1). 579 unit 1 is DONE** (579 Decision 2-3).
- No refold of S7, and no explicit refold of the 6 open Tenders. They heal at their next natural refold.
- 579 U4 (dissolve-path parity) rides 578 U2's deploy as its own commit.

### B. Standing state, required before any writer

**B1. Segments (I-SEG, with W10).**
- Every write since C1 belongs to exactly one single-writer segment: a chain, a weekly tick, a probe, or one drain run.
- At CB, just before the wet POST, record:
  - the /health cursor;
  - /admin/jobs with `.current` and `.queued` empty.
- At CA, after the job's project, record:
  - the /health cursor;
  - the jobs started in (CB, CA].
- **The segment passes only if those are exactly its own job ids.** Any other job (another agent's, the weekly tick, a
  catch-up tick after a restart) is a STOP.
- Before each wet POST, write a writer-lock line ("Writer: 551 <segment> until <ISO>") in 551 and on the drain's issue.
  Clear it at close.
- A segment closes when its check over (K, CA] completes. A fire belongs to the segment of its first lot-sum-awards write.
- No writer runs between a chain and its U9 check. On 10-28, none runs until the day-7 bodies are fetched.

**B2. Registry R and checkpoint K (all three judges; W9).**
- R0 = `b6/u9/20261023/switchoff.json`: 3,478 ids, ids_sha256 `8845f0d03f91defc…`. It is P 3,467 + delta 8 + the daily
  fires 97901, 8859070 and 8859315.
- Each entry holds its served columns (cents, currency, eur, detail.lots, vouched_by), taken offline from the 10-23
  cand/changed archives.
- Source tags: P / delta-U8 / chain-<date> / drain-<issue>-<run> / probe-<rev>.
- B_R = B_P (29). K0 = 959180852. Rg (35) and the pins are read with R every time.
- **K advances when a check's box-read classification (E1 + PK + band) completes with no STOP, SWITCH OFF or NOT A
  MEASUREMENT.** The drain's NEW ids enter R as `drain-<issue>-<run>, read pending`, so a later chain's gd5 attributes them
  to their drain and never counts them as daily (W9).
- **"Trusted" is separate.** The next writer starts only after the NEW reads pass. NEW reads not finished by the gap's latest
  switch-off start count as failed, which means switch off (W9).
- A STOP or NOT A MEASUREMENT freezes K and blocks the next writer.

**B3. The switch-off stays covered (with W13).**
- At the start of every window: `marker_covers <twin>` prints COVERS and `gate_deploy_step <twin>` prints skip. Today the
  twin is adcb7b0.
- After every deploy:
  - rebuild the twin (e718684's 9 files reverted on the new base);
  - gate it last.
- While the twin is gating, start_rule gets `--switchoff-gate-minutes 35` and no W-input segment starts.
- No other `ops/check.sh` runs in this checkout while a W-input segment or its check is open: a gate overwrites the single
  marker, as 1730d45's did.
- No second worktree: a from-clean gate costs about 28 GiB.
- **≥ 15 GB free** (after pruning the scratchpad and old archives) before any gate.
- **A gate that ends GATE-EXIT ≠ 0** (101 with no FAILED line included) means the switch-off is treated as uncovered:
  - HOLD every segment;
  - budget a from-clean gate (cargo clean, then about 35 min) in start_rule until a twin marker covers again.

**B4. A W-on probe after every deploy that touches crates/ingest or crates/store.** Now that means P0, for 546 U3.
- **The probe.** `refold-lot-sum` with tender_ids = R ∪ Rg ∪ pins, one run of ≤ 5,000 (dry, make_wet.py, wet with expect,
  project). It is checked as its own segment, class D.
- **The bar.** 0 moves in R's W columns: derivation, cents, currency, eur, detail.lots, vouched_by membership.
- **Notes, not failures:**
  - seq/published_at moves;
  - the deploy's own non-W columns (for 546, place/country on legacy heads);
  - an eur-only move on a re-dated head, which is explained.
- **One fold-changing commit per probe.** A repair-job or dissolve-path commit may ride along. A parse-only commit rides along
  only if W7 passes.
- The probe is the deploy's entry in the code-drift ledger (C1).

**B5. Box budget.**
- At most 150 /v1/sql statements an hour for this protocol (the 300 is shared), one at a time through the paced Box
  (≥ 2.5 s apart).
- Never read while a job is running; read after its project.
- A statement that does not answer is NOT A MEASUREMENT: run box_health, never retry blind, never retry a 408.

**B6. Clock (W2, W3, W11).**
- **Deadline.** start_rule's deadline is the earlier of the next 09:35 Berlin tick and the next Sunday 03:10 Berlin tick.
  The 2 h switch-off margin is kept before either.
- **Weekly tick.**
  - A segment, with its check and reads, closes before the weekly tick.
  - A switch-off that would have to start inside the weekly jobs' window (~02:10–04:10Z) counts their ~2 h as extra queue
    wait.
  - The weekly tick's range is closed as its own class-Z segment: E1 over it, 0 tender rows expected.
- **The U9 check never starts by the clock.** It starts when /admin/jobs shows that day's probe, process, fetch-rates,
  rederive-eur-recent, project and reveal-recheck finished, with `.current` and `.queued` empty.
- **A running project is never cancelled.**
  - Once a refold has re-queued and stamped, cancelling leaves epoch-stale Tenders for the chain's `project rebuild=false`.
  - So for a corpus job, D7's "switch off before the chain" is unavailable mid-project.
  - start_rule therefore takes the job's UPPER-bound minutes.
  - If a project overruns anyway, the chain queues behind it (FIFO). CA is read at project end, before the chain's first job,
    and the U9 check runs after.

### C. Kit extension

All of it is offline in `.scratch`, pinned in test_kit.py, with sha256s recorded in 551 before first use. It costs 0 box
reads except where noted.

**E1. `kitlib.sql_tender_keyset(after, hi)`**

`SELECT c.cursor, c.entity_id, c.op, (SELECT t.current_value_derivation FROM tenders t WHERE t.id=c.entity_id) AS d FROM
changes c WHERE c.entity_kind='tender' AND c.cursor>? AND c.cursor<=? ORDER BY c.cursor LIMIT 5000`, paged by the last
cursor.

- **Plan probe.** Probe it locally with `kitlib.plan_check`. It must SEARCH `changes_entity_cursor` and look up tenders by
  INTEGER PRIMARY KEY. Paste the plan into 551 (prod-box-reads).
- **Before first live use (W12):**
  - (a) one free sqlite_master read confirms the index exists on prod;
  - (b) one replay over (959004090, 959180852]. Its tender-row id set must equal the tender-row ids in the archived
    all-kind enumeration `u9/20261023/build_changes_2026-10-23T184801Z.tar.xz`, and its lot-sum-awards set must equal R0's
    3,478 minus any id moved since. This costs 10–20 statements, once, before E1 replaces the all-kind reader anywhere.
- **Small spans.** Spans of ≤ 50k cursor values always use the all-kind reader, so D5's hygiene checks (unknown kinds,
  vanished lots, retirements) stay.
- **Fallback if the plan SCANs:** the kit's `enumerate_changes` + `pk_read`.

**E2. `u9/gd5.py read|check <seg> --k K --cursor-after CA --class {chain,weekly,D,Z,N,W,I}`**

- **read:**
  - E1 over (K, CA];
  - PK-read the NEW candidates;
  - PK-read R ∪ Rg ∪ pins (2 statements);
  - the band seek (`read_band`, sql then API; 1 statement);
  - write out the changed live ids for the dry A job.
- **check classifies (per class, section F):**
  - NEW: serves lot-sum-awards and is not in R.
  - MOVED: in R, but S, currency, n, eur or **detail.lots** (the summed keys and their notice ids) differ, or vouched_by is not
    among its recorded vouches (W15).
  - LOST: in R, no longer lot-sum-awards.
  - SILENT: an R, Rg or pin row differs from the registry in a value column, and the Tender has no tender row in the range.
    seq and published_at are ignored (W6).
  - NOTE.
  - BAND: dchecks.d4 against B_R plus the registered band rows.
  - PINS (W5): compared against the registry row, not U6's `ctx.pre`, on value columns only (derivation, cents, currency,
    eur, detail). A seq/published_at move is a NOTE.
- **Exit codes:** 0 PASS, 10 SWITCH OFF, 2 STOP, 3 NOT A MEASUREMENT.

**E3. `u9/registry.py init|advance`.** advance follows B2/W9: it adds tagged NEW ids (read pending), records explained moves,
and sets K = CA.

**E4. switchoff.py build|verify gains `--registry R --base-cursor K`.**
- Candidates = R ∪ Rg ∪ pins ∪ the live Tenders with a tender row in (K, N].
- The C1 route stays as the fallback.

**E5. `u8/delta.py` gains `--population --salt --out`,** so each drain's NEW set and the day-7 population get a predraw before
the draw.

**E6. `u7/start_rule.py`:**
- `next_chain` = the next 09:35 Europe/Berlin (zoneinfo, as `next_berlin_tick`);
- the weekly tick per B6;
- `--runs-left` and `--minutes-per-run` (job + project + check + **expected read time**, about 45 min for ≤ 30 heads with
  reader, skeptic and third reader; W9; the UPPER bound for corpus jobs, W11);
- `--switchoff-gate-minutes` 0 when covered, 35 when uncovered, from-clean when W13 says so.

**Not adopted:** Judge 2's I6 project-summary counter. It needs a deploy, a twin re-gate and a probe during U9. File it as a
U10 follow-up.

### D. Per-segment check G-D5 (every probe, drain run, chain and weekly tick)

1. **Preconditions.**
   - B1–B6 hold.
   - start_rule says GO.
   - Pre-registered in 551:
     - the class;
     - the expected NEW count and served moves;
     - the salt `sha256('551-b6-drain-<issue>-'+id)`;
     - the bar text;
     - the writer-lock line.
2. **Zero-change shortcut (class Z, weekly).** PASS when all three hold:
   - corrected = 0;
   - CA = CB, or one E1 statement over (CB, CA] returns no tender row;
   - the jobs in (CB, CA] are exactly the segment's own.
3. **Otherwise run gd5 over (K, CA]** after the project.
   - For a W-input run, also take A: a dry `refold-lot-sum` over the segment's changed live Tenders (jobs of ≤ 5,000,
     0 statements).
   - cap = max(2·A·1.0974 %, A·1.0974 % + 10).
   - The cap is a tripwire, not an expectation.
4. **After a PASS:** advance the registry. The next writer waits for the NEW reads to pass.

### E. Bars

**NEW from a W-input drain (class W):**
- E5 predraw, then the draw: all if ≤ 30, otherwise the first 30 by salt.
- A reader and an adversarial skeptic per head, a third reader on disagreement (wf_dc85fa7e-c00's rubric).
- **U5's sequential bar.** With ≤ 30: 1 wrong fails. With > 30: 0 of 30 passes; exactly 1 wrong means the next 30 must hold 0;
  ≥ 2 wrong fails.
- Neutral heads outside the band are allowed and counted. A band row reads 0 wrong of any class.
- Over the cap: STOP before the next run.

**NEW from classes N, Z, D, weekly:**
- Expected 0. Each one is an enumeration miss or drift: read every one and explain its mechanism.
- More than 30 means the class premise is broken: programme STOP, a salted 30 under U5's bar, no further drain until
  explained.
- A fire on a 541/571 residual Tender (U6 deviation (a)) is read here as an enumeration miss, not as U7's D5 switch-off. This
  amendment is recorded in 551 before the first drain.

**MOVED / LOST / SILENT (classes D, Z, N, W, I):**
- Predicted 0 for every pending drain.
- Read all of them, never a sample.
- Port each MOVED head on refetched facts:
  - `fetch_b6 --refresh`;
  - port 23cbd850…;
  - rates refreshed after 10-27 (the U6 set's 7-day carry ends), with the new sha256 recorded.

**Deadline.** All NEW reads finish by the gap's latest switch-off start, or they count as failed (W9).

### F. Verdicts

**Probe, drain and in-place classes (D, Z, N, W, I).**

SWITCH OFF on any of:
- a failed NEW read, including reads unfinished at the latest switch-off start;
- a band row that reads wrong;
- a path-W pin off: 585917, 5995030 or 8426318 off its path-W value or vouch; 6465510 serving lot-sum-awards; 6908248 serving a
  double count; a partial sum (6679375, 7153588, 7398935, 7722197, 6779279, 6850401, 7698260) off its S or vouch;
- a registered band head missing. For 899272 and 5842187, within 2 % of EUR 100 m, it is stop-and-explain instead;
- a MOVED head that reads wrong;
- MOVED, LOST or SILENT still unexplained at the end of the gap;
- a W-input drain, in-place writer or deploy that landed without its pre-step;
- a class-I per-id mismatch against the port prediction (D1 rules);
- D7: a W-input run left cannot finish before the latest switch-off start.

STOP (no next run or drain, K frozen, explain or read within the gap) on any of:
- NEW over the cap, or any NEW in N, Z or D;
- any MOVED, LOST or SILENT;
- an extra band row (read it at 0 wrong of any class);
- a probe that moves a W column;
- a Z run with corrected > 0 or tender rows in its range;
- an unknown kind, a vanished lot, or a retired R member;
- a foreign job in (CB, CA] (W10);
- a lot-sum-awards band row outside R and outside the segment (an unread writer: find it);
- a value move on a path-V pin or a must-stay head. It is stop-and-explain (W5);
- a derivation or detail move on an R, Rg or pin row with no tender row but a seq increase. That is a fold announcement gap,
  not an in-place writer: stop and explain, and file it (W6);
- NOT A MEASUREMENT.

NOTE:
- seq/published_at only;
- non-W columns: place/country, party order, head-table rewrites of the 497154 shape, 579's 6 order-only corrections;
- an eur-only move on a re-dated head (explained; D4's 2 % near-band rule).

**Class chain (U9 daily) and weekly: own table (W4).** Section F's segment-wide lists above do NOT apply to chains.
- NEW: the daily fires. They are counted, registered as chain-<date>, and are not a STOP. The 3× rule applies to A_d.
- MOVED or LOST on an R head that has a tender row in the chain range: a counted daily re-decision. Update the registry and tag
  it chain-<date>. It is not a STOP; it is read under H.
- An eur-only move on a non-EUR head whose current_published_at is within rederive-eur-recent's 8-day window: NOTE.
- seq/published_at only: NOTE.
- A B_R head missing or moved: stop and read it at 0 wrong of any class. Switch off only if it reads wrong.
- An extra band row: read it at 0 wrong of any class. Wrong means switch off (U9).
- Pins:
  - a path-V or must-stay value move with a tender row in the chain: stop and explain;
  - a path-W pin that moves with a tender row: a chain-MOVED R head, read at 0 wrong;
  - 6465510 serving lot-sum-awards, or 6908248 serving a double count: switch off.
- SILENT: seq/published_at are ignored. A derivation or detail move with a seq increase: stop and explain (an announcement
  gap). Any other value move with no tender row: STOP (an unannounced writer), and switch off if it is unexplained by the end
  of the gap.
- The weekly segment is class Z: 0 tender rows expected, and any row is a STOP.
- U9's own switch-off triggers are unchanged: a band row read wrong, a day-7 failure, an unexplained count above 3×.

**SWITCH OFF means:**
1. deploy the covered twin (a revert commit whose tree equals the twin outside `.scratch`, push, `./deploy.sh`,
   `gate_deploy_step` = skip);
2. `switchoff.py build --registry R --base-cursor K`;
3. `refold-lot-sum` over that list (≤ 5,000 per run; re-check the D7 budget once R passes about 4,500);
4. `switchoff.py verify`: 0 lot-sum-awards and 0 band rows.

**HOLD (do not start) when:**
- the twin is uncovered and its gate (35 min, or from clean) is not budgeted;
- a gate ended with GATE-EXIT ≠ 0;
- disk is under 15 GB before a gate;
- a chain has run and its U9 check has not closed;
- it is 10-28 and the day-7 fetch is not done;
- a segment is open, its NEW reads are pending, or K is frozen;
- a fold change is deployed but not probed;
- start_rule says NO-GO;
- a W-input pre-step is not recorded;
- another agent's writer-lock line stands.

### G. Amendment C9' (recorded in 551 before 539 step 2)

The standing rule's "port diff on the served set" (551 point 12, decision-b6 C9) is met per class as follows, and by nothing
weaker.

**W-input refold under code already deployed (539):**
- The latest B4 probe plus the recorded fixed-point argument (A6).
- The probe is Rust on the store, given exactly the facts the drain will give R.
- Prediction: 0 R moves.
- A port re-run on unchanged facts is a proxy and is not done.

**In-place W-input writers (578's 358 follow-up, 440/473 runs, 519 M6, any dissolve run):**
- Apply the dry plan's moves to cached facts of R ∩ touched, offline, with port 23cbd850. Record a per-id prediction.
- Pair the writer with a refold-lot-sum of the touched cohort Tenders in the same segment.
- Once 579 U4 ships, a dissolve run's own project refolds its touched Tenders, so the pairing is built in. The prediction is
  still required.
- Otherwise the re-decision surfaces later as a fake U9 daily fire.

**Lot-key or W-input code (575 U3's identifier gate, 561 U4's epoch-5 head value, single-section placement, 567/570/571
follow-ons):**
- The full served-set port diff on refetched R plus the U9 pins, as written.
- After 11-04.

**Parse changes (577):**
- The 0-corrected bar is exact for its drain.
- The deploy's W-neutrality for daily folds needs W7's static intersection to be empty. The golden pin alone does not
  suffice, and P1 cannot show it, because refolds re-project stored parse rows and never run the new parser.

**579's 6 open Tenders:**
- Their natural heal inside another drain's segment is covered by that segment's gd5. They are registered as order-only NOTEs.
- An explicit refold of them, if ever wanted, is class W small with this discharge: they are outside the cohort (lc01 named all
  6 and admitted none), and the cohort SQL reads no org id.
- Prediction: 0 NEW, 0 R moves.

### H. Keeping U9 interpretable

- **The daily check** becomes gd5 class chain over (K, chain end]: about 2–3 keyset statements + 2 R + 1 band, instead of 51
  and growing.
- **A_d** = a dry refold-lot-sum over the chain's changed live Tenders (1–2 jobs, 0 statements). The 3× rule applies to A_d.
- Measure A for days 1–2 after the fact (13,453 Tenders, 3 dry jobs) before any drain writes.
- **U9 counts chain segments only.** Drain and probe fires are tagged and read under E. They are never counted daily and never
  enter the day-7 population.
- **The day-7 population stays C8's pre-registered one:** heads whose first lot-sum-awards write came from a daily fold, not
  in P. Chain-MOVED R heads are read as a separate population under the same bar, named in the predraw (W4; C8's text is
  not amended).
- Flag daily fires on DÖE Tenders before 539's segment as possibly 539-driven (A6).
- **Deploys during U9 must be W-neutral for daily folds:**
  - B4 for fold commits;
  - W7 for 577 U2;
  - 578 U2 and 579 U4 touch no fold.

### I. Conflicts resolved

1. **539 timing: Judge 3.** It runs first, from 10-24.
   - Over Judge 1 (hold until day 7): holding keeps the leak in daily folds (A6).
   - Over Judge 2 (tonight): the kit does not exist yet.
2. **546 U4.**
   - Not "now" (Judge 2), and not a hard hold until 11-04 (Judges 1 and 3).
   - It is non-W by construction. With E1 its exact check fits one gap.
   - It runs after day 7: a failed day-7 read turns W off, and the queue should not be held before U9's only quality read.
   - It runs after 576 U3 and before 577 U3 (R1).
   - If E1 SCANs, Judges 1 and 3 win: after 11-04.
3. **Probes run after every deploy (Judges 2 and 3), not before every drain (Judge 1).** A refold under unchanged code is
   the identity, and each segment's R read catches non-determinism.
4. **C9': Judge 3's measurement rule.** For 539 it gives the same result as Judges 1 and 2. The port is kept for MOVED and NEW
   heads, in-place writers and lot-key code.
5. **Enumeration:** Judge 2's keyset joined with Judge 3's PK subquery (E1). The all-kind reader is the fallback and is used for
   small spans.
6. **Judge 1's B+C plan for 546 is rejected.**
   - It is not exact for NEW: a non-cohort r209 Tender whose refold moves amounts or lot results through drift can enter the
     cohort below EUR 100 m unseen.
   - It doubles the queue time.
7. **One batched deploy (Judges 2 and 3)**, after the day-7 fetch (Judges 1 and 3). One fold-changing commit (576), one twin
   gate instead of three.
8. **Reads end inside the gap (Judge 3).** W9 refines it: K advances at classification, and the reads must finish by the
   latest switch-off start.
9. **150 statements an hour (Judge 1)**, because the 300 is shared.
10. **In-place writers wait until after 11-04 (Judges 1 and 3).**
11. **539 step 2 is staged by profile (Judge 1).**
12. **Judge 3's vouched_by claim:** wrong for non-append folds, partly right on appends (A2/W6). The R read stays.
13. **579 is not a drain (W1).** This overrides all three judges and the synthesis's draft.

---

## 3. Critic amendments

### Adopted (all non-optional; folded in above)

| # | Amendment | Folded into |
| --- | --- | --- |
| W1 | 579 is not a drain: unit 1 decided no refold of S7 and no explicit refold of the 6. U2 registers the 6 (408007, 413501, 454861, 179248, 340956, 823340; all TED, all outside the cohort) as order-only NOTEs in gd5 and in 546 U4's expectations. U4 rides the batch as its own commit (ledger entry; not a fold change, P1 covers it). Dropped: "S7 ∩ DÖE before 539 run a" and "579 before 576/577". A7 carries no weight. | Verdict, A12, G, table |
| W2 | DST: the chain is 09:35 Berlin (07:35Z until 10-24, 08:35Z from 10-25). Fix `next_chain` with zoneinfo. The U9 check starts on /admin/jobs evidence, never by the clock. | A9, B6, E6, H |
| W3 | The weekly Sunday 03:10 Berlin tick (02:10Z on 10-25 and 11-01) is a writer window. No segment spans it. start_rule uses the earlier tick. Its range is closed as class Z. The 10-24 gap ends at 02:10Z. | A10, B6, F, calendar |
| W4 | Class-chain verdicts get their own table. Chain MOVED/LOST is counted data. Fresh-FX eur moves are NOTEs. A B_R move in a chain is stop-and-read. The day-7 population is kept as C8 wrote it; chain-MOVED heads are read separately. | F (chain), H |
| W5 | Do not reuse d3 as written. Pins are compared against the registry on value columns only. A path-V or must-stay move is stop-and-explain. The path-W pins keep switch-off semantics in probe and drain classes. | A4, E2, F |
| W6 | A2 corrected: an append skips the head compare. SILENT ignores seq/published_at. A derivation or detail move with a seq increase and no tender row is an announcement gap (stop, explain, file). Why E1 stays exact for NEW is stated. | A2, E2, F |
| W7 | Before 577 U2 deploys during U9: statically intersect 577's full shadow/overlap pair list (reviewed + the 4,811 unreviewed, 577:134) with every field id or stem the fold reads (Amount, LotResult, StatedTotal incl. BT-27-Procedure at project.rs ~8866, published lot numbers, winners, tenderer refs). 0 statements; recorded in 551. Non-empty means 577 U2's deploy and U3 wait until after 11-04. | B4, G, H, table |
| W8 | 575 is classified. U1 (dry hex-census arm) is non-W and may ride the batch with a twin rebase. U3 (the `gated_identifier` gate) is a W-input deploy and U4 a W-input drain; both wait until after 11-04 under C9' (full port diff). `Blocked by: 551 C9'` lines are owed in 575, 519 M6, 578 U4/358, 440/473 and 561 U4. In this step only 578's could be written (section 7). | Verdict, G, table, §7 |
| W9 | Split "check complete" from "trusted". K advances at classification, with drain NEW ids registered "read pending" and excluded from later NEW counts. The next writer waits for the reads. Reads unfinished by the latest switch-off start count as failed. start_rule's minutes include the read time. | B2, D, E, E6 |
| W10 | A second writer becomes measurable: /admin/jobs at CB (empty) and CA (only own job ids). Writer-lock lines go in 551 and the drain's issue. | B1, F |
| W11 | Corpus jobs: never cancel a running project. On overrun the chain queues behind it and CA is read first. 546 U4 is budgeted from measured runs at ≥ 8 h job + project (550's 2531: 4.3 h for 1.52M corrected; 538's 2564: 4.9 h; 532: 3.3 h), plus E1 1–2.5 h, plus reads: 10–12 h in one gap. start_rule takes the upper bound. | B6, E6, table |
| W12 | E1 preconditions: the sqlite_master check on prod, and the replay over (959004090, 959180852] against the archived all-kind enumeration and R0, before E1 is used anywhere. | A3, E1 |
| W13 | Disk: ≥ 15 GB free after pruning before the batch gate and before the twin re-gate (4.5 GB on 10-23). A gate with GATE-EXIT ≠ 0 means the switch-off is uncovered: HOLD and budget a from-clean gate. "df ≥ 35 GB" is dropped. | A11, B3, F |
| W14 | The batched deploy happens only if gate, deploy, twin re-gate and P1 (with its gd5) all fit before that gap's latest switch-off start. Otherwise no deploy that day. | table, calendar |
| W15 | detail.lots joins E2's MOVED test. | E2 |

### Optional (listed, not adopted)

- **O1. A cheaper exact enumeration for corpus spans.**
  - What: take tender-row ids from the public `/v1/changes?entity=tender&since=K` (read.rs `changes_since`, a planner-clean
    seek). It costs no /v1/sql budget and is paced by fetch_b6. Then send one server-side filtered statement per span: rows
    whose Tender serves lot-sum-awards only.
  - Gain: tens of statements instead of 150–380 E1 pages for 546 U4.
  - Cost: its own plan probe and per-statement timing against the 15 s cap.
- **O2. Reuse after a STOP on a corpus segment.**
  - What: switchoff.py build (E4) reuses that segment's completed E1 read when no writer has run since, instead of re-reading
    (K, N].
  - Gain: it keeps D7's 20-minute drain-back budget after a frozen K.
- **O3. Re-cost 577 U3 as class N with gd5 per slice**, using the Z shortcut only on slices with corrected = 0.
  - Why: merges leave order-only drift (579 U6: `repoint_org_references` stamps nothing), so slices may correct for reasons
    other than 577.
  - Becomes the fallback route for 577 U3 if 546 U4 slides past 11-04 (R1).

---

## 4. Recorder's check R1 (checked against the code and issue texts on 2026-10-23)

**577 U3's 0-corrected bar also presumes 546 U3 has been drained.**
- 546 U3 changes place in `NoticeState::read`, and every fold re-derives the whole chain from the parse. 539's record says it
  of instants, and 546 U3's of "Tenders folded afterwards".
- 577's paired `project` therefore re-places every TED Tender whose chain mixes a legacy r209 notice with an eForms notice.
  That shape exists: 546 U3's own test is "a legacy CN chained to an eForms CAN".
- So 577 U3 run before 546 U4 would correct for 546's reasons and trip its own bar.

**Hence the order 576 U3 → 546 U4 → 577 U3.**
- 546's refold re-projects stored parse rows and never runs 577's parser, so 546 does not wait for 577.
- 546 runs after 576 U3, so its segment carries no undrained 576 change.
- If 546 U4 slides past 11-04, 577 U3 either waits behind it or runs as class N per slice (O3). In that case 577's own bar is
  read as "0 corrections outside the place-only and order-only NOTE classes", recorded in 577 before the first slice.

**576 U3 (class N) and 539 step 2 (class W)** may meet the same mixed-chain place moves. They are NOTEs there.

---

## 5. Final per-drain protocol

Common to every row: B1–B6 hold; G-D5 per segment (section D); bars per E; verdicts per F; C9' per G. "Tick" means the
09:35 Berlin daily tick (08:35Z from 10-25). Every writer's job, project, check and NEW reads end ≥ 2 h before the earlier of
the next tick and the next Sunday 03:10 Berlin tick. Costs are /v1/sql statements unless stated otherwise.

| Drain | Class | When | Pre-step | Per-run check | Bar | Stop / switch-off | Cost |
| --- | --- | --- | --- | --- | --- | --- | --- |
| **U9 daily chain** (days 3–14; the segment every drain respects) | chain | After each tick, once /admin/jobs shows the chain done and the queue empty; before any writer that day; on 10-28 also before the day-7 predraw, draw and fetch | B1–B3 hold. If E2/E3 are not ready, the 10-24 check uses `switchoff.py build` from C1 (≈55–70 statements). Before the first drain: A for days 1–2 (3 dry jobs over the 13,453) | gd5 class chain over (K, chain end] + A_d (dry refold-lot-sum over the chain's changed live Tenders) + registry advance | Daily fires counted, 3× rule on A_d; day 7 (10-28): C8's population (all if < 30, 0 wrong), plus chain-MOVED R heads read separately under the same bar | F's chain table; U9's own triggers (band read wrong, day-7 fail, unexplained > 3×) | ≈5–6 statements + 2 /health GETs + 1–2 dry jobs; 20–30 min |
| **P0** (546 U3 deploy, 512b297); later **P1** (after the batch) | D | P0: once the kit (E1–E6, the start_rule fix) passes offline tests and start_rule says GO; tonight if ready, else 10-24 after the U9 check; before any drain. P1: right after the batch, once its twin is gated last | Twin covered (adcb7b0 for P0; P1 needs the new twin); ledger entry; sqlite_master check + E1 plan probe + E1 replay (W12) before P0; make_wet.py over R ∪ Rg ∪ pins (≈3,550, one run). Expect 0 W-column moves; P0 notes place/country on R's legacy r209 heads | gd5 class D (P0 also runs the all-kind reader beside E1) | 0 W-column moves; NEW expected 0, any one read | W-column move: STOP; unexplained at gap end or read wrong: switch off; path-W pin off: switch off | 4–5 statements per probe; replay 10–20 + 1 once; jobs ≈10–15 min |
| **539 step 1**: repair-notice-instants (doe) | Z (re-stamps notices.published_at only; writes no change rows) | After P0; the 10-24 gap (or tonight after P0 if ready); never between a tick and its U9 check | Dry; review by-profile counts (≈16,344 plus inversions ingested since 10-19); wet with reviewed expect_rows; CB + /admin/jobs; writer lock | Z shortcut (CA = CB, or one E1 statement shows no tender row; only its own job) | 0 tender rows | Any tender row: STOP, then gd5 over the range; a foreign job: STOP | 0–1 statement, 2–4 GETs; 0.5–1 h |
| **539 step 2**: R3 refold of sdk-0.1, de-1.0…de-2.1 (≈1.16M notices) with its trailing project | W (version order → current_seq, head, supersession); medium by corrections (≤ 16,344 re-dated versions, ≈3,800 reorders); never-sampled NEW population | 10-24 after the U9 check, P0 and step 1. Three runs: (a) sdk-0.1, (b) de-1.0–1.2, (c) de-2.0–2.1, a STOP point between runs, start_rule GO per run. All closed before the 10-25 02:10Z weekly tick, else run (c) on 10-25. Never between the 10-28 tick and the day-7 fetch | C9' discharge recorded (A6 + P0): prediction 0 R moves; seq/published_at NOTEs; eur-only moves from re-dates explained (899272/5842187: 2 % rule). Cap formula, salt `551-b6-drain-539-`, bar text, writer lock | Per run: gd5 class W over (K, CA] + A by dry refold-lot-sum + registry advance (NEW read pending) | NEW: all if ≤ 30, else salted 30, U5's sequential bar; band rows 0 wrong of any class; MOVED/LOST/SILENT predicted 0, read all, port on refetched facts | Over cap: STOP before the next run. MOVED/LOST/SILENT: STOP; wrong or unexplained at gap end: switch off. D7 per run. Reads unfinished at the latest switch-off start: switch off | ≈13–21 statements (E1 4–12 for ≈20–60k tender rows + 3 a run) + 4–12 dry jobs; NEW reads via public GETs; ≈3–4 h incl. ≈45 min read |
| **579**: no drain (W1) | none (the 6 are order-only NOTEs) | U2 (docs) now; U4's code in the batch; U3 read inside the 150/h budget and not on a U9 check morning | U2: register the 6 in gd5's NOTE list and in 546 U4's expectations; U4: ledger entry | Their natural heal is checked by whichever segment reaches them | Order-only corrections; any value move on them is a NEW/MOVED per that segment | Per that segment. An explicit refold of the 6 would be class W small with G's discharge; not scheduled | 0 |
| **Batched deploy**: 576 U2 + 578 U2 + 579 U4 (+ 577 U2 only if W7 is empty, + 575 U1 optional) | D (one fold-changing commit, 576; 577 parse-only; 578 repair job; 579 dissolve path; 575 dry census) | After 539 is verified and after the 10-28 day-7 fetch (10-28 afternoon or 10-29). Only if batch gate, deploy, twin re-gate and P1 with its gd5 all fit before the latest switch-off start (W14); else not that day | ≥ 15 GB free (prune first; 4.5 GB on 10-23); gate the batch only with no W-input segment open; deploy (`gate_deploy_step` = skip); rebuild the twin (e718684's 9 files reverted on the new base, keeping 576/577's CHANGELOG/docs/openapi hunks) and gate it last; 577's golden test (b) green; W7 recorded | P1 (class D) | 0 W-column moves | Any gate GATE-EXIT ≠ 0: switch-off uncovered, HOLD, from-clean gate budgeted (W13); P1 per the P0 row | Two gates of 25–35 min, deploy 10 min, P1 4–5 statements + ≈15 min; ≈1.5–2 h |
| **576 U3**: refold-sections kinds=[Touchpoint] | N, mid (≈160–175k carriers, ≤ 100–170k corrected Tenders); non-winner OPT-301 roles only | After P1 PASS and the day-7 read recorded PASS (≈10-29); before 546 U4 and 577 U3; outside the tick-to-U9-check slot | Bounded Touchpoint COUNT on the notice_sections_kind_notice prefix (no dry run, no expect); expected NEW = MOVED = LOST = 0; salt `551-b6-drain-576-`; writer lock | gd5 class N + registry advance (its tender ids then sit on the E4 switch-off list) | NEW expected 0: read every one; > 30 means the class is broken (salted 30 under U5's bar, programme stop); 546 place moves on mixed chains are NOTEs | Any NEW: STOP. MOVED/SILENT: STOP; unexplained at gap end: switch off | ≈25–40 statements (E1 20–35, R/band 3, sizing 1–4); ≈1.5–2.5 h |
| **546 U4**: refold of ted-export-r209 (≈4.5M notices) | N, corpus (≈0.75–1.9M corrected Tenders); place/country only, no W input; the legacy-TED reconciliation walk | After 576 U3 and before 577 U3 (R1). In U9 days 8–14 (target 10-30 → 11-03), in ONE gap starting right after the U9 check, only if E1 plan-probes as a SEARCH, passed its replay and ran with parity on ≥ 2 segments; E4 is in place; and the day-7 read passed. Otherwise after 11-04 (always, if the fallback reader would be needed) | P0 is its code probe (P1 covers later fold deploys); the dry cohort count and a reviewed expect; start_rule `--minutes-per-run` = the upper bound (≥ 8 h job + project + E1 + reads, W11), switch-off gate 0; 579's 6 registered as NOTEs; salt `551-b6-drain-546-`; writer lock | gd5 class N after the project (never cancel it; on overrun the chain queues behind it and CA is read first) + registry advance | Expected NEW = MOVED = LOST = SILENT = 0; each NEW read (all if ≤ 30, else salted 30 under U5's bar) and explained as an enumeration miss (e.g. a 541/571 residual); place moves on R's legacy heads are NOTEs | Any NEW: STOP. MOVED/LOST/SILENT: STOP; unexplained at gap end: switch off. Band and pins as usual | ≈150–380 E1 statements + 3, i.e. 1–2.5 h at 150/h; job + project ≥ 8 h; ≈10–12 h in one gap. The fallback reader adds 375–950 PK statements, hence after 11-04 over two gaps |
| **577 U3**: re-parse of 19 profiles (sdk-1.0/1.2/1.3/1.5–1.15, de-2.0/2.1, de-1.0/1.1/1.2) with its paired project | Z, self-checking (0 corrected, exact under W on); corpus by notices (≈3.3M) | After 539, 576 U3 AND 546 U4 (R1); after the batch with W7 empty (else after 11-04). In slices, the first packages-capped to time the rate, each ending ≥ 2 h before the next tick. If 546 U4 slides past 11-04: wait behind it, or run as class N per slice (O3) | The batch's golden pin, W7, P1; the slice plan recorded; writer lock per slice | Per slice: Z shortcut (corrected = 0 AND (CA = CB or one E1 statement shows no tender row) AND only its own jobs) | 0 corrected (577's own bar); never accept 0 corrected if tender rows exist | Any correction: 577's own stop, then gd5 class N on that slice, every NEW/MOVED read, ids registered in R | 0–1 statement per clean slice; gd5 only on a correcting slice (E1 ≈1 per 5,000 tender rows + 3); several gaps |
| **578**: U2 (440 lookup fix) in the batch; follow-ups: the 358 rehome (5,076 frozen rows; 30920766 → 2660729), 440/473 runs, 519 M6; also 575 U3/U4, 561 U4 and lot-key drains | U2: D (no fold change). Follow-ups: I (in-place W-input writers, org identity → R1/R6), small. 575 U3/U4, 561 U4, lot-key: L/W-input code | U2 in the batch. Every follow-up after 11-04, once filed and sized; never on a U9 check morning | U2: the shared twin gate and P1. Class I: C9' offline prediction (dry plan's moves on cached facts of R ∩ touched, port 23cbd850, refreshed rates), paired with a refold-lot-sum of touched cohort Tenders in the same segment (built in once 579 U4 ships). Class L: the full served-set port diff on refetched R + the U9 pins | U2: P1 only. Class I: gd5 class I + D1 semantics against the prediction, tolerance 0 | Unpredicted fire or wrong S/currency/n/vouched_by: fail. A predicted move that does not happen: explain within the gap | Unpredicted fire or wrong value: switch off. SILENT catches an unannounced write. Landed without its pre-step: switch off | U2: nothing beyond P1. Each follow-up ≈10–20 statements + sizing; port offline (0 box reads); < 1 h |

### Box and hours, through U9 (10-24 → 11-04)

| Item | Statements | Queue / wall |
| --- | --- | --- |
| Kit E1–E6 + start_rule fix + tests | 0 (offline) + 1 sqlite_master | offline |
| E1 replay (W12) | 10–20, once | ≈10 min |
| U9 checks, 12 days | ≈60–72 (+55–70 if the 10-24 check uses the old route) | 20–30 min/day + dry jobs |
| P0, P1 | 8–10 | 2 × ≈15 min |
| 539 steps 1–2 | 13–22 | ≈4–5 h, one gap |
| Batch deploy | (P1 only) | ≈1.5–2 h local gates + deploy |
| 576 U3 | 25–40 | 1.5–2.5 h |
| 546 U4 (if in U9) | 153–383 | 10–12 h, one gap |
| 577 U3 | 0–1 per clean slice | several gaps |
| **Total** | **≈270–560** (never > 150/h) | |

---

## 6. Calendar (W2/W3 applied)

- **10-23 night:**
  - record this decision, C9' and the U6(a) amendment in 551;
  - build E1–E6, the start_rule fix and test_kit;
  - plan-probe E1 locally; initialise R0/K0.
  - P0 and 539 step 1 only if all of that is ready and start_rule (deadline 10-24 05:35Z) says GO.
- **10-24 (Sat, day 3):**
  - tick 07:35Z, then the U9 check (+ A_d; A for days 1–2);
  - the E1 replay;
  - P0 and 539 step 1 if pending;
  - 539 step 2 runs a, b, c, then the NEW read.
  - Everything closed before the weekly tick, 10-25 02:10Z, with the switch-off budget.
- **10-25 (Sun):**
  - the weekly tick 02:10Z, closed as class Z;
  - the tick now runs at **08:35Z**;
  - U9 check; any leftover 539 run.
- **10-25 → 10-27:**
  - build and review 576 U2, 577 U2 (+ W7), 578 U2, 579 U4 (+ 575 U1);
  - prune the disk to ≥ 15 GB;
  - 579 U2 registrations; 579 U3's read.
- **10-28 (Wed, day 7):**
  - tick 08:35Z, U9 check;
  - day-7 predraw (C8 population + chain-MOVED set), draw, fetch, read;
  - no writer until the fetch is done.
  - Then the batch + twin + P1, if W14 fits; else 10-29.
- **10-29 → 10-30:** 576 U3, after P1 PASS and the day-7 PASS.
- **10-30 → 11-03:**
  - 546 U4 in one ≈10–12 h gap, if its E1 conditions hold. The 10-31 gap ends at the 11-01 02:10Z weekly tick, so prefer
    10-30 or 11-02.
  - Then the 577 U3 slices.
- **After 11-04 (U9 day 14 recorded):**
  - 546 U4 and 577 U3 if they slid;
  - then the class-I writers (358 follow-up, 440/473, 519 M6), 575 U3/U4, 561 U4 and any lot-key drain, under C9'.

---

## 7. Owed outside this step (this step edits only 546, 539, 579, 576, 577, 578 and 551's U9 section)

- **`Blocked by: 551 C9' (after 11-04)` lines (W8)** in:
  - 575 (U3, U4; U1 may ride the batch);
  - 519 M6;
  - 473 (its 440 runs);
  - 561 U4.
  - 578's is written in its paragraph.
- **The kit:** E1–E6, the start_rule fix (W2/W3/W11), test_kit pins, and sha256s in 551 before first use.
- **Recorded in 551 before 539 step 2:**
  - C9' (section G) with its evidence;
  - the U6(a) amendment;
  - the E1 plan;
  - R0/K0 and their sha256.

## Files

- `/home/user/tender-db/.scratch/tender-db/551-lot-sum/b6/decision-b6.md` (U9, D1–D7, C1–C10)
- `/home/user/tender-db/.scratch/tender-db/issues/551-a-refused-head-falls-to-one-lots-figure-or-null-never-the-lot-sum.md`
  (B6 points 1–14, U6–U9)
- `/home/user/tender-db/.scratch/tender-db/551-lot-sum/b6/u6/RUNBOOK.md`, `u6/kitlib.py`, `u7/dchecks.py` (D5 :352, D6
  :375-377, d3), `u7/switchoff.py`, `u7/start_rule.py` (:29-31), `u8/verify.py`, `u8/delta.py`,
  `u9/20261023/switchoff.json` (R0)
- `/home/user/tender-db/crates/store/src/canonical.rs` (head compare ~24370-24389; index 16205/18225)
- `/home/user/tender-db/crates/app/src/supervisor.rs` (`next_berlin_tick` 20440, the 09:35 tick, `REPORT_TICK` 19687)
- Issues 539, 546, 575, 576, 577, 578, 579 (`.scratch/tender-db/issues/`)

## Owner amendment W13' (2026-10-23 ~20:40Z) — the disk precondition, re-based on measurement

W13's "≥ 15 GB free before any gate" cannot be met in this container without `cargo clean`: the writable allowance is
~38.5 GB, of which `target/` holds 16 GB (the incremental gate's own working set), the scratchpad 3.8 GB, `/root/.claude`
2.7 GB, `/root/.cargo` 1.1 GB. A `cargo clean` to reach 15 GB would force the 35-min from-clean gate that W13 exists to
avoid. Measured instead: the last four gates in this checkout — 84c5118 (10-21), d569107 (10-21), 1730d45 (10-23) and
adcb7b0 (10-23) — each ran green (GATE-EXIT=0) starting from 4.5–5.3 GB free, because an incremental gate over a code-only
diff reuses `target/` and the gate prunes superseded archives first. **W13' (replaces W13's threshold):** before any gate,
(a) ≥ 4 GB free; (b) no build-input re-hash pending (Cargo.lock, a Cargo.toml, `[patch]`, a profile or the toolchain
unchanged since the last green gate), so the gate's auto-clean cannot fire; (c) read GATE-EXIT from the log, and a
non-zero exit (any cause, ENOSPC included) is W13's HOLD — the switch-off is uncovered, budget a from-clean gate. If (b)
fails, the 15 GB rule stands for that gate (prune, or do not gate that day).
