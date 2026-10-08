# ADR-0017 — A re-derivation compares before it writes; the feed announces what differs from what was stored

Status: PROPOSED 2026-10-08 (issue 495 unit 1; drafted from three competing designs and a judge,
`wf_21089c8e-de5`). D6 and D7 apply now as runbook guidance: they add a route, and nothing a consumer sees
changes. D1–D5 change what `/v1/changes` and SSE emit after a refold. They become ACCEPTED, and take effect,
with issue 495 unit 4 (the flip), after unit 3's shadow measurement, unless Lennart objects first. Until then
the fold rewrites stale Tenders in full and replays their history as before.

## Context

An epoch-stale Tender is rewritten in full. `apply_tender_tx` forces `keep = 0`
for it. It then deletes every stored version (`delete_version`: 14 re-parsed
DELETEs per version) and re-inserts every version. `append_version_changes`
re-emits the whole history under new cursors.

The last all-profile refold, job 2044 (epoch 3, 2026-10-07), measured:

| | |
|---|---|
| Fold | 23,674 s (6 h 35 m) of an 11 h 06 m job |
| Tenders / versions | 8,780,780 / 14,896,923 |
| Leaf rows deleted and re-inserted | 1,117,908,878 |
| Change rows appended | 70,643,598 |

More than 95 % of the rewritten rows came back byte-identical.

- The single writer thread is the bottleneck: it is on the CPU 84.5 % of the time.
- The filesystem is not the bottleneck. Issue 488's defrag made the same fold at
  most 4.6 % faster.
- One week saw three all-profile refolds (479, 484/489, 490): about 20 h of folding
  and about 210 M change rows, all of them permanent.

**The feed's behaviour on a refold was inherited, never decided.**

- The replay is the only way a consumer learns what a refold corrected.
  `append_version_changes` compares new versions with each other, never with what
  was stored. If the fold simply skipped unchanged rows, a 484-type fix
  (`is_buyer`) would never reach `/v1/changes` or SSE, and nothing would report an
  error.
- The docs describe the replay without deciding it:
  - docs/operations.md records it as "the accepted issue-179 cost";
  - the `PROJECTION_EPOCH` doc calls the noise "permanent";
  - docs/architecture.md says re-projection "never rewrites history", which
    `keep = 0` contradicts.
- Seq-less `changed` rows already exist. The org merges write them (issue 286), and
  SSE probes the current head for them (issue 287).
- The shape itself is undocumented: the published event schema says `version` is
  "null for notices" (docs.rs, openapi.json).

**The routing was never written down either.**

- 490's lot value could have been derived from stored rows alone. REST already
  computed it at read time; 6,297 of 6,297 sampled lots agreed.
- Issue 340's `backfill-original-lang` showed the precedent: 7,924,745 Tenders in
  70 min.
- The epoch-4 refold was chosen because the epoch doubles as the completeness
  marker. That is a reason, but one the runbook should weigh against 11 hours.

## Decision

### D1 — Stale versions are re-derived and compared with the stored rows, at version × table grain

For an epoch-stale Tender, the fold computes `chain_keep`, the common prefix of
the stored causing notices and the new chain. Today that is forced to 0.

Versions past the prefix are written exactly as today: deleted, written, and
announced with transition rows.

Versions inside the prefix take the compare path:

1. **Re-derive.** The version is re-derived by the same `write_version` call the
   full rewrite uses, into a per-version scratch buffer. Every input is the same as
   in a full rewrite:
   - the `ScalePartners` running state, because every version is fed in order;
   - the `EurContext`;
   - the identity lookups, run in the same order.
   The scratch rows are therefore exactly what a full rewrite would insert.
2. **Read the stored rows.** For each of the 14 version-keyed tables, the writer
   runs a prepared `SELECT rowid, <the INSERT's columns> … WHERE tender_id = ? AND
   seq = ?`.
   - It runs inside the batch's `BEGIN IMMEDIATE`.
   - The result is drained before any write on the connection, then sorted by
     rowid.
3. **Compare.** Equality is positional in rowid order.
   - Row counts are compared first, then every value, using Rust equality on
     `turso::Value`: `Null == Null`, integers exact, text bytewise,
     `Integer != Real`. Reals compare by bit pattern, though no leaf column is
     REAL.
   - Order counts as part of equality, because `PARTIES_SQL`, `BID_PARTIES_SQL`
     and the result-stats read have no ORDER BY. They serve rows in
     `(tender_id, seq, rowid)` order.
4. **Rewrite only what differs.**
   - A differing `tender_versions` row rewrites the whole version.
   - Otherwise, each differing table is deleted for that `(tender_id, seq)` and
     re-inserted whole from the scratch buffer.
   - Identical tables are not touched.
   - A table is never partly rewritten, so relative row order always equals a full
     rewrite's.
