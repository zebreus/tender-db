# 04 — Canonical projection + versioning + change log

Status: resolved
Blocked by: 03

Goal: parsed notices project into the versioned canonical layer (ADR-0001)
and every canonical write appends to the change log — the cursor spine
exists.

Scope:
- `store`: `tenders` + `tender_versions(tender_id, seq, caused_by_notice_id,
  published_at, …)` + version-keyed satellites (lots, texts, amounts,
  classifications, parties); `organizations` + `organization_mentions`;
  `changes(cursor AUTOINCREMENT, entity_kind, entity_id, version_seq, op,
  changed_at)` written in the same transaction. Current-state SQL views.
- `ingest`: deterministic projection: group parsed notices by BT-04 (TED)
  / notice UUID chains (DÖE later) / island rule (single-notice Tenders);
  new notice ⇒ new tender_version; defensive lot resolution per version
  (FA/DPS relabeling); org mentions always, canonical org merge only on
  exact normalised official identifiers (plausibility gates from
  ted-legacy-mapping.md).
- Rebuildability: a `reproject` CLI that rebuilds the canonical layer from
  the notice layer from scratch (appending new versions/cursor rows).
- Tests: fixture chains (CN → corrigendum → CAN) assert version sequences,
  change rows, island tenders, org mention dedup.

Acceptance: the fetched day projects into Tenders with correct version
counts; reproject is idempotent-in-content (same canonical state, new
cursor rows); `cargo test` green.

## Answer

Delivered: the versioned canonical layer (`crates/store/src/canonical.rs`), the
projection stage (`crates/ingest/src/project.rs`) and the change cursor.
Verified on the real TED daily **2026-136**: 3715 parsed notices → **3594
Tenders, 3715 versions, 12 478 Lots, 8221 Organizations, 24 553 change rows** in
29 s; a re-run wrote nothing at all.

### Schema

`tenders` is identity only — a source's procedure key (BT-04) or, when a notice
publishes none, the island notice itself. Both live in one table because UNIQUE
over a NULL column is vacuous in SQLite, so the two identity kinds cannot
collide and neither needs a discriminator column. `tender_versions(tender_id,
seq, caused_by_notice_id, published_at, …)` carries `UNIQUE(tender_id,
caused_by_notice_id)`: one version per Notice is the constraint that makes
re-processing a no-op at the schema level rather than by convention.

Satellites are keyed by (tender_id, seq) with a nullable `lot_id` — NULL means
the value is the Tender's own. That is one satellite set instead of two, and it
keeps the Lot link a real foreign key. `lots(tender_id, lot_key)` is Lot
identity and holds *only* the published id, which is the defensive rule made
structural: two versions share a Lot exactly when they publish the same id, and
there is nowhere to record a guess.

A sixth satellite was added beyond the issue's list: `tender_version_dates`.
The chain fixture settled it — the corrigenda in that real procedure change the
submission deadline and nothing else, so without dates the diff would have been
empty and ADR-0001's own motivating question ("how did the deadline move?")
unanswerable.

`changes(cursor AUTOINCREMENT, entity_kind, entity_id, version_seq, op,
changed_at)` is written in the same transaction as the canonical write, for
three entity kinds (tender, lot, organization). Current state is the `v_tenders`
/ `v_lots` / `v_organizations` views over MAX(seq).

### Projection

One code path serves both the incremental and the rebuild case. Because the
projection is deterministic, a Tender's *sequence of causing notices* is its
state key: the reconcile compares stored and computed sequences, keeps the
longest common prefix, and rewrites only the tail. Appending a new notice
touches one version; a notice that arrives late and belongs mid-chain repairs
the tail by itself; an unchanged notice layer produces no writes and no change
rows. `project --rebuild` is the reproject CLI — it drops the canonical layer's
content and re-derives it, appending to a cursor that is never renumbered.

Change ops are diff-based (ADR-0001 amendment): a version emits `changed` only
when its fact set differs from the previous version's, per entity. On the real
daily, all 121 non-first versions differed — no notice was a no-op — and the
ops split tender:added 3594 / tender:changed 121 / lot:added 12 478 /
lot:changed 139 / organization:added 8221.

### Verification (2026-136, scratch db; production untouched)

| | |
|---|---|
| distinct BT-04 | 3470 = keyed Tenders |
| parsed notices without BT-04 | 124 = island Tenders |
| Tenders | **3594** (3470 + 124, exactly) |
| versions | 3715 = one per parsed notice |
| multi-version Tenders | 45, longest chain **27 versions** |
| Lots | 12 478 |
| Organizations | 8221 — **7005 canonical, 1216 provisional** |
| mentions | 14 388 |
| re-run | 0 versions, 0 change rows, identical counts |
| rebuild | identical canonical state; changes 24 553 → 49 106 (appended) |

The 27-version chain is a Spanish framework agreement publishing 27 award-round
notices in one day — the "award rounds are repeated award notices under one
procedure" case of CONTEXT.md, arriving as a single Tender exactly as intended.

### Three findings

1. **An Organization's identifier is not on its Organization section.** The
   first real-data run resolved **zero** canonical Organizations from 14 388
   mentions: eForms hangs BT-501 off a `CompanyLegalEntity` *child* (14 813
   values that day) while BT-500/BT-514 stay on the Organization. A mention now
   collects from the whole Organization subtree, keyed by the nearest enclosing
   Organization — the same rule lot scoping already used, so both share one
   helper. The fixture-only tests could not have caught this; the real daily
   did, in one run.
2. **The plausibility gate earns its keep on eForms too.** ted-legacy-mapping.md
   measured 16 % junk ids in the *legacy* era; on this eForms daily 1216 of 8221
   profiles are provisional, i.e. ~8 % of mentions carry no usable identifier.
   The gate rejects no-digit values ("Romania"), all-zeros, single-character
   fillers and anything under four characters.
3. **VAT and national ids need different country scoping.** A VAT id carries its
   country in its own prefix and is scoped by it; a national registry number is
   only unique inside its country, so it is scoped by the mention's BT-514 and
   stays separate when that is absent. This is visible in the data — the same
   country appears as `RO` on VAT rows and `ROU` on national ones — and it is
   deliberate: merging Spanish NIF `P4900000C` with a same-digits foreign
   registry number would be the silent corruption the rule exists to prevent.

### Notes for later issues

- `crates/app/src/api.rs` changed by one argument (`list_tenders(200)`), since
  the placeholder `tenders(id, title)` table became the real one and the list
  now reads `v_tenders`. Any database created before this issue must be
  rebuilt — there is no migration for the placeholder table, and by design
  nothing needs one.
- Party scope is the nearest enclosing Lot, else the Tender. Award-side roles
  reached through `LotResult` therefore land at Tender scope for now; issue 13
  models the results layer and will scope them per Lot.
- `Db::scalar` was added as a single-value read for tests and operational spot
  checks. The guarded public SQL endpoint is issue 07 and does not build on it.
