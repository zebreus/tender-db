# 580 unit 1: remedy design and costing (code-side)

Written 2026-10-24. Code only: no git, cargo, box or HTTP. Line numbers are against the working tree of that date.
Sizing comes from `per-archive-counts.json` in this directory, built from the archived feed windows by another agent
(I read it; I did not rebuild it).

## TL;DR

- **The retire paths don't know the successor.** `retire_chunk_tx` receives only ids. The data that identifies the
  successor is still on disk at retire time: the retired Tender's `tender_versions` rows and `plan_notice`.
  - On the full path the successor Tender already exists.
  - On the incremental path it may not exist yet: retirement runs before Phase 2 mints it.
  - Splits (several successors) are real and tested. So are chains (a successor retired later), and so is a merge
    followed by an undo split.
- **Recommendation: (b′).** This is option (b) with a ledger anchored on notices, plus (a)'s docs written to that
  contract.
  - `retired_tenders(retired_id, notice_id, retired_at)` is written in `retire_chunk_tx`'s own transaction, before
    the rows go. It needs no knowledge of the successor.
  - `GET /v1/tenders/{id}` resolves successors at request time: wherever the retired Tender's notices are now. One
    successor answers `308`; several, or none, answer `410` with `successors`. An id retired before the ledger
    existed answers `410` with `successors: null`, read from the change log.
  - This handles splits, chains and merge-then-undo with no hop limit. A ledger keyed on successor ids would be
    wrong in the undo case and has no id to write on the incremental path.
  - Size **M**. It touches canonical.rs, docs.rs, openapi.json and CHANGELOG.md, all 551 flip files, so the
    switch-off twin must be rebuilt and re-gated (C7). It touches crates/store, so B4's W-on probe applies. It moves
    no W input.
- **Fallback if no store deploy is wanted before 551 U9 closes (11-04):** ship (a) alone now as unit 2a. Its text is
  below and is true today. Ship (b′) as 2b later; it only adds the 308/410 sentences.
- **(c), `tender_id` on `/v1/notices/{id}`: size S, but it solves less.** It needs the client to have saved notice
  ids, and the `publication_id` path already gives them the same answer. It is worth doing on its own merits (the
  notice→Tender direction of ADR-0001), not as the 580 remedy.

## Sizing: how often an id is retired (from `per-archive-counts.json`)

| Window | Retired | Context |
|---|---|---|
| 5 daily chains, 10-19 → 10-23 (jobs 2645, 2669, 2688, 2716, 2726) | 11, 14, 12, 13, 11 (61 in all, mean 12.2 a day) | ~230 per 100k Tenders written; ~0.56% of the ids minted |
| 539 run c (project 2741) | 93 | Out of 147,825 Tenders written; 93 minted |
| Campaign folds that leave grouping alone (2.33M Tenders) | 0 | — |
| Issue 93's 2026-08-02 refold | 216,450 | The worst case recorded |

- A reappearance check over 154 retired ids found none with any later change row and none retired twice. This
  matches AUTOINCREMENT: an id is never issued again.
- So a retirement is a routine daily event (about a dozen a day), not only a campaign one. Ledger growth is a few
  dozen rows a day. A 216k-Tender campaign adds roughly 10^5–10^6 rows (one per version of each retired Tender).
  That is negligible.

## Q1. Does the code know the successor(s) when it retires an id?

**Not at the moment it retires.**

- `retire_chunk_tx(&self, conn, ids, now)` (crates/store/src/canonical.rs:43320-43357) takes ids only.
  1. It emits `tender removed` with version null (43322), then a `removed` row for each lot, lot result, bid and
     contract (43323-43341).
  2. It deletes the CONTENT_TABLES rows (43343-43350), then the `tenders` rows (43352-43356).
- All four callers decide a Tender is orphaned because its key is absent from the plan, never by asking where its
  notices went:
  - `retire_absorbed_legacy_tenders` (43244-43269): `ojs:%` keys missing from `produced`.
  - `retire_regrouped_tenders_chunked` (43555-43589): `SELECT 1 FROM plan_notice WHERE group_key = ?` misses
    (43567-43582).
  - `retire_regrouped_nonlegacy_tenders` (43609-43647): two `NOT EXISTS` anti-joins on `plan_notice` (43612-43640).
  - `twin_repair_tx` (43900-43975): `plan.tenders_emptied` → `retire_chunk_tx` (43971-43973).

