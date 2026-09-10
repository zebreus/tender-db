# 375 — two backfill jobs still compute the head columns with a raw extremum, so running either silently undoes issue 366's drain

Status: DONE 2026-09-10 — every unit, every "Done when" and the live observation are met; the label
said `ready-for-agent` for a firing after that became true, which is exactly the drift `/triage` exists
to catch. Was: ready-for-agent — **UNITS 1, 2 AND 4 DONE 2026-09-10, and the severity was understated when
filed. This is not latent: `rederive-eur` chained `backfill-values` AUTOMATICALLY and its completion
message instructed it too, so a routine rates correction would have reverted issue 366's election
corpus-wide.** The chain is cut, `backfill-values` refuses with the reason, `backfill-deadlines` is
repaired (it was the transcribable half), and the discriminating test exists. **UNIT 3 DONE too: the
successor is built — `rederive-eur` now reports the tenders whose value actually moved and stamps
exactly those epoch-stale, so the FOLD re-elects them on the next `project`. The last divergent
writer is DELETED and the writer count is pinned by a test — `current_value_eur_cents` and
`current_deadline` now have exactly two writers, the fold's own election and the deadline backfill
that transcribes the horizon faithfully. THE LIVE RUN IS DONE TOO (job 1001): 0 of 267,401,093 money
rows changed, 0 tenders stamped — the wiring works, the no-op case does not stamp the corpus, and the
daily rate reload is confirmed not to move `eur_cents`.** Every unit, every "Done when" and the live
observation are met. **Nothing is open.** Was: ready-for-agent (filed 2026-09-10 by the owner while landing 366 unit 3; found by reading
the third implementation rather than by a failure, and it had no test that could catch it)
Kind: defect (corpus regression on a routine maintenance path — not latent, see the escalation below)
Relates to: 366 (the election these two disagree with, and the drain they would undo), 343 (the same
shape: two places computed one election and disagreed), 216 (the deadline backfill's original job
706), ADR-0014 D5 (the value backfill)

## Observed

`head_value_eur_cents` and `head_deadline` have filtered sentinels, the €100 bn ceiling and the
ten-year deadline horizon since `aa732c5`, and the ~16,500 standing rows were drained to match on
2026-09-09/10. **Two backfill jobs still write the same two columns with unfiltered aggregates.**

`Db::backfill_current_deadline` (`crates/store/src/lib.rs:3051`):

```sql
UPDATE tenders SET current_deadline =
    (SELECT MAX(d.utc_seconds) FROM tender_version_dates d
      WHERE d.tender_id = tenders.id AND d.seq = tenders.current_seq
        AND d.field = 'submission_deadline')
```

`Db::backfill_current_value_eur` (`crates/store/src/lib.rs:3141`):

```sql
UPDATE tenders SET current_value_eur_cents =
    (SELECT MAX(a.eur_cents) FROM tender_version_amounts a
      WHERE a.tender_id = tenders.id AND a.seq = tenders.current_seq)
```

No horizon, no ceiling, no sentinel test, no `quality IS NULL`. Running either over the corpus
re-stamps every row the drain fixed: 3323836's deadline goes back to 3005-07-06, 4490098's value
back to €4.97×10¹⁶, and the ~15,600 −1.00 rows return to the value bounds.

**Both docs assert the agreement they no longer have**, which is what makes this a trap rather than a
known limitation:

- deadline: *"the same MAX-over-the-version's-rows (lot rows included) the fold's `head_deadline`
  computes in memory for new writes"*
- value: *"The aggregate mirrors `head_value_eur_cents` exactly — same population, already-derived
  values, so the two can never disagree on a rate."*

Both sentences were true when written. Neither is now, and a reader checking whether it is safe to
run a backfill will find an explicit promise that it is.

## Why the tests cannot catch it

