# 375 — two backfill jobs still compute the head columns with a raw extremum, so running either silently undoes issue 366's drain

Status: ready-for-agent (filed 2026-09-10 by the owner while landing 366 unit 3; found by reading
the third implementation rather than by a failure, and it has no test that can catch it)
Kind: defect (latent corpus regression — a runnable admin job that reverts a correctness fix)
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