5. **Compare the head.** The head columns are read on the identity row:
   `current_seq`, `current_published_at`, `current_deadline`, `current_title`,
   `current_value_eur_cents`.
   - If they are equal, only `projection_epoch` is updated. turso maintains every
     index over a SET column even when the value does not change.
   - If they differ, today's head UPDATE runs.
6. **Keep the markers and the sweep.**
   - `projection_epoch` is stamped on every stale Tender considered, so it stays
     the completeness marker.
   - The orphan sweep (issue 103) runs for every stale Tender.
   - `tender_currency_presence` is always unioned.

One descriptor, `LEAF_TABLES`, generates the INSERT prefixes, the compare
SELECTs, the DELETEs and the retire and reset lists. A `PRAGMA table_info` test
pins it to the schema, so no column the fold writes can escape the compare.

Unchanged paths:

- Non-stale Tenders.
- `rebuild = true`.
- Versions past the common prefix.

Two settings exist besides the default:

- `TENDER_REFOLD_COMPARE=off` restores `keep = 0` exactly. It stays as a kill
  switch for at least one release.
- `shadow` compares and counts, but still rewrites.

Which Tenders are stale does not change. A Tender stamped `0` by a scoped refold,
a re-parse, a rebind or a rederive is compared, and so is one with an older epoch.
The compare reads the current stored state, so it is correct whatever wrote that
state: org merges, re-parse party deletes, in-place backfills, or a binary that was
rolled back.

### D2 — Two kinds of change row

- **Transition** (`version_seq = N`): what version N changed against N−1. These
  are written only when a version is first written or re-sequenced. This is
  today's `append_version_changes`, unchanged.
- **Correction** (`version_seq` NULL, op `changed`): the stored state of an
  existing entity was re-derived and differed from what was stored.

Swept entities keep their seq-less `removed` rows, as today. No schema change is
needed: `changes.version_seq` is already nullable.

### D3 — A correction is emitted if and only if the re-derivation differs from what was stored

For each stale Tender, after its compare:

- **Nothing differs.** That means every compared table, the head tuple, `source`
  and `kind`, and no version removed. No change row is written; the epoch is
  restamped.
- **Something differs.** Two rules apply:
  - **Rule T:** one `('tender', id, NULL, 'changed')`.
  - **Rule L:** one `('lot', id, NULL, 'changed')` for:
    - every lot of the head version, whenever anything at the head differs (any
      table of the head version, the head tuple, `source` or `kind`) — because
      lots inherit the Tender's version predicates, and `?min_value=` on
      `/v1/lots` reads `tenders.current_value_eur_cents`;
    - every lot whose own rows differ at any seq;
    - every lot the re-derivation minted.
    Swept lots are excluded, because they get `removed`. An implementation may
    over-deliver, up to every surviving lot of the Tender. It must never deliver
    less.
- **No correction rows for `lot_result`, `bid` or `contract`.** They are not
  public kinds (issue 211) and resolve to no endpoint. Their `removed` rows from
  the sweep stay.
- **Placement.** Correction rows are written last for the Tender, in fold order,
  inside the batch transaction. They count toward `Applied.changes`, so the
  doorbell rings.

Versions past the common prefix still get today's transition rows. A stale Tender
that also gained a notice gets those, plus one correction set if its prefix
differed.

### D4 — The consumer contract

- `version: null` on a `tender` or `lot` event means: existing versions of this
  entity were rewritten in place.
  - Re-read the entity by id, including its history if you mirror versions.
  - Upsert, because you may not hold it yet.
  - `changed` is an upsert too.
  This has been true for org merges since issue 286; this ADR documents it.
- SSE classifies an in-place row at the current head against head−1 (issue 287).
  - A subscriber may see `added` or `removed` for an entity whose filter membership
    the correction moved.
  - When neither side matches, it may see an over-delivered `removed`.
  - Both are idempotent for a client.
- History is announced once. Transition rows are as of their emission. A refold no
  longer re-diffs old versions under new logic. A consumer that wants every version
  re-announced after a logic change uses the generation and reset path (issue 46).

### D5 — In-place backfills announce through the same rows

- A walk that changes a value REST already serves, or that an SSE filter reads,
  writes D3's correction rows for the moved entities. It writes them inside its
  window transaction.
- `rederive-eur` therefore stops being quiet.
- A new additive column moves nothing that is served, so it stays quiet and is
  announced in CHANGELOG.md. The same holds for a stored copy of a value REST
  already derived, which was 490's case.

### D6 — Routing: the cheapest re-derivation that is complete

Before any refold, the runbook asks these questions in order. The first yes wins.

