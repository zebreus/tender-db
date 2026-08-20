# 103 — a shrinking rewrite orphans `lots` / `lot_results` / `bids` / `contracts` rows

Status: FIXED 2026-08-20 (owner) — the sweep is implemented on the accumulate-the-written-ids
formulation this issue's 2026-08-20 note derived, gated on the full-rewrite path (`keep == 0`), with
both long-specified gates in one red-checked test. NOT YET DEPLOYED (job 289 is mid-fold on the
pre-sweep build); landing plan at the bottom — the strays 289 creates need one targeted epoch-stale
refold of the DE-1.x cohort after the sweep deploys.
Kind: correctness (latent) / data hygiene
Blocked by: —
Relates to: 99 (the forced rewrite that made this worth writing down), 93 (`retire_tender_tx`, which
does delete these tables), 85/98

## The wart

`delete_version` clears the twelve version-keyed satellites **and** the `tender_versions` row, but not
the four **tender-scoped entity** tables:

```rust
// delete_version deletes: tender_version_* (12) + tender_versions
// it does NOT delete:     lots, lot_results, bids, contracts
```

That is deliberate and load-bearing. Those four carry surrogate ids referenced by
`tender_version_lots`, `tender_version_lot_results`, `tender_version_bids` and
`tender_version_contracts`, and the write path re-links them by natural key
(`UNIQUE(tender_id, lot_key)` etc., via the `lot_lookup`/`bid_lookup`/… prepared statements). Leaving
them in place is exactly why a rewrite reuses the same lot/bid/contract ids and is therefore
**byte-identical** rather than merely equivalent — which is what issue 99's
`an_epoch_forced_rewrite_reproduces_identical_content` gate depends on.

The wart is the other direction: if a rewrite produces **fewer** entities than the stored version had —
a lot dropped, a bid removed — the old row stays in `lots`/`bids`/… with nothing referencing it. It is
invisible to the API (every read path joins through the version-keyed satellites) but it is dead weight
in the table and in the `UNIQUE` index, and it means `SELECT COUNT(*) FROM lots` overstates reality.

## Why it is not reachable today

A shrinking rewrite requires the projection to produce fewer entities for the same notice set — i.e. a
projection-logic change that *removes* mappings. Every change so far has been additive: issue 85 added
DE-1.x facts, issue 98 adds only party roles and touches no lot/bid/contract structure. The issue-99
epoch re-fold rewrites the whole DE-1.x cohort, but produces a superset, so nothing is orphaned.

It becomes reachable the first time a mapping is **removed or narrowed** — which the 86/48/88 semantics
batch could plausibly do.

## Fix sketch

Either delete the four entity tables' rows for the Tender before rewriting and let the write path
recreate them (simple, but renumbers surrogate ids and would break byte-identity — so it would have to
be paired with the ids being derived rather than surrogate), or sweep entities not referenced by any
surviving version after a rewrite (`DELETE FROM lots WHERE tender_id = ? AND id NOT IN (SELECT lot_id
FROM tender_version_lots WHERE tender_id = ?)`), which preserves ids and is the cheaper, safer shape.

The second is preferred. It should run only on the rewrite path, and its gate is the same one issue 99
uses: a forced rewrite under unchanged logic must still be byte-identical, so the sweep must be a no-op
when nothing shrank.

## Note

Recorded because it is precisely the kind of latent wart that gets discovered as a mysterious count
discrepancy months later. It costs nothing to know about, and the fix is cheap when it is actually
needed rather than speculative now.

## The invariant currently HOLDS corpus-wide — measured 2026-08-03

Issue 115's cross-Tender decoration bug (`summarise` keyed on `lot_id` alone rather than
`(tender_id, seq, lot_id)`) was reachable **only** if some satellite row carried a `lot_id` belonging to
a different Tender — i.e. only if this issue's orphaned rows actually existed. That made 103 the
mechanism that would decide whether a shipped defect was live or latent, so it was measured directly
against the serving database.

Full walks of all three lot-scoped satellites joined to `lots`:

```sql
SELECT COUNT(*) FROM <satellite> s JOIN lots l ON l.id = s.lot_id
 WHERE s.lot_id IS NOT NULL AND l.tender_id <> s.tender_id;
```

| satellite | rows with a foreign `lot_id` |
|---|---|
| `tender_version_texts` | **0** |
| `tender_version_amounts` | **0** |
| `tender_version_dates` | **0** |

**Not one satellite row anywhere in the corpus references a Lot belonging to another Tender.**

Two consequences worth recording:

1. **This wart is still latent, as the Status line says — now with a corpus-wide measurement behind it
   rather than an argument.** No shrinking rewrite has yet orphaned a row that another Tender's version
   then picked up.
2. **The `a39d53a` fix for 115 is therefore PREVENTIVE, not remedial.** It closed a path that the data
   could not currently walk. That is the right thing to have shipped — the guard belongs in the code
   regardless — but the record should not imply production was serving cross-Tender values. It was not.
   Independently corroborated end-to-end: 1,700 served lots across 11 pages spanning 10–117 distinct
   Tenders each, recomputed against the pre-fix three-column semantics, showed zero divergence on both
   the buggy build and the fixed one.