`crates/store/tests/deadline_backfill.rs` exists and its header claims the fold, the backfill and the
list pick "all three answers agree" — but its fixture publishes deadlines at 5000/7000 seconds against
`current_published_at = 100`, comfortably inside a ten-year horizon, so the filter's absence is
invisible to it. The test is not wrong; it was written before there was a filter to miss.

That is the general shape worth noting: **a test that pins agreement between two implementations only
pins it on the inputs where they were already going to agree.** Discriminating inputs have to be
chosen on purpose, and this test's were chosen to exercise MAX-over-rows, lot rows and head-vs-non-head
— every axis except the one that later mattered.

## Units

1. **Stop the bleeding.** Make both jobs refuse to run unless explicitly forced, naming
   `refold-notices` as the correct route and this issue as the reason. Cheap, and it removes the
   trap while the disposition below is decided.
2. **Correct the two doc comments** so neither promises an agreement that does not hold. Do this
   even if unit 3 retires the jobs, because the sentences are wrong today.
3. **Decide the disposition, and it is a real decision:**
   - **Retire both.** They are one-time migrations (job 706 for 216; ADR-0014 D5 for the value) that
     have already run. Their remaining value is near zero and `refold-notices` — demonstrated over
     ~16,500 rows on 366 — does the job correctly by aiming the fold's own election. This is the
     option the drift argument favours: the election should have exactly one implementation.
   - **Fix them in place.** The horizon transcribes faithfully (one comparison against
     `DEADLINE_HORIZON_SECS`, as `tender_select_head` now does). The value side does NOT: the rule
     is a digit walk in `sentinel_amount`, and transcribing it into SQL is precisely the second
     implementation this issue is about. So "fix in place" can only be partial for the value column,
     which means the trap survives for the 322 repdigit rows.
   That asymmetry is the argument for retiring rather than fixing, and it should be recorded as the
   reason rather than re-derived.
4. **A discriminating test**, whichever way unit 3 goes: a fixture whose deadline is beyond the
   horizon and whose amounts include a sentinel, asserting that no path which writes the head columns
   disagrees with the fold. If the jobs are retired, the test becomes "no other writer of these
   columns exists", which is checkable by grep and worth pinning.

## Done when

- no code path writes `current_deadline` or `current_value_eur_cents` with a rule that differs from
  `head_deadline`/`head_value_eur_cents`;
- the two doc comments say what their code does;
- a test fails if a second implementation is reintroduced.

*One issue because:* both jobs are the same defect — a second implementation of an election that has
since grown filters — and the disposition (retire vs fix) has to be taken once for both, since the
value half is the one that cannot be transcribed and therefore decides the answer.

## ESCALATION: this was never latent — `rederive-eur` ran the corrupting job automatically (2026-09-10)

Filed above as "a runnable admin job that reverts a correctness fix", i.e. something an operator would
have to choose to run. **That was wrong, and the correction matters more than the original filing.**

```rust
"rederive-eur" => Ok(vec![
    self.push("rederive-eur", "rederive-eur".into(), Spec::RederiveEur).await,
    self.push("backfill-values", "backfill-values".into(), Spec::BackfillValues).await,   // ← this
]),
```

`rederive-eur` (issue 306) re-derives `eur_cents` across the corpus from the current rates table —
exactly what you run when a rate is corrected, and `fetch-rates` runs on the daily pipeline. Its
second step re-stamped every head value with the unfiltered `MAX`. **So the corpus-wide reversion of
issue 366's election was not something a careless operator might trigger; it was the documented,
automatic second half of a routine maintenance job**, with nothing in the job log to distinguish it
from a normal run. The completion message pointed the same way: *"follow with backfill-values (issue
306)"*.

**How the mis-severity happened, since it is the reusable part:** the filing was written from reading
the two `Db` methods and their docs. Who CALLS them is one grep further, and that grep is what turned
"latent" into "live". *Reading a defect's implementation tells you what is wrong; only reading its
callers tells you how often it happens.* The habit to keep is to grep for callers before assigning
severity, not after.

## What landed (units 1, 2, 4)

