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

Status: the mint is fixed and gated (`GATE-EXIT=0`), **DEPLOYED** — verified present in the
running build 2026-09-17 (`Recorded::Rekeyed` in `crates/store/src/lib.rs`, `report.rekeyed` in
`crates/ingest/src/process.rs`, serving rev `7726bcb`). **LIVE ACCEPTANCE PASSED 2026-09-17** on the first ordinary weekday fold since the deploy — twice
over, on both halves of what 404 broke. See the comment at the foot. The 281 standing duplicates
are NOT yet resolved — that is the only remaining unit.

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

## 2026-09-16 — the fix was O(corpus) per new notice, and it is a severe regression

Status: REGRESSION from this issue's own fix, found the same day by running a backfill into it, and
fixed in the same sitting. Caught **before** the next daily ingest hit it.

### What happened

The 402 backfill fetched OJ S `2026-00135` (3,550 members, all genuinely new) and enqueued a
`process`. Two hours later it had done **896 of 3,550 members** — about **7.5 members per minute**,
i.e. 0.12 notices/s. The same shape ran at **30 notices/s** the morning before (job 2300: 145,386
members walked, 3,534 new notices, **119 s**; job 2270: 3,647 new in 109 s).

### Why

`moved_identity_twin` — added by this issue's fix, deployed 09:58Z the same morning — runs

    SELECT id FROM notices
     WHERE source = ? AND member_path = ? AND content_hash = ? AND publication_id <> ? LIMIT 2

and **nothing indexes `member_path`.** `notices` has UNIQUE(source, publication_id, content_hash) and
indexes on profile, parse_state and fetch_id; none can serve this. So the lookup full-scans a
14.4M-row table.

Measured on the box, the exact query shape against a real member:

    ~9 s per call.

3,550 new notices × 9 s ≈ 9 hours, which is precisely the observed rate. The daily ingest adds ~3,500
new notices every morning, so tomorrow's 07:00Z run would have taken the same nine hours instead of
two minutes — and a 12-issue backfill would have taken four days.

### The reasoning that produced it, quoted so it is not repeated

From this issue's own write-up, hours earlier:

> **Asked AFTER the insert, not before.** The ordinary path — the overwhelming majority, where
> nothing moved — pays one indexed insert and nothing else. The lookup only runs for a row that was
> genuinely new, which on a normal day is the small tail.

Wrong twice in one sentence. The lookup was **not indexed** — I wrote "one indexed insert and nothing
else" about the insert and never asked what the lookup cost. And genuinely-new rows are **not a small
tail**: they are exactly what an ingest exists to add. The clause is true only of a re-process, which
is the shape I had in my head from the 394 re-key campaign.

### The fix

    CREATE INDEX IF NOT EXISTS notices_member_identity ON notices(source, member_path);

in the schema batch beside `notices_fetch_id`, which has the same "first open after this change
builds it once" property; `deploy.sh`'s health check already waits minutes for exactly that, and says
so in a comment. `(source, member_path)` rather than the full triple: a member path names one thing,
so the seek returns one or two rows and the hash filters them — indexing the 64-char hash as well
would double the index for nothing.

Pinned by `the_moved_identity_twin_lookup_seeks_by_member_rather_than_scanning`, an EXPLAIN QUERY
PLAN test: a plan test rather than a timing test, because the failure is a SCAN and a SCAN shows up
on an empty table in milliseconds while a timing test would need the whole corpus to show anything.

**The first red check of that test passed when it should have failed** — renaming the index to
`notices_member_identity_DISABLED` still satisfied `plan.contains("USING INDEX notices_member_identity")`,
because a prefix matches. The assertion now anchors on the seek TERMS
(`notices_member_identity (source=? AND member_path=?)`), which only an index on those two columns
produces, and the real red check deleted the index outright.

### What this costs

One index over 14.4M rows on a short text column, built once at the first open after deploy. Disk on
the box is 1.2 T used of 1.7 T, 527 G free, so this is affordable — but it IS the kind of standing
cost issue 169 tracks, and it is here because the ingest path now asks a question the schema was
never shaped for.


## Comment — 2026-09-17: live acceptance passed on both halves

The first ordinary weekday fold since the deploy (jobs 1452–1460, serving rev `7726bcb`). Both of
404's failure modes were visible in it, and both are clean.

**1. The mint no longer doubles.** Job 1455:

    doe daily (all): 46019 members → 1184 notices (1184 parsed, 0 quarantined, 0 unrecognised, 44835 dup)

**No `re-keyed` clause**, which is the signal — the summary omits it entirely at zero. Compare the
regression this issue was filed on, quoted in the code comment that now guards it:
`44835 members → 1138 notices (1138 parsed … 43697 dup)`, an ordinary-looking day during which 281 of
those 1,138 were duplicates of notices already held. Same shape, same scale, and today nothing was
re-keyed and nothing was doubled.

**2. The index regression stays fixed.** The other half of 404 was the `notices(source, member_path)`
index whose absence made every genuinely-new notice full-scan 14.4M rows — 9 s per lookup, a TED daily
projected at 9 HOURS, 0.12 notices/s against 30/s. Issue 407's per-package line prices it directly now:

    [process] ted daily 2026-00180: 3424 members → 3424 notices (0 dup) in 25.1s (136.4 members/s)

25 seconds for a full genuinely-new daily, consistent with the 155.7 members/s measured right after
the index shipped. The gap between that and 0.12 notices/s is the whole of the regression, and it is
gone.

**What remains** is the 281 standing duplicates. The bleeding is stopped and measured; the stock is
not drained. Issue 278's ghost cleanup is the precedent to re-read first, because rows that have
PROJECTED cannot simply be deleted.

