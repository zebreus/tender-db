# 138 — pre-registered verification criteria for the #29 execute re-spec

Status: **written before the re-spec exists**, deliberately. Owner: sdk-vendor (verification).
Relates to: 84/#29 (the population), 137 (the sizing), 136 (the precedent for pre-registering)

## Why this is written first

I am the verifier for the #29 execute re-spec. If I write the acceptance criteria *after* reading
the implementation, I will unconsciously write criteria the implementation satisfies — the same
reason `negative_money_407_triage.sql` carried its reading key before any number existed. Criteria
fitted to an artifact are not criteria; they are a description.

So these are fixed now, and any later change to them must be **argued and dated**, not quietly made.

## The six things I will check

### 1. `expect` is an EXACT number, and it is 592,856

Not a range, not a minimum, not "approximately". A count-correction whose expectation is a range
cannot detect the thing it exists to detect: if the population shifted between dry-run and execute,
that is exactly when it must refuse. **The dry-run measured 592,856 markable. The execute must
assert that same figure and abort on any deviation, in either direction.**

*Fails if:* `>=`, a tolerance band, or a recomputation at execute time that makes the assertion
vacuous (asserting a number against itself is check #3's problem in a new costume).

### 2. `skipped_at` is used — NEVER `reprocessed_at`

The schema says this explicitly at `store/src/lib.rs:158`:

> *Distinct from `reprocessed_at`, which means RECLAIMED: using that column here would claim ~593k
> notices entered the corpus that never did.*

**Marking 592,856 rows with `reprocessed_at` would fabricate the largest single false recovery claim
in the project's history.** I nearly conflated these two columns myself while measuring issue 137 —
in a read-only query, where the cost was a wrong sentence rather than 593k wrong rows.

*Fails if:* the execute writes `reprocessed_at`, or writes both, or leaves the outcome inferable
only from `reason`.

### 3. The 154 are tracked as their OWN finding and cannot be absorbed

The 154 (originals held, did not parse) are **not** part of the duplicate population and must not
be marked skipped. They need their own record, their own count, and their own resolution path.

*Fails if:* the predicate is widened to include them; if they are marked skipped "for tidiness"; if
they appear only as `592,856 + 154 = 593,010` in a summary; or if the number 154 appears nowhere in
the post-execute state.

**And the framing must survive:** they are **held-but-unextracted**, not lost. team-lead corrected
this already. If the re-spec's own text calls them data loss, that is a defect in the artifact, not
a wording preference — "loss" is the reasoning shape that justifies keeping backups forever.

### 4. The ledger carries BOTH populations, separately, in the user-facing surface

Per the resolved-categories rule, landing this requires a `quarantine-ledger.json` entry. It must
show the duplicate population **and** the 154 as distinct lines.

*Fails if:* the dashboard's Resolved section shows one number; if the 154 are a footnote; or if
"resolved" is claimed for the aggregate.

### 5. The guard must still be able to fire after the change

The sibling guard **rejected 154 and passed 592,856** — a non-trivial split, which is what
demonstrated discrimination rather than mere firing. After the re-spec, that same guard must still
be capable of rejecting.

*Fails if:* the re-spec widens the predicate such that the guard's reject arm becomes unreachable.
**A guard that cannot fail is not a guard**, and the fastest way to make a red gate green is to move
the bar rather than the data. I will ask for a demonstration that it still rejects something —
synthetic is fine, absent is not.

### 6. The dashboard's three-way split lands with it, or is explicitly deferred with a date

Issue 137 measured that the dashboard counts **reclaimed** rows inside a "quarantined" total —
1,734,594 of 2,419,410 already done, and `unknown-field-code` showing as a 577K gap when 10 remain.
That is the same honesty-defect family this execute exists to fix.

*Fails if:* the execute lands and the dashboard still presents a two-way (or one-way) count, with no
dated follow-up. Fixing the count while leaving its presentation wrong resolves the arithmetic and
not the misinformation.

## What I will NOT hold it to

* The 8 unmatched rows from issue 136 and `eforms-sdk-1.2`'s 3 rows are unrelated; not this change's job.
* Performance. The execute is a bounded UPDATE over an indexed column (`quarantine_notice_id`
  exists precisely for this, per issue 80); if it is slow that is a separate finding.
* Reversibility beyond the ordinary. `skipped_at`/`skipped_reason` are additive columns; a wrong
  mark is correctable by nulling them. I will check that no row's `reason` or `content_hash` is
  rewritten, since those are the identity and would not be recoverable.

## Post-execute assertion I will run myself

Independent of whatever the execute reports about itself — a job's self-report is not verification:

```sql
SELECT COUNT(*)                                     AS dtd_rows,
       SUM(skipped_at IS NOT NULL)                  AS marked_skipped,
       SUM(reprocessed_at IS NOT NULL)              AS wrongly_reclaimed,
       SUM(skipped_at IS NULL AND reprocessed_at IS NULL) AS still_outstanding
  FROM quarantine
 WHERE reason = 'unparsable-xml' AND detail LIKE 'XML with DTD detected%';
```

**Expected: `marked_skipped = 592,856`, `wrongly_reclaimed = 0`, and `still_outstanding` accounting
for the 154 plus the ~1,905 non-markable remainder** (594,915 measured outstanding − 592,856 −
154 = 1,905, which itself needs an explanation before I sign off; an unexplained remainder is a
finding, not a rounding error).