- **R0 Deploy only.** The change is to the read path, serialization or
  performance.
- **R1 Requeue only.** The fix moves grouping or chain membership; the chain
  compare catches that.
- **R2 In-place backfill** (no fold, no planning). Only if all of the following
  hold:
  - **E1:** the value is a pure function of three inputs only: the stored
    canonical rows, reference tables such as `currency_rates`, and the causing
    notice's own value rows. No resolver, section or mention state. 484's
    `is_buyer` fails this test.
  - **E2:** there is one implementation. The walk calls the fold's own function,
    injected across the crate seam as 340's `normalize_lang` is. A fixture test
    pins fold(new) == fold(old) + backfill, byte for byte.
  - **E3:** no entity or version is minted or removed.
  - **E4:** every stored value derived from the moved one is re-derived by the
    walk, or its Tenders are stamped and requeued, as `rederive-eur` does.
  - **E5:** version N depends only on versions 1..=N.
  - **E6:** the fold writes the same value from the deploy onwards.

  Mechanics:
  - windows over the tenders primary key;
  - one `BEGIN IMMEDIATE` and one checkpoint per window;
  - a watermark in `projection_state`;
  - D5's correction rows.

  These qualified: 340, 306/375, 371, and 490's lot value. These do not: 484, 479,
  and the 98/174/177 mapping fixes.
- **R3 Scoped-stale refold** (requeue plus stamp 0, no epoch bump). Use it when
  the fold output changes for a cohort that can be enumerated. The tools:
  - `refold` by profile;
  - `refold-fields`;
  - `refold-sections`;
  - `refold-notices`;
  - tender ids.
  Every profile is allowed when no rollback-proof marker is needed, as with 479
  and 484.
- **R4 Epoch bump plus an all-profile refold.** Use it only when one of these
  holds:
  - no cohort finder can enumerate the affected Tenders; or
  - completeness must survive a rollback below the deploy, or a restore from a
    backup taken before the fix. The epoch is the only marker that lives in code.

After D1, R3 and R4 cost a verify, not a replay. Planning and grouping, about
4.5 h of an all-profile job, then take most of the time. Batch corpus-wide fixes
into one fold.

### D7 — Completeness

- **R4:** the epoch, read in bounded 100k-id windows, as in docs/operations.md
  today. Unchanged.
- **R3:**
  - The check is the stale-0 residue in the same windows, plus
    `verified + corrected == stamped` on the job's counts line.
  - A stamp over-reports and never under-reports.
  - It is not restore-proof or rollback-proof. After either, re-run the scoped
    refold; that is now mostly a verify.
- **R2:**
  - The watermark reaches `MAX(id)`, and a completion flag is set in
    `projection_state`, as `currency_presence_complete` is.
  - The read path gates on the flag.
  - After a rollback below the deploy, clear the flag and re-walk.

## Consequences

- **Fold cost.** A stale refold now costs a per-Tender verify, the stored-row
  reads, and a rewrite of only what differs. Estimated, not yet measured:

  | Term | Estimate |
  |---|---|
  | Reads | 1.1 B rows at 2.4–5 µs each |
  | Per-version seeks | 14.9 M versions × 13 prepared seeks |
  | Row building | paid today too |
  | Per-Tender floor | 0.15–0.28 ms |

  The read rate is anchored on job 1001, which read 267,401,093 money rows in
  646 s on one connection. That gives about 1.3–2.8 h for a no-change or
  484-type refold, and about 1.4–3.1 h for a 490-type one, against 6.6 h today.
  Issue 495 unit 3 measures it in shadow mode before the flip.
- **Change rows per refold:**

  | Refold | Change rows |
  |---|---|
  | Today | 70.6 M |
  | No logic change | 0 |
  | 484-type fix | thousands |
  | 490-type fix | about 10–16 M |

  WAL volume, rowid consumption and freelist churn all fall with the rewrite
  share.
- **Where the gain applies.** Nothing needs seeding, so every stale path gains:
  the first refold after the flip, the scoped cohorts (re-parse,
  refold-notices/sections/fields, rederive), and the daily fold when it meets a
  stale Tender.
- **Rollback below the flip is safe.** The old binary rewrites with `keep = 0` and
  replays, as before. There is no stored state to invalidate.
- **New counters.** `Applied` gains `tenders_verified`, `tenders_corrected`,
  `tables_skipped`, `tables_rewritten`, `rows_skipped`, `rows_rewritten` and
  `correction_rows`.
  - `leaf_rows` keeps its meaning of rows flushed.
  - `tenders_written` keeps its meaning of "the write path ran".
  - The heartbeat and the counts line print the new counters.