**What is on disk at retire time, and when the successor id becomes knowable:**

- **The data is there.**
  - The retired Tender's notices are its `tender_versions` rows. They are deleted only inside `retire_chunk_tx`.
  - Each notice's new group key is in `plan_notice`: `notice_id INTEGER PRIMARY KEY`, `group_key`
    (canonical.rs:18489-18499). It exists on both paths: the incremental plan is built before retirement
    (crates/ingest/src/project.rs:3707-3797), and the full path retires before `clear_plan` (project.rs:2223-2234).
  - A group key is the Tender's `procedure_key`, or `island:<notice>` for an island. The comparison at
    canonical.rs:43572-43578 relies on exactly that.
- **On the full non-rebuild path the successor already exists.**
  - Retirement runs after Phase 2 (project.rs:2168-2215, then 2225 and 2232), so the successor Tender exists.
  - Its id can be read in the same transaction:
    `SELECT DISTINCT tender_id FROM tender_versions WHERE caused_by_notice_id IN (<old's notices>) AND tender_id <> ?`,
    via `tender_versions_notice` (canonical.rs:231).
  - During that window a notice is under two Tenders. The schema allows it: `UNIQUE (tender_id,
    caused_by_notice_id)`, canonical.rs:225-226.
- **On the incremental (daily) path the successor may not exist yet.**
  - Retirement runs **before** Phase 2 (project.rs:3794-3797: "BEFORE applying, so a regrouped notice's old Tender
    is gone").
  - The successor's key is known. Its id is known only if a Tender with that key already exists (the
    `tender_identity` probe, canonical.rs:24639-24660); that is a `tender changed`.
  - A new Tender is minted later in Phase 2 (`INSERT INTO tenders`, canonical.rs:44905). Its id is knowable only
    after Phase 2. If Phase 2 stops (`report.stopped`), it is knowable only after a later run.
- **Cost of reading it after apply:** per retired Tender, one primary-key prefix read on `tender_versions(tender_id,
  seq)` plus one seek on `tender_versions_notice` per version. That is cheap: k is the Tender's version count,
  usually 1–10.
- **Twin repair is the exception.** The retired Tender's versions and notices are deleted before `retire_chunk_tx`
  runs (43908-43930), so nothing is left to anchor on. Its real successor is the Tender of the twin's surviving
  notice (`plan.plan[].keep`), and nothing records that today.

**One id can have several successors (a split).**

- A UUID hub or placeholder key whose buyers disagree is refused. Its Tender is retired and "minted anew per buyer":
  crates/ingest/tests/project_incremental.rs:3755-3758, test `a_uuid_hub_splits_per_buyer_cluster_on_full_and_daily_folds`
  at 3771.
- Group keys `refused:K:<label>` (issues 369, 386, 482) and the weld split from issue 481's buyer guard work the same
  way.
- Many-to-one also happens: in a match, a DÖE island is retired into the TED Tender. See
  `a_matched_row_merges_on_the_next_daily_and_its_undo_splits`, project_incremental.rs:2403, which asserts "one
  removed event" at 2434.

**Successors can be retired later (chains).**

- Nothing protects a freshly minted successor. Issue 386:605-612 records FTS welds that "self-heal as fresh releases
  touch them".
- Ids are `INTEGER PRIMARY KEY AUTOINCREMENT` (canonical.rs:110), so they are never issued twice within a
  generation.
- A rebuild resets the sequence: `reset_tender_layer` drops `tenders` and deletes the `sqlite_sequence` rows
  (canonical.rs:18139-18194); `clear_canonical` does the same through `drop_and_recreate` (16091-16099,
  18100-18122). Any ledger must be cleared at the same points.

**Partial splits retire nothing.**

- A Tender that keeps its key but loses notices is not retired: only a key missing from the plan retires.
- The undo in the same test: "a split retires nothing" (project_incremental.rs:2457; also 2579 and 2707). The DÖE notice becomes a
  fresh island, and the TED Tender keeps its id. Such an id still answers 200, so this is outside 580's scope.

## Q2. Option (a), docs only

