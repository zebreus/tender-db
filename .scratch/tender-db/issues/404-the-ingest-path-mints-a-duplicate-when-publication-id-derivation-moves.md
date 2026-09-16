# 404 — the ordinary INGEST path has no identity-shift fallback, so a derivation change makes `process` MINT a duplicate notice instead of matching the one it already holds

Status: ready-for-agent — found 2026-09-16 09:40Z while verifying issue 394's re-key acceptance. **Self-inflicted and confirmed to the row**: my own 394 unit 1 guard, deployed 07:00Z, caused it, and the evidence is two notices holding the same bytes under two keys. At least **281** duplicate rows exist now and the count grows with every DÖE daily until this is fixed.
Kind: defect (ingest — the mint/match decision in the `process` path, `crates/ingest/src/process.rs` + `store::Db`'s notice insert; NOT the re-parse path, which was given the fallback by issue 290 and behaved correctly here)
Relates to: 290 (RESOLVED 2026-09-16 — it gave `reparse_notice` a `(source, content_hash)` fallback and made a derivation shift loud; this is the SAME hazard on the sibling path, which nobody looked at because 290 was scoped to the re-parse), 394 (the derivation change that triggered it, and whose acceptance this blocks — the placeholder cohort cannot reach 0 by re-parsing), 278 (the ghost-tender cleanup — the precedent for draining rows that projected before being found redundant), 21 (durable job rows and recovery, which is why the two jobs could overlap at all), ADR-0004 (the archive is the record; both rows point at the same archived member, so nothing is lost — what is wrong is that the corpus holds it twice)
Blocked by: nothing

## The evidence, two rows

    curl /v1/notices/27608916
      publication_id: 00000000-1900
      content_hash:   ed0c47f2a665ac052cde7d9df2ba73ad67953bdcfbc33ac9f1eaee8d50b44fae
      member_path:    3a3558a2-826c-4411-93ed-d6f88bc4226e-01.xml
      profile:        eforms:eforms-de-2.1
      ingested_at:    2026-08-07T07:38:21Z

    curl /v1/notices/45616483
      publication_id: 3a3558a2-826c-4411-93ed-d6f88bc4226e-01
      content_hash:   ed0c47f2a665ac052cde7d9df2ba73ad67953bdcfbc33ac9f1eaee8d50b44fae   <- identical
      member_path:    3a3558a2-826c-4411-93ed-d6f88bc4226e-01.xml                        <- identical
      profile:        eforms:eforms-de-2.1
      ingested_at:    2026-09-16T07:58:07Z                                                <- TODAY

Same archived member, same bytes, two notice rows, two identities. The second was minted an hour
after rev `6612b2b` (issue 394's guard) reached the box.

## The mechanism, end to end

1. **394's guard changed identity derivation.** `dispatch_eforms` now refuses the all-zero OJS
   placeholder, so a DÖE member that used to key on `00000000-1900` keys on its `<uuid>-<version>`
   stem. That was the point of the change.
2. **A re-parse campaign was re-keying the 7,177 standing carriers** through issue 290's fallback,
   in chunks, over about three hours.
3. **The daily `process doe daily (all)` ran in the middle of it** — job 2302 at ~07:58Z, walking
   44,835 members. For every member whose stored row the re-parse had NOT yet reached, the new
   derivation produced a key that matched nothing, and the ingest path's `INSERT OR IGNORE` on
   `(source, publication_id, content_hash)` therefore **minted a second row**. 1,138 DÖE notices
   were ingested that run; the ~281 orphans below are the part of that which is duplication.
4. **The later re-parse chunk then matched the TWIN.** `reparse_target` tries the full triple first;
   `(doe, 3a3558a2-…-01, ed0c47…)` now EXISTS, so it hit, replaced the twin's parsed layer, and
   returned `Replaced`. The placeholder row was never a candidate — the fallback only fires when the
   triple misses. Hence the last two chunks reporting **0 unmatched, 0 re-keyed** while 281 carriers
   stood.
5. **And the fallback would have refused them anyway, correctly.** Two `doe` notices now share that
   content hash, so `(source, content_hash)` names 2 rather than 1, and issue 290's uniqueness guard
   declines to adopt. That guard did exactly what it was built for. **The bug is the mint, not the
   refusal.**

The asymmetry is the whole issue: `reparse_notice` was taught that a moving `publication_id` means
"the same notice under a new name" (issue 290). The ingest path was not, and it is the path that runs
every day.

## Scope

| | |
| --- | --- |
| notices still carrying `00000000-1900` | **281** (from 7,177; 6,896 were correctly re-keyed) |
| their profile | all sampled are `eforms:eforms-de-2.1` |
| DÖE notices ingested 2026-09-16 | 1,138, ids 45,616,483–45,647,617 |
| `doe` notices total | 1,136,716 |

**281 is a floor, not the number.** It counts orphaned PLACEHOLDER rows; the duplication is the pair,
and a member re-walked after this issue lands would add more. The measurement this issue needs, and
which no bounded query on the box could produce (a `GROUP BY` on the unindexed `publication_id`
returned 408 twice, and per `docs/agents/prod-box-reads.md` a 408 is never retried): **how many
`(source, member_path, content_hash)` triples are held under more than one `publication_id`.** That
needs an id-windowed pass or an index, and it is the first unit.

## Why it matters

Nothing is lost — ADR-0004 holds, both rows point at the same archived member, and the newer row is
the CORRECT one. What is wrong is that the corpus asserts two notices where the publisher published
one, and both project. A duplicate notice becomes a duplicate tender version, which inflates
`current_seq`, emits a spurious change on the feed, and makes the version history of an affected
tender a record of this system's own accident rather than of the publisher's republications.

It also silently blocks 394: its acceptance is the placeholder cohort reaching **0**, and no amount
of re-parsing will get there, because every remaining carrier now has a twin that absorbs the match.

## Repro

1. `curl /v1/notices/27608916` and `curl /v1/notices/45616483` → identical `content_hash` and
   `member_path`, different `publication_id`, ingest timestamps five weeks apart.
2. `curl '/v1/notices?publication_id=00000000-1900&limit=200'` → 200 items, `more: true`, every one
   `eforms:eforms-de-2.1`.
3. `SELECT count(*) FROM notices WHERE publication_id = '00000000-1900'` → 281.
4. `/admin/jobs` → job 2320 `re-parsed 17826 notices across 40 packages (… 0 unmatched, 0 re-keyed …)`
   and 2322 `re-parsed 467 notices across 1 packages (… 0 unmatched, 0 re-keyed …)`: every DÖE
   package has now been walked and the cohort has not moved.

## Done when

- **The size is measured**, per the note above: triples held under more than one `publication_id`,
  by source. Everything below is scoped by that number.
- **The ingest path stops minting on a derivation shift.** Before inserting, a record whose
  `(source, member_path, content_hash)` is already held under a DIFFERENT `publication_id` is the
  same notice under a new name: update the key in place, the way `reparse_notice` adopts it, rather
  than inserting a second row. The same uniqueness discipline applies — refuse when the match is
  ambiguous, and say so in the job summary rather than silently.
- **The job summary says it happened.** Issue 290's lesson applied here: a derivation shift must be a
  NUMBER in the `process` summary (`re-keyed`, beside `dup`), not an invisible change in what the
  corpus holds. Today it was invisible — job 2302 reported `1138 notices (1138 parsed … 43697 dup)`
  with nothing distinguishing a genuinely new notice from a re-mint.
- **The standing duplicates are resolved**, and the choice is written down: delete the superseded
  placeholder rows (they are redundant — the bytes are held under the correct key) and repair any
  tender version that was folded from them, or keep them and mark them. Issue 278's ghost cleanup is
  the precedent for the first and should be re-read before executing, because rows that have
  PROJECTED cannot simply be deleted.
- **A sequencing rule is recorded in `docs/operations.md`**: a re-parse that changes identity
  derivation must not overlap the daily ingest, because the two race exactly as they did here. Either
  the campaign takes the queue, or the derivation change ships after the cohort is drained.
- A test pins the mint/match decision on a fixture: the same member bytes, offered twice with two
  different derived `publication_id`s, produce ONE notice row.
- 394's acceptance is restated in terms of this: the placeholder cohort reaching 0 is now a
  consequence of resolving the duplicates, not of re-parsing.

## Recorded because it is the honest version

This is my own change, deployed today, and the mechanism was visible in advance: when issue 290 was
resolved this morning I wrote on it that the reporting "covers the case where identity derivation
MOVES, and not the case where it DISAPPEARS", and reasoned about the re-parse path only. The ingest
path runs the same election every day and had no fallback at all — that is the gap, and looking at
one path because the issue named one path is how it was missed.

What worked: the acceptance was "the cohort must reach 0, not a small number", it did not, and that
refusal to round down is what exposed this within hours rather than at the next audit.

## The BLEEDING is stopped 2026-09-16 — the ingest path adopts a moved key

Status: the mint is fixed and gated (`GATE-EXIT=0`), not yet deployed. The 281 standing duplicates
are NOT yet resolved — that is the remaining unit.

### What changed

`record_notice` returns `Recorded::{Inserted, Duplicate, Rekeyed}` instead of a bool, and
`record_notice_tx` asks one new question: is the same `(source, member_path, content_hash)` already
held under a DIFFERENT `publication_id`? If exactly one row is, that row is this notice under a name
the parser no longer derives — so its parsed layer is cleared, the row removed, and the new one
written with its layer, all inside the existing transaction.

Three decisions worth reading:

- **Keyed on `(source, member_path, content_hash)`, not on the hash alone.** `member_path` is the
  physical location and `content_hash` is over the record's own payload, so together they name one
  thing the publisher published once. Matching on bytes alone would silently merge two packages that
  legitimately carry identical payloads, and the test has an arm for exactly that: same bytes,
  different member, still mints.
- **Exactly one, or nothing.** The same discipline `reparse_target` applies. Two rows sharing a member
  and bytes is a shape nobody has explained, and adopting one on a guess is worse than leaving a
  duplicate a census can find.
- **Asked AFTER the insert, not before.** The ordinary path — the overwhelming majority, where nothing
  moved — pays one indexed insert and nothing else. The lookup only runs for a row that was genuinely
  new, which on a normal day is the small tail.

`write_parse` was extracted so the ordinary insert and the adoption cannot drift into writing
different things for the same payload.

### It says so now

The `process` summary gains `N re-keyed` with the same NOTE clause issue 290 put on `reparse`.
The line that hid this read

    44835 members → 1138 notices (1138 parsed, 0 quarantined, 0 unrecognised, 43697 dup)

— an ordinary-looking day, with 281 of those 1,138 being duplicates of notices the corpus already
held. It would now carry `, 281 re-keyed — NOTE: this run CHANGED publication_id derivation`.

### Test

`a_member_re_ingested_under_a_moved_identity_is_not_a_second_notice` walks the whole sequence: ingest
under the old derivation, re-process (still `Duplicate`, so the idempotency the `dup` counter reports
is undisturbed), then the same member under the new derivation → `Rekeyed`, **`COUNT(*) = 1`**, the
surviving row carrying the new identity with its parsed layer, and a second offer now `Duplicate` —
the adoption is a one-time bridge. Plus the different-member-same-bytes arm.

Run red first: `left: Inserted, right: Rekeyed` — the mint, reproduced.

### Still owed

- **The 281 standing duplicates.** They have PROJECTED, so this is issue 278's shape, not a DELETE.
  The placeholder rows are the redundant half (their bytes are held under the correct key in the
  twin), so the repair is to drop them AND repair the tender versions folded from them.
- **The real size**, per the measurement note above — triples held under more than one
  `publication_id`, by source. 281 is the floor.
- ~~**The sequencing rule in `docs/operations.md`**~~ — recorded 2026-09-16 beside the issue-290
  re-key counters, where anyone sizing a re-parse already reads: either the campaign takes the queue,
  or the derivation change ships after the cohort is drained, and `/admin/jobs` is checked before the
  FIRST chunk (an idle queue now is not an idle queue in forty minutes, because the daily is
  scheduled).
- Live after deploy: the next DÖE daily reports `0 re-keyed` (nothing left to move) and the notice
  count does not jump.