- **`/v1/sql` readers see the drop.** `changes` is on the `/v1/sql` allow-list,
  so analysts see far fewer rows per refold. ADR-0015 D1 does not promise row
  counts, but the change of meaning gets a CHANGELOG.md entry.
- **Docs that change with this ADR:**
  - the version-null wording in docs.rs and openapi.json, and the docs.rs event
    section;
  - docs/architecture.md (the "never rewrites history" line and the cursor
    section);
  - docs/operations.md: the refold cost notes and the counts-line wording;
  - the `PROJECTION_EPOCH` doc, whose "noise is permanent" warning is retired.
- **Tests.** The tests that assert the issue-99 replay are re-specified:
  `an_epoch_forced_rewrite_reproduces_identical_content`,
  `a_scoped_stale_stamp_rewrites_only_the_profiles_tenders` and
  `snapshot_content`. A new equivalence test pins that compare equals full
  rewrite, in content and in relative rowid order, by running `off` against `on`.
- **Worst case.** A logic change that moves nearly every row costs today's time
  plus the compare, about 5–20 % more.

## Alternatives rejected

- **Keep the full rewrite and the replay.** It costs 6.6 h of folding and 70.6 M
  permanent change rows per all-profile refold, to re-announce content that is
  more than 95 % unchanged.
- **A stored digest per (version, table)** (issue 495 option (a)). It would fold
  about 1–1.5 h faster than D1, because it never reads the stored rows. It is
  rejected for these reasons:
  - **Soundness depends on an invariant:** a stored digest must always equal the
    digest of the stored rows. Every writer of the 14 tables outside the fold would
    have to keep it, in its own transaction:
    - `repoint_org_references` and its merge callers;
    - `dissolve_condemned`;
    - `repair_version_instants`;
    - the re-parse's party deletes;
    - `backfill-original-lang`;
    - `rederive-eur`;
    - retire, reset, clear and rekey;
    - every future writer.
    A missed one is exactly the silent staleness this ADR exists to avoid.
  - **It needs extra machinery:** a 1–1.2 GB side table, a seeding job, an audit
    job and a rollback guard.
  - **The scoped cohorts gain nothing.** It cannot trust the epoch-0 cohorts
    (scoped refolds and re-parses), so those keep today's full cost.
  - **As proposed, it serves the wrong order.** The digest is an order-blind sum,
    and splitting each table into tender-scope and lot-scope parts would serve
    lot parties before tender parties, an order no full rewrite produces.

  Revisit it only if issue 495's measurement shows the compare reads dominating,
  and the fenced reader-thread pre-compare cannot take them off the writer.
- **A digest per version.** A 490-type change touches `tender_version_lots` on
  most versions, so most versions would still be rewritten whole.
- **`rebuild = true`.** Issue 179 rejected it permanently, and nothing here changes
  that:
  - It reissues tender ids, and organization ids too
    (`strip_organization_indexes`), so every mirror resyncs through a generation
    bump.
  - It serves an empty or partial tender layer for hours (ADR-0009).
  - `tender_version_classifications` (about 278 M rows) is over the 240 M
    auto-index cap. Its index would be rebuilt at the next boot with the API down
    (the issue-205 pattern).
- **Defragmenting the filesystem** (issue 488). It is not the cause:
  - There are 1,947,070 extents now, against 1,946,893 right after the 10-06
    defrag.
  - `xfs_iext_lookup_extent` is 0.12 % of samples.
  - The defrag made the same fold at most 4.6 % faster, and that 4.6 % also
    includes removing the reflink snapshot.
  - A filesystem defrag cannot reorder B-tree pages inside the DB file. VACUUM
    could, but it runs out of memory at this size.
- **Seq-carrying corrections** (re-emitting `changed` at each corrected historical
  seq). SSE classifies such a row at that seq and its predecessor, and
  `include_data` would serve a historical version. Consumers would read old state
  as the correction.
- **A new op or entity kind** (`corrected`). It breaks the published `ChangeEvent`
  enums for every client (the 211/285 precedent). A seq-less `changed` already
  means this.
- **Quiet corrections.** This is the 484 silent miss: a fix that changes what a
  filter returns would never reach a subscriber.
- **A generation bump or resync marker per refold.** It forces every mirror into a
  full resync for a change that touches a fraction of the corpus. That is the
  reason issue 179 rejected rebuild.
- **An interim "every stale prefix counts as changed" emission** before the compare
  lands. It would cut 70.6 M rows to about 22 M, but it would change the contract
  for consumers twice within one issue.
- **A parallel pre-compare on reader threads.** Deferred, not rejected.
  - The writer is released between buckets. So an "identical" verdict from a
    reader is safe only behind a fence: accept it only if no non-fold writer took
    the writer since the reader's snapshot.
  - Build it only if the measurement shows reads dominating.