**What a client can do today (no doc says so):**

1. Keep each version's `publication_id` (detail `versions[].publication_id`, json.rs:463) or `caused_by_notice_id`
   (json.rs:470). A list row carries the current version's `publication_id` (openapi.json:968).
2. Ask `/v1/tenders?publication_id=<n>&source=<s>`. This resolves "the tender whose versions that number caused —
   current or superseded", via `tender_versions_publication` (read.rs:2050-2097; docs.rs:283; openapi.json:800-803).
   After a retirement it answers the Tender that holds the notice now.
3. Or, with a token, run `SELECT tender_id FROM tender_versions WHERE caused_by_notice_id = ?` on /v1/sql.
   `tender_versions` is allow-listed (sql.rs:157) and served by `tender_versions_notice`. `v_tender_notices` works
   too (sql.rs:1014).

**Two corrections to the issue text:**

- `/v1/notices?tender=<retired>` answers **404** "no such tender", not an empty page: `tender_notices` returns
  `not_found("tender")` when it finds no version (mod.rs:2117-2128).
- The issue's draft has the feed emit `tender removed` **then** the absorbing Tender's `added` or `changed`. That
  order holds on the incremental path only (retirement at project.rs:3796, before Phase 2). On the full path
  `added`/`changed` come first (Phase 2 at 2168-2215, retirement at 2225 and 2232). The docs must say "in either
  order".

**Surfaces and sentences.** docs.rs and openapi.json are 551 flip files, so the twin must be rebuilt and re-gated
(C7); CHANGELOG.md is one too. Two text variants follow: [a] is true today, [b′] replaces it if (b′) ships.

1. **docs.rs:128**, endpoint table, `/v1/tenders/{id}`. Append:
   - [a] "A Tender id can be retired within a generation — see <a href="#retired">retired Tender ids</a>."
   - [b′] "A Tender id a fold retired answers `308` to its successor, or `410` naming its successors."
2. **docs.rs:293-305**, Tender detail. Keep "A missing id is `404`." and add `<p id="retired">` after it. Keep
   "Tender id can be retired" on one source line, because the Verify grep is line-based on the served HTML.

   > **A Tender id can be retired within a generation.** A Tender is the group its notices fold into, and a fold can
   > regroup them: a link between two procedures found or refused, a corrected procedure key, a re-dated chain
   > re-linking, a key found to join unrelated procedures. The Tender that held them is then retired. Its notices fold
   > into another Tender, new or existing, or into several. This happens on the daily fold as well as in refolds. The
   > `generation` does not move, and the id is never issued again. The feed carries the retired Tender's `removed`
   > (`version: null`, with a `removed` for each of its lots) and the receiving Tenders' `added` or `changed`, in
   > either order.
   >
   > [a] The retired id then answers `404`, and so does `/v1/notices?tender=` on it.
   >
   > [b′] `GET /v1/tenders/{id}` on a retired id answers `308` with `Location` and `"successors": [<id>]` when its
   > notices now sit in one Tender. It answers `410` with `"successors": [<id>, …]` when they sit in several, or `[]`
   > while none is folded (as can happen briefly during a fold). Successors are where the notices are when you ask, so
   > an id retired twice answers its latest successor directly. An id retired before <deploy date> answers `410` with
   > `"successors": null`; follow it by its notices.
   >
   > To follow a retired id by its notices, keep each version's `publication_id` (a list row carries the current one)
   > and ask `/v1/tenders?publication_id=…&source=…`. That answers the Tender holding the notice now. On /v1/sql:
   > `SELECT tender_id FROM tender_versions WHERE caused_by_notice_id = ?`.
3. **docs.rs:457-460**, notice-content id note: "entity ids are scoped to the feed's `generation` and are reissued by
   a rebuild". Append ", and a Tender id can also be retired within one (see retired Tender ids)".
4. **docs.rs:500-507**, generation paragraph. After "entity ids are reissued and your cursor indexes a feed that no
   longer exists", insert:

   > Within a generation no id is issued twice, but a Tender id can be retired when a fold regroups its notices. The
   > generation does not move for that; the Tender's `removed` event is the signal (see retired Tender ids).
