# 411 — issue 404's own adoption arm could not run: it DELETED the projected row, and `tender_versions` is a real foreign key

Status: needs-info — three of four `Done when` items met (built, red-then-green test, deployed `22f5bab`); the fourth, the live exercise, waits on issue 404's wet `repair-member-twins` run, the only population that can re-key on prod — and that run waits on an explicit go-ahead (the permission classifier refused it 2026-09-17). The signal is that job's outcome row; `## Verify` reads it. Was: ready-for-agent — **FIX DEPLOYED 2026-09-17** at rev `22f5bab`, health green (gated `GATE-EXIT=0`), found by reading 404's remaining unit rather than by an incident, and reproduced red before it was written. See "Repro" — the failure is `Constraint("immediate foreign key constraint failed")`, which `process` propagates with `?`.
Kind: defect (store — `Db::record_notice_tx`'s moved-identity arm, `crates/store/src/lib.rs`; self-inflicted, by the fix on issue 404)
Relates to: 404 (whose fix this is — the mint it stopped was real and the counter it added is right; only the adoption itself was wrong), 290 (`reparse_notice`'s adoption, which does it correctly and was the model 404 claimed to follow), 247 (the deferred FK checks `clear_parsed` needs and this arm did not set), 248 (the `keep` set that stops the expensive mention proof), 85 (`projected = 0` as the way a store-layer change reaches the canonical layer), ADR-0001 (the canonical layer is derived; the ingest path must not write it)
Blocked by: nothing

## The claim

404's fix made the ingest path adopt a moved `publication_id` by **writing the new row and deleting
the stale one**. It cannot do that. Every notice the arm exists for has already PROJECTED, and

    tender_versions.caused_by_notice_id INTEGER NOT NULL REFERENCES notices(id)
    tenders.island_notice_id            INTEGER          REFERENCES notices(id)

are real foreign keys under `PRAGMA foreign_keys = ON` (`crates/store/src/lib.rs:72`, on every
connection). The `DELETE FROM notices WHERE id = ?` therefore fails, `record_notice` returns `Err`,
and `process` propagates it:

    db.record_notice(&resolved_notice(source, fetch_id, now, n, &parse), &parse).await?;

So the next time any parser's identity derivation moves — a routine change on this project; issue 394
did it three weeks ago and issue 290 exists because of the same class — the daily ingest of that
source does not duplicate anything. It **stops**, on the first affected member.

## Why 404's own test did not catch it

`a_member_re_ingested_under_a_moved_identity_is_not_a_second_notice` ingests, re-processes, and
re-ingests under a moved key. It never folds in between, so its stale row has no version pointing at
it — the one property that every row in production has and that decides the outcome. The test pinned
the mint/match decision, which was the thing being fixed, and said nothing about the state the corpus
is actually in.

## Repro

Add a `tenders` row and a `tender_versions` row for the stale notice between the two ingests, which
is what the fold does within the hour:

    INSERT INTO tenders(id, source, kind, created_at, current_seq) VALUES(1,'ted','procedure',0,1);
    INSERT INTO tender_versions(tender_id, seq, caused_by_notice_id, published_at, publication_id)
         VALUES(1, 1, <stale>, 0, '00000000-1900');

then offer the same member under the new key:

    called `Result::unwrap()` on an `Err` value: Constraint("immediate foreign key constraint failed")

Pinned by `a_moved_identity_is_adopted_even_when_the_stale_notice_has_projected`.

## The fix: adopt IN PLACE, on the row that already exists

Exactly what `reparse_notice` has done since issue 290, and what 404's write-up said it was copying:

    UPDATE notices SET publication_id = ?, … , projected = 0 WHERE id = ?

The freshly minted row is deleted instead (it has no children — its parsed layer has not been
written yet), the surviving row keeps its id and takes the new key, its parsed layer is replaced, and
`projected = 0` hands it to the next fold.

**Preserving the id is not merely how the FK is dodged — it is the correct answer.** An island
Tender's group key is literally `island:<notice_id>`, so a preserved id keeps the notice in the
Tender it already folded into. A new id re-homes it, which is precisely the **3 cross-tender cases**
the 2026-09-16 duplication left standing (issue 404's own measurement): the re-keyed member minted a
fresh Tender beside the one it had always belonged to, and the old Tender — measured on prod, exactly
one version each — was left holding nothing else.

Two things the arm was also missing, both already solved on the re-parse path:

- **`PRAGMA defer_foreign_keys = ON`** (issue 247). `clear_parsed` deletes `organization_mentions`,
  which `tender_version_parties` references with no index serving the proof — measured at ~10 s per
  row on prod. The constraints are still checked, once, at COMMIT.
- **The `keep` set** (issue 248): the sections the new parse re-creates are kept, so their mentions
  survive and that proof never runs at all.

## Done when

- ~~The adoption preserves the row id and re-queues it~~ — built 2026-09-17.
- ~~A test folds the stale notice before the re-key~~ — `a_moved_identity_is_adopted_even_when_the_stale_notice_has_projected`,
  run red first (the FK constraint above), and asserting the surviving id, the new key,
  `projected = 0`, and that the Tender still holds the version this notice caused.
- ~~Deployed~~ — `22f5bab`, 2026-09-17 15:31 CEST, `/health` 200 and `rev` confirmed. The next
  ordinary weekday fold should report `0 re-keyed` with no `process` failure, as every fold since 404
  shipped has.
- **The live exercise is still owed.** Nothing has re-keyed on the ingest path since 404 shipped
  (job 1455 omitted the clause entirely), so neither the broken arm nor the fixed one has run on
  prod. The 281-row repair on issue 404 is the natural place to exercise it, because that cohort is
  the only population that can.

## The lesson, stated plainly

404's fix was reasoned about as a mint/match decision and tested as one. The row it deletes is not a
free-standing record — the corpus hangs off its id, by two foreign keys and by an island Tender's
group key that embeds it. "Copy `reparse_notice`" was the right instinct and the write-up said so;
what was actually copied was the DECISION (same member, same bytes, one notice) and not the
MECHANISM (update in place, re-queue the fold), and the mechanism was the part carrying the
constraints.

The same shape as 404 itself, one level up: looking at one path because the issue named one path.
Here it was looking at one LAYER because the fix lived in one layer.

## Verify

    ssh -o BatchMode=yes root@zebreus.click "/root/aj.sh '/admin/jobs?limit=60'" | python3 -c "import sys,json; r=[j for j in json.load(sys.stdin)['recent'] if j['kind']=='repair-member-twins' and 'dry' not in (j.get('params') or '')]; print([(j['job_id'], j['outcome'], (j.get('counts') or '')[:120]) for j in r] or 'no wet run yet')"

- **done**: one wet row, `ok`, whose counts line names 281 survivor(s) re-keyed and no `process` failure after it — the adoption arm ran on prod and held
- **open**: `no wet run yet` — only dry-run rows exist (read 2026-09-18)

A metadata read (the job table), free per `prod-box-reads.md`.