The two halves are not symmetric, and the fix follows the asymmetry rather than treating them alike:

- **`backfill-deadlines` is REPAIRED, not refused.** Its rule transcribes faithfully — one comparison
  against one constant — so the SQL now carries
  `AND d.utc_seconds - tenders.current_published_at <= {DEADLINE_HORIZON_SECS}`, interpolated from
  `canonical` exactly as `tender_select_head` does. The job stays usable.
- **`backfill-values` is REFUSED**, with the reason in the error. `sentinel_amount` is a digit walk;
  writing it in SQL would be the second implementation this issue is made of.
- **The automatic chain is cut**, and `rederive-eur`'s completion message now says the head columns do
  NOT follow and must be re-folded.
- **The walk body is kept unreachable rather than deleted**, because `rederive-eur` still needs a
  successor and that successor wants this batching/checkpoint/watermark structure.

**The discriminating test** (`the_backfill_refuses_a_deadline_beyond_the_horizon`) covers the 3323836
shape, the only-a-typo case, and the boundary — inclusive, matching `head_deadline`'s `<=`, because a
drifting comparison is the whole risk. Verified against its own negative: with the filter reverted the
new test fails naming 3323836 and **the old test still passes**, which is the point that section "Why
the tests cannot catch it" was making, now demonstrated rather than argued.

## What is left — unit 3, and it is a different question now

"Retire vs fix" is settled for the mechanism: the deadline half is fixed, the value half is refused.
What is NOT settled is the hole that leaves.

**`rederive-eur` has no correct successor.** It moves `eur_cents` on the satellites; the head values
must then be re-elected, and only the fold can do that. Options, none costed yet:

1. **Have `rederive-eur` collect the tender ids whose `eur_cents` actually changed** and stamp just
   those epoch-stale, so the next `project` re-elects them. `rederive_eur_window` already counts
   `changed` per window but does not record WHICH — that is the work. Cheapest by far if the changed
   set is small, which it usually will be (a rate correction touches one currency-day).
2. **A `PROJECTION_EPOCH` bump** after any rederive. Correct and enormous — measured at 6 h 02 m for a
   2.69 M-notice cohort — for what is typically a handful of rows.
3. **Leave the head columns stale** until something re-folds those tenders naturally, and say so. Not
   as bad as it sounds: stale-but-filtered is a value the election once approved, whereas
   freshly-wrong is a sentinel. But it is silent drift and needs at least a report line.

Option 1 is the obvious lead and matches what issue 366 learned — aim the fold rather than reimplement
it. **Until it exists, `rederive-eur` leaves head values stale on the rows it touched**, which is
recorded here rather than in the job's output because it is a property of the pair, not of one run.


## Unit 3 landed: the successor is a scoped stale-stamp, not a recomputation (2026-09-10)

Option 1 from "What is left", which was the lead and is also what issue 366 had already learned:
**aim the fold rather than reimplement its election.**

`rederive_eur_window` now returns the tender ids whose stored `eur_cents` actually moved, and the
`RederiveEur` arm stamps exactly those `projection_epoch = 0`. The next `project` re-elects their head
value through `head_value_eur_cents` — the filtered implementation, and the only one left that decides
this. `stamp_stale_for_tenders` is the third caller `stamp_tenders_stale`'s own doc had anticipated,
beside the profile join and the notice-id join, so the primitive was already there.

**Scoping is the substance, not an optimisation.** A rate correction usually touches one
currency-day, so the changed set is a handful of tenders. The alternative that would also have been
correct — a `PROJECTION_EPOCH` bump — costs 6 h 02 m for a 2.69 M-notice cohort every time a single
rate is fixed. Reporting *which* rows moved is what makes the cheap route available at all, and the
information was already in the loop: `rederive_eur_window` selected `a.tender_id` and threw it away.

Two details that are load-bearing rather than tidy:

- **Stamped per window, not accumulated to the end.** A crash mid-walk then leaves the finished
  windows correctly marked, matching the watermark's own resume discipline.
- **An empty cohort is a no-op.** "Nothing changed" and "everything changed" must not differ by a
  missing guard — an unguarded `IN ()` or a bare `UPDATE tenders` would turn the cheapest case into
  the most expensive one. Pinned by a test.

The two pre-existing walks in `currency_rates.rs` now assert the changed-id contract on both sides: a
window that rewrote rows names its tender exactly once (deduped across the four loci), and a window
that rewrote nothing names none. That second one extends the file's existing idempotence claim — which
was about WRITES — to re-elections, and it matters: a walk that stamped on every run would trigger a
full re-fold each time and quietly reintroduce the cost this unit exists to avoid.

### Not yet observed on a live run

`rederive-eur` has not been run since this landed, so the stamped count is untested against real data.
It is safe to run — the failure mode it replaced was silent corruption, and this one's is at worst
stamping too few — but the first live run should be read for two things: the ratio of stamped tenders
to `updated` rows (expected far below 1, since a tender carries several money rows), and whether the
following `project` actually clears them. Recorded here rather than assumed.

## The audit "Done when" asked for, and the last writer (2026-09-10)

*"No code path writes `current_deadline` or `current_value_eur_cents` with a rule that differs from
`head_deadline`/`head_value_eur_cents`."* Checked by grepping every assignment, and it was not yet
true: **`backfill_current_value_eur` had only been blocked at the JOB level**, so the method remained
a divergent writer and `lots_filter_fixture.rs` was still calling it.

Deleted. Both reasons for keeping it had expired without being revisited:

- *"`rederive-eur` still needs a successor and that successor wants this batching structure"* — the
  successor was then built and does not use it at all. It stamps epoch-stale instead.
- *"it can be repaired later"* — it cannot, for the digit-walk reason recorded above, and that was
  known when it was kept.

Worth naming as a habit: **a thing kept "for now" needs its reason re-read when the work that
justified it lands**, or it survives on a justification that has quietly stopped being true. That is
the same failure as the two doc comments this issue opened with.

The job KIND stays, refusal only, so an operator who enqueues `backfill-values` gets the explanation
rather than "unknown job kind" — the error is the useful artefact.

**The fixture's comment was its own small version of the same thing.** It said it used the real walk
because the fixture "needs it stamped exactly the way prod stamps it", which had already stopped
being true when the fold grew filters. It now computes the column inline, and the comment records the
property that makes that honest rather than a shortcut: its amounts are 10,000, 500,000 and
999,999,900 cents — no sentinel, nothing over the ceiling, and 9,999,999 is seven identical digits
where the rule needs nine — so the filtered and unfiltered rules agree on that data. If a value there
ever grows into one of those classes, the comment is the warning that it must move to the fold.

### The pin, and why counting is the only way to check this

`the_head_columns_have_exactly_one_writer_that_decides_them` reads the four source files and counts
assignments to either column. Two are expected: the fold's write, and the deadline backfill.

Counting is not a stylistic choice. **Each of the six writers this issue dealt with was internally
self-consistent** — the read pick, the two backfills, the fold — and every one would have passed a
unit test of itself. The defect existed only BETWEEN them, and appeared only when one changed. There
is no test of a single writer that could have caught it, which is exactly why it survived `aa732c5`,
the drain, and two rounds of reading the code.

Verified against its own negative: adding a fourth writer fails the test and names its file and line.

## The repaired deadline backfill, verified on the real corpus (job 903, 2026-09-10)

The fixes above were tested on fixtures. `backfill-deadlines` was then run against prod — safe by
construction now, and the strongest available check, since this is the exact job whose unfiltered
form would have reverted the drain.

**7,940,318 tenders stamped in 111 s**, `ok`. The decisive reading is what did NOT happen:

| tender | before the walk | after |
| --- | --- | --- |
| **3323836** | deadline **2005-06-15** | **2005-06-15** — the old walk would have put 3005-07-06 back |
| 26 | 2023-11-28, value €0.01 | unchanged |
| 34 | 2025-10-02, value NULL | unchanged |
| 43065 | 2026-02-17, value NULL | unchanged |
| 4490098 | 2011-04-08, value €50,000 | unchanged |

Beyond-horizon FUTURE head deadlines: **0 before, 0 after**. `?status=open&sort=deadline&order=desc`
still tops at 2036-04-30. The value column is untouched, as it must be — this walk writes only
`current_deadline`.

**It also closed a gap issue 366 had left open and could not cheaply close.** That issue drained only
the future half of the beyond-horizon cohort (379 rows), because the full set needs
`current_published_at` per row and the selector for it is a scan; the PAST half was recorded there as
"a gap rather than a finding". This walk applies the horizon to every row in the table, so the past
half is now correct **by construction rather than by measurement** — worth stating precisely, because
its before-count was never taken and cannot be recovered.

**The cost was nothing like what was assumed.** The walk was expected to be heavy enough to want a
quiet window; 111 s for 7.9 M rows, with 838 GB free on `/data` and no warning in the journal. A
transient 503 on the first read straight after it ("a stalled internal wait … safe to retry")
resolved on retry. Recorded because the earlier reasoning on issue 366 treated "re-stamp the whole
tenders table" as expensive enough to shape a decision, and at this measured cost it is not — which
is worth knowing the next time that trade-off comes up.

### Still unobserved: `rederive-eur`'s stamped count

This run verifies the deadline half. The unit-3 successor — `rederive-eur` reporting changed tenders
and stamping them epoch-stale — has still not executed against real data, and that gap stands exactly
as recorded above. `backfill-deadlines` exercises none of that path.

## The live run happened (job 1001, 2026-09-10) — and it is a clean measured negative

The one thing left open above was that `rederive-eur`'s new successor had never executed against real
data. It has now, over the whole corpus:

```
rederive-eur ok, 646 s: eur_cents re-derived from 277,358 cached rates over 7,942,759 tenders:
0 of 267,401,093 money rows changed, 0 tender(s) stamped epoch-stale for the fold to re-elect
their head value (issue 375) — run `project` to apply
```

Three things this settles:

1. **The wiring works end to end.** The changed-tender count is computed, reported in the job's own
   completion message, and reached the stamping call. It was previously only tested on fixtures.
2. **The no-op case behaves.** Zero rows moved, so zero tenders were stamped — the corpus was NOT
   marked stale. That is the property `currency_rates.rs`' idempotence assertion pins in a unit test,
   confirmed here at 267 M rows. A walk that stamped on every run would have triggered a full re-fold
   each time and quietly reintroduced the cost this unit exists to avoid.
3. **It answers a question that prompted the run.** `fetch-rates` upserts ~220,629 rows every morning
   into a 277,358-row table — most of the history, daily, with REPLACE semantics. The obvious worry is
   that a revised historical rate would leave `eur_cents` silently stale corpus-wide with nothing
   re-deriving it. **It does not: zero of 267 M money rows changed.** The daily reload is idempotent
   in effect, and the derived layer is not drifting.

### Cost intuition was wrong AGAIN, in the same direction

The previous firing declined to run this because it "walks the whole corpus … would hold the queue
for hours". **It took 646 s.** That is the second time in two days a full-corpus walk was assumed
expensive and measured cheap — `backfill-deadlines` was the first, at 111 s for 7.9 M tenders after
the same reasoning.

Worth stating as a correction rather than a coincidence: **on this box, a single-pass walk over the
tenders table or the four money loci is minutes, not hours.** The expensive thing is the FOLD
(issue 179's 6 h 02 m for a 2.69 M-notice cohort), because it re-derives; a scan that reads and
compares is a different order entirely. Estimates on this issue and on 366 repeatedly conflated the
two, and both times the conflation argued against taking a measurement that turned out to be cheap.