## Comment — 2026-09-17: the 281 are measured, and they are not one population

The remaining unit's first bullet — *"the size is measured … everything below is scoped by that
number"* — done, on an idle box, four bounded reads through `/v1/sql`. The cohort has **not moved**,
which is itself the expected result now that the bleeding is stopped:

    SELECT COUNT(*) FROM notices
     WHERE source='doe' AND publication_id='00000000-1900'          → 281

**Every one is genuinely redundant.** Each has a sibling holding the same archived member under a
different key — the 404 shape exactly:

    … JOIN notices b ON b.source=a.source AND b.member_path=a.member_path
                    AND b.publication_id <> a.publication_id        → 281 of 281

**And every one has PROJECTED**, so issue 278's precedent binds and a bare `DELETE` is off the table:

    … JOIN tender_versions v ON v.caused_by_notice_id = a.id        → 281 of 281

**But they are two populations, not one**, and this is the part that decides the repair:

| class | count | shape |
| --- | --- | --- |
| placeholder and sibling on the **same** tender | **278** | a spurious extra VERSION on a tender that already holds the real one |
| placeholder and sibling on **different** tenders | **3** | the placeholder sits on an old tender; the sibling minted a new one |

The three, named so they cannot be rounded away:

| placeholder notice | its tender | sibling notice | sibling key | sibling tender |
| --- | --- | --- | --- | --- |
| 31145658 | 7,964,960 | 45642425 | `06b602e1-5053-442b-b5ad-3371e3ed4ac2-01` | 8,555,911 |
| 29332209 | 7,939,911 | 45632756 | `71fa2d15-c6cb-479b-b7b9-66c5a529692e-01` | 8,555,910 |
| 28130872 | 1,745,988 | 45623699 | `9fbf38c9-f0e2-44d5-83bf-bc991033ca6e-01` | 8,555,909 |

The sibling tenders are **consecutive** (8,555,909–911) — minted together during the 394 re-key
campaign — while the placeholder tenders are scattered across the corpus (1.7M, 7.9M, 7.96M). So for
these three the placeholder is the OLD identity on an OLD tender and the re-key minted a fresh tender
beside it, rather than adding a version to the existing one. Removing the placeholder version there
could leave an old tender with one fewer version, or none.

### What this means for the cleanup

The 278 are the easy, uniform case and the 3 are not, so **a single sweep that assumes one shape
would quietly do the wrong thing to three tenders** — which is the same failure as issue 278's, where
rows that had projected could not simply be dropped. Any repair job needs:

- a dry arm that reports the two classes separately and refuses if the split is not 278/3, because a
  changed split means the cohort moved and the plan is stale;
- the 3 handled explicitly, by decision rather than by the same rule;
- the projection re-run afterwards, with 394's acceptance (the placeholder cohort reaching **0**) as
  the check — restated on this issue precisely because reaching 0 is now a consequence of resolving
  these rows, not of re-parsing.

Not executed here: `/v1/sql` is SELECT-only by design, so this belongs in a job with dry/wet arms,
and the measurement above is what that job's dry arm has to reproduce before anything is written.

## Comment — 2026-09-17 (later): the repair is existing machinery, not surgery

Read the store layer rather than assuming the cleanup needs bespoke repair code. It does not, and
that materially de-risks the remaining unit.

**The notices → tenders step already exists.** `Db::tenders_for_notice_ids` resolves a notice set to
the Tenders those notices fold into, via exactly the `caused_by_notice_id` lookup this cohort needs,
and its doc says the walk "pulls each hop's tender memberships so those Tenders **re-derive IN
FULL**". `Db::touched_existing_tender_ids` is the same lookup on the changed set. So identifying what
to rebuild after deleting the 281 is a call, not an algorithm.

**Targeted re-derivation exists twice over**, and either works here:

- **Re-queue by watermark** (issue 85): clear the `projected` watermark for the affected parsed
  notices so the next `project rebuild=false` "re-derives just those Tenders. The parsed layer is
  untouched." Batched by id range with a TRUNCATE between, because a cohort-wide UPDATE balloons the
  WAL-index (issue 63) — a trap already solved and documented.
- **Stamp epoch-stale** (issue 179): set `projection_epoch = 0` on the affected Tenders so the next
  fold's chain-compare keeps nothing for THEM and rewrites them under current logic, without bumping
  the global `PROJECTION_EPOCH`.

### So the repair reduces to

1. Select the cohort (`source='doe' AND publication_id='00000000-1900'`) — an indexed seek, already
   measured at exactly 281.
2. Resolve its Tenders with `tenders_for_notice_ids`, and assert the split is **278 same-tender /
   3 cross-tender** before writing anything; a different split means the cohort moved and the plan is
   stale.
3. Delete the 281 notices and their `tender_versions` rows.
4. Mark those Tenders for full re-derivation by one of the two mechanisms above.
5. `project rebuild=false`, then check 394's acceptance: the placeholder cohort reads **0**.

The only genuinely new code is the cohort selection, the delete, and the dry/wet plumbing — which is
the shape every `repair-*` job in `supervisor.rs` already has, including the guard I want here:
`Spec::RepairNoticeInstants` refuses a wet run unless a stored DRY report matches, and says so
("no stored … plan — run the dry pass first"). Copy that.

**The 3 cross-tender cases still need a decision, not a rule** — the placeholder sits on an old
Tender while the sibling minted a new one, so step 4 rebuilds an old Tender that may end up with one
fewer version, or none. Whether that old Tender should survive is the judgement the sweep must not
make on its own, and it is why step 2 asserts the split rather than trusting it.