**This is a measurement of today, not a guarantee.** The moment a shrinking rewrite lands without the
sweep described above, the invariant can break — and 115's decoration is no longer the thing that would
notice, because it now matches on all three columns. So the value of this number is that it bounds the
past, not the future: it says the fix was preventive, and it says nothing about whether this wart stays
harmless.

2026-08-09 (orchestrator): re-checked reachability under the issue-174
epoch-2 refold now running against the whole r208 era — still NOT
reachable. The mapping change is additive (one date fact); lots/bids/
contracts structure is identical, so every rewrite is a superset. The
"first removed/narrowed mapping" tripwire stands.

2026-08-20 (owner): **the tripwire has tripped.** Issue 259 narrows a mapping for the first time —
two nested Organization sections that used to mint two parties now mint one — so a refold of the
r208/r209 era will produce FEWER entities for the same notice set. That is the "removed or narrowed
mapping" this issue named as the condition for reachability. The four tender-scoped entity tables
(`lots`, `lot_results`, `bids`, `contracts`) are not what 259 shrinks — it shrinks `organizations`,
which `delete_version` also does not touch — but the shape is identical and the sweep sketched above
is the same fix. Read 259 before the r208/r209 refold, and decide there whether the orphans are swept
or left; do not let the refold land while this is still "not reachable today".

## 2026-08-20 — the tripwire fired, and it was my own change that pulled it

This issue has said since it was filed that it "becomes reachable the first time a mapping is **removed
or narrowed**", and the 2026-08-09 re-check confirmed every change to date had been additive. Today's
issue-100 fix is the first narrowing one.

The DE-1.x inventory labelled three nested *reference carriers* with *entity* node ids
(`ND-LotTender`, `ND-SettledContract`), so each carrier opened a result-entity section and
`read_results` minted a `RawBid`/`RawContract` for it. Renaming them to the SDK's own reference ids
means the projection now produces, per DE-1.x award notice, **two fewer `bids` rows and one fewer
`contracts` row**. `delete_version` clears `tender_version_bids`/`tender_version_contracts` but not
`bids`/`contracts` — which is the wart described above — so the carriers' rows survive with nothing
referencing them.

Order of magnitude: the DE-1.x cohort is 218,638 notices, of which the award-bearing share carries the
carriers. The refold in flight (job 289) is the rewrite that strands them.

Still invisible to the API — every read path joins through the version-keyed satellites — so this is
dead weight and a lying `COUNT(*)`, not a served defect. The urgency is unchanged; what has changed is
that the population is no longer hypothetical, so the sweep can be **verified against real orphans**
instead of only against a synthetic fixture.

## Two implementation constraints the fix sketch did not know about

The sketch above proposes `DELETE FROM lots WHERE tender_id = ? AND id NOT IN (SELECT lot_id FROM
tender_version_lots WHERE tender_id = ?)`. Both halves of that need amending.

**1. The satellite probes are not indexed for this shape.** `tender_version_bids`' primary key is
`(tender_id, seq, bid_id)` — `seq` sits between the two columns the sweep needs, so `(tender_id,
bid_id)` is not a prefix, and this engine has already been caught not using a composite index for a
DELETE (issue 248). Worse, `tender_version_bid_parties` — which also references `bids(id)` and must
therefore be checked — carries **no primary key and only an `organization_id` index**, so a
`NOT EXISTS` against it is a full-table scan **per swept tender**. On a whole-corpus refold that is
not a cost, it is an outage.

The cheap formulation avoids the satellites entirely: `write_version` already resolves every entity id
it references, so the rewrite can accumulate them and the sweep becomes
`DELETE FROM <entity> WHERE tender_id = ? AND id NOT IN (<the ids just written>)`, which rides each
entity table's existing `tender_id`-leading UNIQUE index against an in-memory list.

**2. It is only sound when the WHOLE chain was rewritten.** `apply_tender_tx` keeps a prefix of
`keep` versions whose satellite rows are untouched by `delete_version`. Their entity references are
therefore NOT in "the ids just written", and sweeping against that set would delete rows those kept
versions still point at. So the sweep must be gated on **`keep == 0`** — the forced/epoch-stale
rewrite path, which is exactly the path that can shrink (and exactly what job 289 is doing to the
128,005 epoch-stale DE-1.x Tenders).

That gate also preserves issue 99's byte-identity property for free: under unchanged logic a forced
rewrite writes the same entity ids, the set covers everything, and the DELETE removes nothing.

The reference map, for whoever writes it — an entity may not be deleted while ANY of these name it:

| entity | referenced by |
|---|---|
| `lots` | `tender_version_lots`, `tender_version_lot_group_members` (×2), `tender_version_texts`, `_dates`, `_amounts`, `_classifications`, `_parties`, `tender_version_lot_results.lot_id`, `tender_version_bids.lot_id` |
| `lot_results` | `tender_version_lot_results`, `tender_version_result_stats`, `tender_version_result_winners` |
| `bids` | `tender_version_bids`, `tender_version_bid_parties` |
| `contracts` | `tender_version_contracts` |