5. **docs.rs:569-575**, the `version: null` list. After "every `organization` event;" insert:

   > a `tender` a fold retired, removed together with its lots (see retired Tender ids);

   The existing item "an entity `removed` because it left its Tender" covers a lot, not the Tender itself.
6. **openapi.json:65**, `generation`. Append:

   > Within a generation no id is issued twice, but a Tender id can be retired when a fold regroups its notices,
   > without the generation moving (a `tender` `removed` event; see GET /v1/tenders/{id}).
7. **openapi.json:130-150**, `getTender`.
   - Append the retirement and follow-path sentences to `description` (line 134).
   - Replace the shared `"404": NotFound` (line 146; the shared response at 882-885 says only "No such entity.")
     with an inline 404:
     - [a] "No such Tender: never issued in this generation, or retired by a fold (see the description)."
     - [b′] "No such Tender: never issued in this generation."
   - [b′] Also add `308` and `410`, modelled on getOrganization's 308 (239-250):
     - `Location` header;
     - body `allOf[Error, {successors: [int64]}]`;
     - for 410, `successors` is nullable.
8. **openapi.json:1155**, `ChangeEvent.version`. After "an organization event,", insert "a tender a fold retired (its
   lots are removed with it),".
9. **CHANGELOG.md**: an `Unreleased (issue 580)` entry. A docs-only change would normally skip it ("Additive fields
   … land without an entry"); [b′] changes how an existing request answers (404 → 308/410), so it needs one.
10. **Test** in docs.rs `mod tests`: `the_docs_say_a_tender_id_can_be_retired_and_how_to_follow_it`, using
    `assert_says` (docs.rs:1540).
    - On PAGE: the `#retired` paragraph, the generation paragraph and the `version: null` item, with one distinctive
      phrase each.
    - On SPEC: `generation`, `getTender.description` (plus `responses.308`/`410` under [b′]) and
      `ChangeEvent.version`.
    - Also assert that a raw PAGE line contains both "Tender id" and "retired", so the issue's grep Verify cannot go
      to 0 through a rewrap.

Cost of (a): **S**. No store or ingest change, so B4's W-on probe does not apply. It can ride any gated deploy with
a twin re-gate.

## Q3. Option (b), a successor pointer

### Two ledger shapes

| | (b1) id ledger `retired → successor id` | (b′) notice ledger `retired → notices it held` |
|---|---|---|
| Written in the retirement transaction? | Only on the full path. On the incremental path the successor is not minted yet (project.rs:3794-3797), so it needs a fill after Phase 2, and holes are left when Phase 2 stops | Yes, on every path. It reads the Tender's own `tender_versions` before they are deleted |
| Split | Several rows | Several successors, by construction |
| Chain | Follow hops, as `resolve_org` does with `MERGE_HOPS` = 8 (read.rs:5452-5487) | No hops: successors are wherever the notices are now |
| Merge, then undo split (project_incremental.rs:2403-2470) | Stale: points to the TED Tender after the undo | Correct: points to the new island |
| Twin repair (43971) | No successor known | No versions left, so no rows (falls to the change-log `410`) |

Choose **(b′)**.

### Schema

In SCHEMA, beside `org_merge_log`, canonical.rs ~506:

```sql
-- Issue 580: every notice a retired Tender held at its retirement, so the retired id can
-- answer where those notices are NOW (308 to one Tender, 410 naming several). Written by
-- retire_chunk_tx in its own transaction, before the rows go. No FK (org_merge_log precedent).
-- Not a content table: it outlives the Tender. Cleared with the tender layer, because ids
-- restart at 1 on a rebuild.
CREATE TABLE IF NOT EXISTS retired_tenders (
    retired_id INTEGER NOT NULL,
    notice_id  INTEGER NOT NULL,
    retired_at INTEGER NOT NULL
) STRICT;
CREATE INDEX IF NOT EXISTS retired_tenders_id ON retired_tenders(retired_id);
```

- **Name and columns.** Keep it out of `CONTENT_TABLES` (canonical.rs:1462). Otherwise `RETIRE_TABLES` would delete
  it, and the rebuild teardown would treat it as content. Do not name it `tender_version_*`: the golden digests
  assert exactly 14 tables under that GLOB (crates/ingest/tests/project_golden.rs:394-397,
  crates/store/tests/fold_writer_golden.rs:586-590).
- **Constraints.** No PRIMARY KEY or UNIQUE: issue 62 found turso hangs building UNIQUE indexes at scale. A
  duplicate is impossible anyway: a Tender is retired once, and its notices are unique per Tender
  (canonical.rs:225-226).
- **Migration.** The table and index are created empty at the first open, so the index build is instant. After that,
  `IF NOT EXISTS` makes them no-ops.
- **/v1/sql.** Keep it off the allow-list, as `org_merge_log` is (canonical.rs:9402). sql.rs is a flip file as well.

### Write path: in the same transaction, with no successor needed

In `retire_chunk_tx`, after the change-row loop (43341) and before the DELETE loop (43343), for each `IN_CHUNK`
part:

```sql
INSERT INTO retired_tenders(retired_id, notice_id, retired_at)
SELECT tender_id, caused_by_notice_id, ? FROM tender_versions WHERE tender_id IN (…)
```

- **It covers every path.** All three fold callers and the twin repair go through `retire_chunk_tx`.
- **It is invisible to the feed.** It writes no `changes` row, and the ordering contract in the 43311-43319 comment
  holds. It is chunk-invariant by construction.
- **Cost: one statement per 512 retired Tenders.** Today's cost is about 23 statements per Tender (issue 93). For the
  216,450-Tender refold that adds about 423 INSERT…SELECTs, which is negligible. The INSERT…SELECT form has
  precedent (canonical.rs:19582).
- **Resets.** Add `drop_and_recreate(&conn, "retired_tenders")` to `reset_tender_layer` (18139) and
  `clear_canonical` (16091). `drop_and_recreate` replays the index DDL from `sqlite_master` (18100-18122). It is a
  DROP, not a DELETE, because of issue 63's WAL hazard.
- **Not a W input.** Nothing the head election reads changes.

### Read path and API

`read::resolve_tender(conn, id) -> TenderResolution`, beside `resolve_org` (read.rs:5460):

1. **Live** (`tenders.id = ?`) answers 200. The handler only calls this after `tender_detail` returned `None`
   (mod.rs:2012-2016).
2. **Ledger.**
   `SELECT DISTINCT v.tender_id FROM retired_tenders r JOIN tender_versions v ON v.caused_by_notice_id = r.notice_id WHERE r.retired_id = ? ORDER BY v.tender_id LIMIT 101`
   is a seek on `retired_tenders_id`, then `tender_versions_notice` per row.
   - If any ledger row exists, the result is `Retired { successors }`.
   - A successor is live by construction: a version row exists only for a live Tender, because `retire_chunk_tx`
     deletes them together. So there are no cycles and no hops.
3. **No ledger rows**, but
   `SELECT 1 FROM changes WHERE entity_kind = 'tender' AND entity_id = ? AND op = 'removed' LIMIT 1`
   (index `changes_entity`, canonical.rs:1034) hits, and `id <= (SELECT MAX(id) FROM tenders)`: the result is
   `RetiredBeforeLedger`.
   - `tender removed` rows come only from `retire_chunk_tx` (canonical.rs:43322; the other tender appends are
     `added`/`changed`, at 43124, 43143, 32166, 32831, 33397 and 34940).
   - The change log is never pruned (read.rs:5407).
   - The `MAX(id)` guard matters because `reset_tender_layer` keeps `changes` (18183-18188). It stops an id retired
     in an older generation, and never issued in this one, from answering 410.
   - This also covers twin-repair retirements and every id retired before deploy, such as the Verify id 8811451.
4. **Otherwise** the result is `Unknown` → 404.

The handler, `tender` in mod.rs:2002-2017, follows `organization()` (mod.rs:2067-2089):

| Successors | Status | Body |
|---|---|---|
| Exactly 1 | 308 | `Location: /v1/tenders/{s}`, keeping `?lang=`. Use axum `RawQuery`, or re-serialise `lang`. `{"error":{"message":"tender retired","status":308},"successors":[s]}` |
| 2 or more | 410 | `"successors":[…]` (a split) |
| 0, with ledger rows | 410 | `"successors": []` (the notices are folded nowhere, e.g. Phase 2 stopped mid-fold) |
| Retired before the ledger | 410 | `"successors": null` |

- Use one field name, `successors`, an array, for both statuses. Do not reuse `merged_into`: a Tender retirement is
  not always a merge.
- `/v1/notices?tender=<retired>` stays 404, unchanged. Answering it from the ledger is possible later; it is not
  part of this unit.

### SSE and feed

- **Change nothing in the feed.**
  - The `changes` schema (canonical.rs:1026-1033) and its cursor order are the byte-identity surface. They are
    pinned by project_incremental.rs:196-207 and the golden digests.
  - The three transports must stay one shape: SSE `entity_event` (sse.rs:491-505), the /v1/changes poll and
    webhooks (issue 285's three-feed contract).
- **A client follows a `tender removed` by `GET`ting the removed id.**
- **Why not a `successors` field on the event.** It would be computed at delivery time, not event time. A replay
  would show today's state against an old cursor. It would also cost a seek per removed row on every transport.
  Not recommended.

### Backfill

**None is possible from the store.**

- A retired Tender's `tender_versions` rows are deleted in the retirement itself (43343-43350).
- Its change rows carry only `(entity_id, version_seq)`, never the causing notice (canonical.rs:1026-1033).
- `plan_notice` is cleared after every run.

So every id retired before deploy answers `410` with `successors: null` (step 3 above) and is followed by
publication number. This includes 8811451, 8680277 and 8818210.

The issue's done line ("the first line answers `308`") therefore cannot hold for 8811451. Unit 3 must Verify on an id
retired after the deploy: the first `tender removed` past the deploy cursor, by a bounded keyset read on
`changes_entity_cursor`. 8811451 itself should answer `410`.

### Tests

- **crates/ingest/tests/project_incremental.rs.**
  - Extend `a_matched_row_merges_on_the_next_daily_and_its_undo_splits` (2403):
    - after the match, the island's retired id resolves to the TED Tender;
    - after the undo, it resolves to the new island (the chain through the notice);
    - the full and daily DBs hold the same `(retired_id, notice_id)` set.
  - Extend `a_uuid_hub_splits_per_buyer_cluster_on_full_and_daily_folds` (3771): the retired hub resolves to two or
    more successors.
  - Add a ledger digest to `retirement_is_identical_however_it_is_chunked` (959), so chunk invariance covers the
    ledger.
- **crates/store.**
  - `reset_tender_layer` and `clear_canonical` empty the ledger.
  - A plan gate in lib.rs tests, using the `seeks(...)` helper as at ~8710: the resolver statements seek
    `retired_tenders_id`, `tender_versions_notice` and `changes_entity`.
- **crates/app/tests/api.rs.** Add `a_retired_tender_id_redirects_to_its_successor_or_names_its_split`, beside
  `a_merged_away_org_id_redirects_and_filters_by_its_survivor` (3452) and `a_retirement_reaches_the_stream_as_removed`
  (4928). It checks:
  - a no-redirect client gets 308, `Location` and `successors`;
  - a following client gets the successor's detail;
  - `?lang=` is kept;
  - a split gives 410 with both ids;
  - a seeded `tender removed` with no ledger rows gives 410 with `successors: null`;
  - an id never issued gives 404.
- **docs.rs.** The docs test from Q2 item 10, with the [b′] phrases, plus getTender declaring 308 and 410.

Size **M**: one table, about 25 lines in canonical.rs, about 60 in read.rs, about 40 in mod.rs, the docs, openapi
and CHANGELOG, and 5–6 tests.

## Q4. Option (c), `tender_id` on `/v1/notices/{id}`

- **Read.** `SELECT MAX(tender_id) FROM tender_versions WHERE caused_by_notice_id = ?` is a seek on
  `tender_versions_notice` (canonical.rs:231). The same seek already exists, store-private, as `tender_of`
  (canonical.rs:43979-43987).
  - Put it in read.rs, or in the app to stay out of crates/store and B4.
  - Call it from the `notice` handler (mod.rs:2021-2033), on the by-id path only, like `quarantine`.
  - Serve it from `json::notice_detail` (json.rs:203-216).
- **Uniqueness.** The schema allows several Tenders per notice (`UNIQUE (tender_id, caused_by_notice_id)`,
  canonical.rs:225-226). The invariant is one:
  - `plan_notice.notice_id` is a primary key, so a notice keys to exactly one group (canonical.rs:43651);
  - the ghost census verified on prod "max rows per notice = 1" (43675-43676).
  - Two Tenders appear transiently during a full non-rebuild fold, between Phase 2 and retirement (project.rs:2225
    and 2232). `MAX` picks the newer, minted id, which is the successor. Otherwise serve `tender_ids`.
  - Null for a held notice: it has no Tender (json.rs:197-201).
- **Docs.**
  - docs.rs:133 and 283 ("versions[].caused_by_notice_id links back the other way"; now both ways).
  - openapi `NoticeDetail` (openapi.json:1114) gains `tender_id` (int64, nullable).
- **Tests.** An api.rs test: `tender_id` equals the Tender citing the notice, and is null for a quarantined notice.
- **Size S.** For 580 it helps only a client that saved notice ids. Such a client can already use
  `?publication_id=`. It does nothing for `GET /v1/tenders/{old}`.

## Q5. Recommendation: (b′), with (a)'s docs in the same unit

**Reasons:**

1. **Same harm as issue 455, same answer.** A held id goes silently dead: a bare 404, and a subscriber learns nothing
   from `removed`. The project already fixed this for organizations with 308 (docs.rs:140-150). Tenders lose ids
   more often (about a dozen a day, versus merge campaigns).
2. **The notice anchor removes the one real difficulty.** On the daily path the successor is not known at retire
   time, because retirement runs before Phase 2 (project.rs:3794-3797). The anchor still writes in the retirement's
   own transaction. It needs no fill after apply and no hop limit, and it is right for splits, chains and
   merge-then-undo.
3. **Cheap and safe to write.** One INSERT…SELECT per 512 retired Tenders. No change row, no feed shape change, no
   W input.
4. **(c) does not answer 580.** (a) alone leaves the 404 and puts the work on clients who must have saved data.

**Process:**

- The flip files touched are canonical.rs, docs.rs, openapi.json and CHANGELOG.md, so the switch-off twin must be
  rebuilt and re-gated (C7).
- crates/store is touched, so B4's W-on probe is due after the deploy. It is a P1-class probe, expected to show 0
  moves, because no W input changes.
- Not before the 10-28 batched deploy (b3effc2 is gated). Ideally after 551 U9 closes on 11-04, or in a deploy that
  pays a W-on probe anyway.
- If something must ship sooner, ship (a) alone as unit 2a with the [a] text, and (b′) as 2b.

**Unit 2 build list:**

| # | File | Change |
|---|---|---|
| 1 | crates/store/src/canonical.rs | SCHEMA `retired_tenders` + `retired_tenders_id` (~506); INSERT…SELECT in `retire_chunk_tx` (43320, before 43343) and an updated doc comment; `drop_and_recreate("retired_tenders")` in `reset_tender_layer` (18139) and `clear_canonical` (16091) |
| 2 | crates/store/src/read.rs | `TenderResolution` + `resolve_tender` beside `resolve_org` (5436-5487); statement seams for the plan gate |
| 3 | crates/app/src/v1/mod.rs | `tender` handler (2002-2017): 308 / 410 / 410-null / 404, modelled on `organization` (2067-2089), keeping `?lang=` in `Location` |
| 4 | crates/app/src/v1/docs.rs | The Q2 sentences, [b′] variant (128, 293-305 new `#retired`, 457-460, 500-507, 569-575) and the docs test |
| 5 | crates/app/data/openapi.json | 65, 134, getTender 308/410 + inline 404, 1155 |
| 6 | CHANGELOG.md | Unreleased (issue 580) entry |
| 7 | crates/ingest/tests/project_incremental.rs | The two extensions + chunk-invariance ledger digest |
| 8 | crates/app/tests/api.rs | The API test above |
| 9 | crates/store (lib.rs tests) | Plan gate + reset test |

**Gate and Verify:**

- Gate with `ops/check.sh`.
- Unit 3 Verify:
  - an id retired after the deploy answers 308 or 410 with its successors;
  - 8811451 answers 410 with `successors: null`;
  - `/docs` matches "Tender id … retired" at least once;
  - B4's probe reports 0 W moves.
