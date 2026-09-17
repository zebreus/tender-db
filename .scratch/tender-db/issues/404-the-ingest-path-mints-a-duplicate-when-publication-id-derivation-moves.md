# 404 — the ordinary INGEST path has no identity-shift fallback, so a derivation change makes `process` MINT a duplicate notice instead of matching the one it already holds

Status: ready-for-agent — **THE SIZE IS MEASURED CORPUS-WIDE 2026-09-17** (job 1466, 3 s over 14.5M rows): the corpus holds **exactly 281** members under two identities, all `doe`, all this cohort — 281 was not a floor after all. The mint is fixed (and its own fix was broken and re-fixed, issue 411). Only the repair of the 281 standing pairs remains, and every input it needs is now written down. Was: found 2026-09-16 09:40Z while verifying issue 394's re-key acceptance. **Self-inflicted and confirmed to the row**: my own 394 unit 1 guard, deployed 07:00Z, caused it, and the evidence is two notices holding the same bytes under two keys. At least **281** duplicate rows exist now and the count grows with every DÖE daily until this is fixed.
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

## Comment — 2026-09-17: the fix's own adoption arm could not run — issue 411

Before building the repair, I read the adoption arm this issue shipped, against the state the 281
rows are actually in. **It deletes the stale notice row, and the corpus hangs off that row's id.**
`tender_versions.caused_by_notice_id` and `tenders.island_notice_id` are foreign keys under
`PRAGMA foreign_keys = ON`, and all 281 have projected — so the delete fails with
`Constraint("immediate foreign key constraint failed")`, which `process` propagates with `?`.

The mint this issue stopped was real and the `re-keyed` counter is right. What was wrong is that the
arm copied `reparse_notice`'s DECISION and not its MECHANISM: the re-parse path adopts by
`UPDATE notices SET publication_id = …, projected = 0 WHERE id = ?`, in place. Filed and fixed as
**issue 411**, red-checked against a stale notice that has folded.

This also settles a question this issue left open. Preserving the notice id is not just how the FK is
avoided — an island Tender's group key is `island:<notice_id>`, so a preserved id keeps the notice in
the Tender it already belongs to. **A new id re-homes it, which is exactly the 3 cross-tender cases
below.** The in-place adoption would have produced none of them.

### And the 278/3 split has a SHAPE, not just a count

Re-measured on the idle box, one bounded read, grouping the cohort by whether the placeholder's
version is its Tender's head and how many versions that Tender holds:

    is_head  versions  count
    0        2          29
    0        3         194
    0        5          38
    0        6           8
    0        8           2
    0        9           4
    0       15           3
    1        1           3

The two populations are **exactly** these: 278 placeholders that are NOT the head of a Tender holding
2–15 versions, and 3 that ARE the head of a Tender holding exactly ONE version. The 3 are the same 3
named above — confirmed directly: tenders 1,745,988 / 7,939,911 / 7,964,960 each hold one version,
and so do their day-old siblings 8,555,909–911.

That is a far better dry-arm assertion than a bare 278/3: **a repair can refuse on the SHAPE** — no
placeholder may be the head of a multi-version Tender, and no singleton-Tender placeholder may be
anything but one of the three — rather than on a count that says nothing about which row is which.

### Which row to keep, decided

Keep the OLD (placeholder) notice and give it the correct key; delete the day-old sibling. Not the
other way round, which is what this issue's earlier sketch assumed:

- it is what issue 411's in-place adoption does, so the repair and the ingest path agree rather than
  each having their own idea of which row survives;
- the placeholder is the row the Tender already references, so for the 278 no version changes
  membership at all — only the sibling's extra version is removed;
- for the 3, keeping the placeholder keeps the LONG-LIVED Tender (created 2026-05 to 2026-08) and
  retires the one created 2026-09-16. Served ids stay stable and the `removed` event lands on a
  Tender that existed for a day, rather than on one consumers may have been citing for months.

### The pairing is a function, and the sibling's headship splits three ways

Two more bounded reads, so the repair's dry arm can assert shape rather than trust it:

    sibling count per placeholder:   1 → 281        (exactly one, 281 of 281)

    sib_head  same_tender  count
    0         1            249     sibling is a middle version of the SAME Tender
    1         1             29     sibling is the HEAD of the same Tender
    1         0              3     sibling is the head of its OWN one-version Tender

So deleting the sibling is three distinct amounts of churn, all of them correct and all of them
handled by re-derivation rather than by a rule:

- **249** — a middle version disappears; the head is untouched.
- **29** — the head disappears; `current_seq` and the `current_*` columns drop back to the
  placeholder's version. The served content does not change, because the two rows hold the SAME
  BYTES — what changes is that the corpus stops asserting a republication that never happened.