Under the accumulate-the-ids formulation the map is not queried, but it is what the accumulator has to
cover: `lots` in particular is reached from nine places, so a `Referenced` set must be fed by
`lot_identity` AND `result_lot`, not only by the version-lots loop.

**Deliberately not implemented in the same breath as noticing it.** A sweep is a DELETE against
canonical entity tables; getting it wrong destroys rows rather than leaving spare ones, and the
existing damage is invisible dead weight. It waits for its own unit with the two gates this issue has
always specified — a shrinking rewrite leaves no orphan, and a forced rewrite under unchanged logic is
still byte-identical.

## Measure it first, after job 289 drains

The orphan count is now a real number rather than a prediction, and it should be read before the sweep
is written so the sweep has something to be checked against:

```sql
-- orphaned bids: no surviving version references them
SELECT COUNT(*) FROM bids b
 WHERE NOT EXISTS (SELECT 1 FROM tender_version_bids v WHERE v.bid_id = b.id)
   AND NOT EXISTS (SELECT 1 FROM tender_version_bid_parties p WHERE p.bid_id = b.id);

-- and orphaned contracts
SELECT COUNT(*) FROM contracts c
 WHERE NOT EXISTS (SELECT 1 FROM tender_version_contracts v WHERE v.contract_id = c.id);
```

Both are whole-table walks with unindexed probes, so they are **not** `/v1/sql` queries — they need a
quiet box and an offline read, or a `tender_id`-bounded window like the one issue 234 used.

## Implemented (2026-08-20, same day the tripwire fired)

`Db::sweep_orphaned_entities` (crates/store/src/canonical.rs), on the design derived above:

- **Accumulate, don't probe.** `write_version`/`write_round` feed a `WrittenEntities` set as they
  resolve ids — every resolution site covered: the lots loop, the group-membership pairs, and the
  three result-entity + two `result_lot` sites. The sweep then does one indexed
  `SELECT id … WHERE tender_id = ?` per entity table (each table's UNIQUE leads with `tender_id`),
  diffs in memory, and deletes the strays by id. The satellites are never queried, so their
  index-shape problem is moot.
- **Gated on `keep == 0 && !stored.is_empty()`** — only the full-rewrite path has a complete
  reference set, and the two orphan-free paths that would pay four SELECTs for nothing (fresh
  Tenders, rebuild-minted Tenders whose chains are empty by construction) skip it.
- **Every sweep is announced**: one `removed` change row per entity, `append_change` with no seq,
  matching the retirement path — issue 164's discipline, since the version-diff machinery compares
  only the NEW chain's versions and structurally cannot see these removals.
- Surfaced as `Applied::entities_swept`, printed in the fold's done-line and the CLI report, so a
  narrowing deploy's first refold states its sweep count instead of leaving it to be mined.

**Gates** (`a_shrinking_rewrite_sweeps_the_entities_no_version_references`, store/src/lib.rs, driven
through the real `apply_tenders` with the rewrite forced the way production forces it —
`projection_epoch` stamped stale):

1. Shrinking rewrite: 2 bids → 1; exactly one swept, the survivor keeps its original surrogate id,
   and one `changes` row (`bid`, `removed`) exists. Falsified: with the sweep gate disabled it fails
   `left: 0, right: 1`.
2. Unchanged forced rewrite: `entities_swept == 0` and the bid ids are identical before and after —
   issue 99's byte-identity property asserted directly, not inferred.

## Landing plan

Job 289 (the refold that strands the DE-1.x carriers) is running on the pre-sweep build; deploying
over it would restart a multi-hour fold, and the sweep does not need to be present when the orphans
are CREATED — only when the cohort is next rewritten. So:

1. Let 289 drain.
2. Deploy the sweep build.
3. Measure the stray population (the bounded windowed count, off-hours).
4. Stamp the DE-1.x cohort epoch-stale (the reparse path's `mark` machinery, or
   `stamp_projection_stale` over the 128,005 tender ids) and run one incremental projection: every
   stamped Tender takes the `keep == 0` path, the sweep fires, and the done-line's
   `entities swept N` should approximate the step-3 count.
5. Re-run the step-3 count; it must be ~0 for the cohort.

## First live firing — 749,349 entities swept (2026-08-21 00:54)

The refold that carried the sweep's first run (job 289, the issue-100/255/257/259 batch over the
128,005 epoch-stale DE-1.x tenders plus the ordinary delta) finished:

    [project] done: 14289308 notices → 7903634 tenders (676926 islands), 4439050 versions,
                    749349 entities swept, 9787850 change rows in 10305.8s

**749,349 strays deleted, each announced with a `removed` change row.** ~5.9 per rewritten DE-1.x
tender, which fits the mechanism: every award round on the cohort minted two phantom LotTender
carriers plus one phantom SettledContract carrier, and chains carry multiple rounds. The step-3/4/5
landing plan above collapsed into this one run — the strays were swept in the same fold that would
have created them, because the sweep deployed first.

The plan's step 5 (the ~0 residual check) stays worth an offline pass when the box is quiet, but the
live counter plus the byte-identity gate is the substance of this issue done.