- **3** — the sibling's whole Tender is left with no versions and must be retired, with its `removed`
  events, by `retire_tenders_chunked`. It is one day old.

### And the split is not arbitrary — it IS the keyed/island distinction

    shape   size        count
    island  singleton     3
    keyed   multi       278

Complete, and derivable rather than observed:

- **278** sit on **keyed** Tenders. Both notices carry the same `procedure_key`, so the re-keyed twin
  folded into the Tender that was already there — as a second version of it.
- **3** sit on **island** Tenders, which have no key: their group key is literally
  `island:<notice_id>`. A new notice id is a new group, so the twin could not land anywhere but a
  fresh Tender.

So the class boundary is the group key, not a property of the data, and the dry arm should assert
exactly that: **every placeholder on a keyed Tender shares it with its sibling; every placeholder on
an island Tender is the only version of it.** A cohort that violates that has moved since this
measurement, and the plan is stale.

It also closes issue 411's argument from the other end. The 3 exist *because* the ingest path minted
a new id; an in-place adoption keeps `island:<notice_id>` pointing at the same group and produces
none of them.

### The "real size" bullet, first slice: no unexplained duplicates in DÖE

The cohort is 281 **placeholders**; the duplication is the PAIR, and the issue asked for the honest
number — triples held under more than one `publication_id`. That is now cheap to ask, because the
`notices_member_identity(source, member_path)` index this issue shipped serves it, and it is safe to
ask because the slice key is the group key (issue 278's argument): no group can straddle a boundary.

One bounded slice, DÖE member paths `0…1` — a sixteenth of the uuid hex space:

    duplicate member_path groups        17
    rows in them                        34
    groups holding a placeholder        17
    groups holding NO placeholder        0

So within that slice the duplicates are **exactly** the known cohort and nothing else, and 17 × 16
≈ 272 extrapolates onto the 281 pairs. Nothing in DÖE is duplicated for some other reason.

A corpus-wide answer still belongs in the job rather than in a probe — the other sources are much
larger and want the same slicing discipline. Noted while sizing it: the obvious form of this question,
`SELECT source, COUNT(*), MIN(member_path), MAX(member_path) FROM notices GROUP BY source`, is
UNBOUNDED and returned a **408** on the first try. Per `docs/agents/prod-box-reads.md` it was not
retried.

### The census is built, and it costs about two seconds

`member-twin-census` (`Spec::MemberTwinCensus`, read-only, stoppable, stores no report when
stopped) walks `notices_member_identity` one source at a time and reports, per source, how many
member names are held more than once and how many of those are ONE payload under two or more
`publication_id`s.

**The first shape written for it would have been quadratic**, and reading the plan rather than
assuming it is the only reason that was caught before it ran. The obvious corpus-wide keyset —
`source > ?1 OR (source = ?1 AND (member_path > ?2 OR …))` — plans as

    SCAN notices USING COVERING INDEX notices_member_identity

with the whole predicate applied as a FILTER, so every batch restarts at the head of the index: 144
batches over 14.4M rows would examine a billion of them. Seeking needs an EQUALITY on the leading
column, so the walk takes one source at a time (`source = ?1 AND member_path >= ?2`, which plans as
`SEARCH … (source=? AND member_path>=?)`) and steps between sources with `source > ?1 … LIMIT 1`,
itself a seek. Both are pinned by a plan test anchored on the seek TERMS, because this issue already
taught that a bare `contains` matches `notices_member_identity_DISABLED`.

Measured through `/v1/sql` on the live box, same statement shape as the job's batch:

    LIMIT 1,000        1.96 s      LIMIT 200,000      1.84 s
    LIMIT 10,000       1.68 s      LIMIT 500,000      1.70 s
    LIMIT 50,000       1.69 s      LIMIT 2,000,000    1.98 s

Two million rows cost the same wall time as one thousand, so ~1.7 s of that is ssh and HTTP and the
query itself is **about 0.3 s per 2M rows** — roughly **2 s for the whole 14.4M-row corpus**. It also
settles a question the plan text cannot answer: turso prints the same `SEARCH … USING INDEX` string
whether or not a seek is covering, but 2M rowid lookups could not complete in 0.3 s, so the walk is
served from the index and never touches a page of `notices`.

## Comment — 2026-09-17: the size is measured CORPUS-WIDE, and 281 was not a floor — it is the number

`member-twin-census` deployed at rev `df9fd64` and run as job 1466. **Three seconds**, against the
~2 s predicted from the `/v1/sql` timings:

    14,508,563 rows walked across 3 sources in 147 batches of 100,000
    stalled_batches 0

| | duplicate member names | twin sets | twin rows |
| --- | --- | --- | --- |
| `doe` | 281 | **281** | **562** |
| `ted` | 107 | **0** | 0 |
| `fts` | 0 | 0 | 0 |
| **total** | 388 | **281** | **562** |

**This unit's first bullet is closed.** The issue said *"281 is a floor, not the number"* — it counted
orphaned PLACEHOLDER rows of one DÖE incident and could say nothing about the rest of the corpus.
Walked end to end, the corpus holds **exactly 281** members under more than one `publication_id`,
every one of them `doe`, every one of them the 2026-09-16 cohort, pairing 1:1 into 562 rows. Nothing
else in 14.5M notices has this shape.

**And the two-stage design earned itself on the first run.** TED holds 107 member names twice — a
one-stage census keyed on the name alone would have reported 388 "duplicates" and sent a destructive
sweep at 107 rows that are not duplicates at all: the same member name over DIFFERENT bytes is two
records, which is exactly what the second stage tests and what the fixture arm
`the_same_name_over_other_bytes_is_not` pins.

**The report carries the whole cohort.** 281 sets is under the 400 listing cap, so
`sample_truncated` is `false` and every pair is named in the stored report, with both notice ids and
both keys, lowest id first. The repair's dry arm has something exact to be checked against rather
than a count to be trusted.

### What is left on this issue

Only the repair itself. Everything it needs is now measured and written down:

- the cohort is **exactly** these 281 pairs, corpus-wide, and the report names them;
- the split is **278 keyed / 3 island**, and that boundary is the group key, not a property of the
  data;
- the row to KEEP is the older one, re-keyed in place — which is also what the ingest path now does
  (issue 411), so the repair and the daily agree on which row survives;
- the churn is **249 middle versions / 29 heads / 3 whole Tenders**, all of it handled by
  re-derivation plus `retire_tenders_chunked` for the three;
- and the head columns of the 29 do not move, because the twins hold the SAME BYTES — only
  `current_seq` does.

## Comment — 2026-09-17: why no "delete a projected notice" path exists, and what the repair must therefore do

Before writing the repair, I inventoried what actually references a notice. **Seventeen foreign keys,
across fourteen tables:**

| handled by | tables |
| --- | --- |
| `clear_parsed` | `notice_sections`, `notice_texts`, `notice_codes`, `notice_classifications`, `notice_amounts`, `notice_dates`, `notice_integers`, `notice_numbers`, `notice_ids`, `organization_mentions` (+ `tender_version_parties` / `tender_version_bid_parties` by `mention_notice_id`) |
| **nothing** | `quarantine`, `tenders.island_notice_id`, `tender_versions.caused_by_notice_id`, `lot_results`, `bids`, `contracts`, `legacy_adjacency` |

`clear_parsed` says so itself, and deliberately: *"`lot_results`, `bids` and `contracts` also
reference this notice … so nothing breaks by leaving them for the fold to replace."* True for a
re-parse, which keeps the row. **Not true for a delete.**

And the four canonical tables in the second row — `lot_results`, `bids`, `contracts`,
`legacy_adjacency` — **carry no index at all.** Not on `notice_id`, not on `tender_id`, none. So
deleting one notice row makes SQLite prove the constraint by scanning four multi-million-row tables,
and deleting their rows by `notice_id` is itself a full scan per statement. At 281 notices that is
issue 247's ~10 s-per-row shape multiplied by four tables.

**That is why this path does not exist anywhere in the codebase**, and the only `DELETE FROM notices`
outside tests is issue 411's — which now removes a row that has no children by construction.

### So the repair batches by TABLE, not by notice

- Bracket the whole pass with `set_foreign_keys(false)`, restored after — `project.rs` already does
  exactly this for the R2/E0/R3 merge loop (issue 352), for exactly this cost.
- Delete the child rows explicitly, **one statement per table over all 281 ids** (chunked into `IN`
  lists), so an unindexed table is scanned a handful of times in total rather than 281 times.
- The three island Tenders go through `retire_tenders_chunked`, which already deletes every
  `tender_id`-keyed satellite and emits the `removed` events.
- Then the keepers: `publication_id` = the elected key, `projected = 0`.

Deferring the FK checks is NOT an alternative here: a deferred constraint is still proved at COMMIT,
so it moves the scan rather than removing it.

**Not yet measured:** how many rows the 281 siblings actually hold in those four tables. It does not
change the design — a batched DELETE costs the same whether it matches 0 rows or 500 — but it is
worth knowing, and it cannot be asked cheaply for the same reason the delete is expensive. The dry
arm is the right place to count it, once, under the same bracket.
